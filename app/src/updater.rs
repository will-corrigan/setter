//! Updates, for Setter installed by its Setup.exe (Velopack). At start-up it
//! looks for a newer release on GitHub and downloads it in the background; the
//! top bar then offers "Restart to update", and if Setter is simply closed the
//! update is installed as it goes. Builds that weren't installed that way
//! (development builds) never update.

use std::sync::Mutex;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateManager, VelopackAsset};

/// Where releases are published.
pub const REPO: &str = "https://github.com/will-corrigan/setter";

/// A downloaded update, waiting to be installed.
struct Ready {
    manager: UpdateManager,
    asset: VelopackAsset,
}

static READY: Mutex<Option<Ready>> = Mutex::new(None);

/// Look for a newer release and download it. Blocks; run it off the UI
/// thread. Returns the version now ready to install, if any. Every failure
/// (offline, not installed by Setup.exe, GitHub unavailable) means "no update":
/// nothing to tell the setter about.
pub fn fetch() -> Option<String> {
    let source = GithubSource::new(REPO, None, false);
    let manager = UpdateManager::new(source, None, None).ok()?;
    let UpdateCheck::UpdateAvailable(info) = manager.check_for_updates().ok()? else {
        return None;
    };
    manager.download_updates(&info, None).ok()?;
    let asset = info.TargetFullRelease.clone();
    let version = asset.Version.clone();
    *READY.lock().unwrap_or_else(|e| e.into_inner()) = Some(Ready { manager, asset });
    Some(version)
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
/// starting it again.
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

/// The top bar's tooltip for a downloaded update.
pub fn ready_tip(version: &str) -> String {
    format!("Setter {version} is ready")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_not_installed_by_setup_never_updates() {
        assert_eq!(fetch(), None);
        assert!(restart_now().is_err());
        install_on_exit();
    }

    #[test]
    fn the_tip_names_the_version() {
        assert_eq!(ready_tip("0.2.0"), "Setter 0.2.0 is ready");
    }
}
