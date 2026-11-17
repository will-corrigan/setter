//! Files: the user's config folder, puzzle files and settings.

use std::fs;
use std::path::Path;

/// Config-file templates written to the user's config folder on first run.
#[allow(dead_code)] // used from Task 7
pub const USER_FILES: &[(&str, &str)] = &[
    (
        "words.txt",
        "# Extra words and phrases, one per line. These are treated as the most popular\n\
         # entries, so they come first in fill suggestions and wordplay results.\n",
    ),
    (
        "banned.txt",
        "# Words to never suggest, one per line (letters only, any case).\n",
    ),
    (
        "abbreviations.toml",
        "# Your abbreviations, merged with the built-in list.\n\
         # An empty list removes a built-in entry.\n\
         [abbreviations]\n\
         # zz = [\"sleep\", \"snore\"]\n",
    ),
    (
        "indicators.toml",
        "# Your indicator words, appended to the built-in lists.\n\
         # [anagram]\n\
         # words = [\"discombobulated\"]\n",
    ),
    (
        "theme.css",
        "/* Your CSS, loaded after the app's own styles. Every colour and font is a\n\
            CSS variable — override them here, e.g.:\n\
         :root {\n\
           --paper: #fbf8f1;\n\
           --ink: #1d1b17;\n\
           --accent: #b4442a;\n\
           --font-serif: \"EB Garamond\", Georgia, serif;\n\
         }\n\
         */\n",
    ),
];

#[allow(dead_code)]
pub(crate) fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}
