//! Updates, for Setter installed from a release (Velopack). At start-up, and
//! from "Check for updates…", it looks for a newer release on GitHub. By
//! default it then asks: nothing is downloaded until the setter chooses
//! Update, and then Setter restarts into the new version. With "Install
//! updates automatically" on, it downloads quietly, offers "Restart to
//! update", and installs as Setter closes. Builds that weren't installed from
//! a release (development builds) never update.

use std::sync::Mutex;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, VelopackAsset};

/// Where releases are published.
pub const REPO: &str = "https://github.com/will-corrigan/setter";

/// What a check found.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Found {
    /// A newer version, not downloaded yet.
    Newer(String),
    /// This is the latest.
    Current,
    /// This copy can't update itself (not installed from a release), or
    /// GitHub couldn't be reached.
    Unavailable,
}

/// A newer release that has been found but not downloaded.
static FOUND: Mutex<Option<(UpdateManager, UpdateInfo)>> = Mutex::new(None);

/// A downloaded update, waiting to be installed.
struct Ready {
    manager: UpdateManager,
    asset: VelopackAsset,
}

static READY: Mutex<Option<Ready>> = Mutex::new(None);

fn manager() -> Option<UpdateManager> {
    UpdateManager::new(GithubSource::new(REPO, None, false), None, None).ok()
}

/// Look for a newer release. Blocks; run it off the UI thread.
pub fn check() -> Found {
    let Some(manager) = manager() else {
        return Found::Unavailable;
    };
    match manager.check_for_updates() {
        Ok(UpdateCheck::UpdateAvailable(info)) => {
            let version = info.TargetFullRelease.Version.clone();
            *FOUND.lock().unwrap_or_else(|e| e.into_inner()) = Some((manager, *info));
            Found::Newer(version)
        }
        Ok(_) => Found::Current,
        Err(_) => Found::Unavailable,
    }
}

/// Download the release [`check`] found, ready to install. Blocks; run it off
/// the UI thread. Returns its version.
pub fn download() -> Result<String, String> {
    let found = FOUND.lock().unwrap_or_else(|e| e.into_inner()).take();
    let Some((manager, info)) = found else {
        return Err("There is no update to download.".into());
    };
    manager
        .download_updates(&info, None)
        .map_err(|e| format!("Couldn't download the update: {e}"))?;
    let asset = info.TargetFullRelease.clone();
    let version = asset.Version.clone();
    *READY.lock().unwrap_or_else(|e| e.into_inner()) = Some(Ready { manager, asset });
    Ok(version)
}

/// Install the downloaded update and start the new version. Save first: this
/// ends the process.
pub fn restart_now() -> Result<(), String> {
    let guard = READY.lock().unwrap_or_else(|e| e.into_inner());
    let Some(ready) = guard.as_ref() else {
        return Err("There is no update to install.".into());
    };
    ready
        .manager
        .apply_updates_and_restart(&ready.asset)
        .map_err(|e| format!("Couldn't install the update: {e}"))
}

/// Setter is closing: install a downloaded update once it has gone, without
/// starting it again. (Only ever downloaded if the setter chose to update, or
/// updates install automatically.)
pub fn install_on_exit() {
    let guard = READY.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ready) = guard.as_ref() {
        let _ = ready.manager.wait_exit_then_apply_updates(
            &ready.asset,
            true,
            false,
            Vec::<String>::new(),
        );
    }
}

/// The release's page on GitHub: what's new in it.
pub fn notes_url(version: &str) -> String {
    format!("{REPO}/releases/tag/v{version}")
}

/// The top bar's tooltip for a downloaded update.
pub fn ready_tip(version: &str) -> String {
    format!("Setter {version} is ready")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_not_installed_from_a_release_never_updates() {
        assert_eq!(check(), Found::Unavailable);
        assert!(download().is_err());
        assert!(restart_now().is_err());
        install_on_exit();
    }

    #[test]
    fn the_tip_and_notes_name_the_version() {
        assert_eq!(ready_tip("0.2.0"), "Setter 0.2.0 is ready");
        assert_eq!(
            notes_url("0.2.0"),
            "https://github.com/will-corrigan/setter/releases/tag/v0.2.0"
        );
    }
}
