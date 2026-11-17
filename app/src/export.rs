//! Exporting the puzzle: formats, file names, and the bytes to write.

use xw_engine::export::{player, puz};
use xw_engine::puzzle::Puzzle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// A self-contained page anyone can solve in a browser.
    WebPage,
    /// Across Lite, opened by most crossword apps.
    Puz,
    /// The open ipuz standard.
    Ipuz,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 3] =
        [ExportFormat::WebPage, ExportFormat::Puz, ExportFormat::Ipuz];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::WebPage => "Web page (.html)",
            ExportFormat::Puz => "Across Lite (.puz)",
            ExportFormat::Ipuz => "ipuz (.ipuz)",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::WebPage => "html",
            ExportFormat::Puz => "puz",
            ExportFormat::Ipuz => "ipuz",
        }
    }

    pub fn file_name(self, title: &str) -> String {
        format!("{}.{}", file_stem(title), self.extension())
    }
}

/// A file name from the puzzle title, with characters files can't hold replaced.
pub fn file_stem(title: &str) -> String {
    let t = title.trim();
    if t.is_empty() {
        return "crossword".into();
    }
    t.chars()
        .map(|c| {
            if "/\\:*?\"<>|".contains(c) || c.is_control() {
                '-'
            } else {
                c
            }
        })
        .collect()
}

/// The bytes to write for a format.
pub fn render(p: &Puzzle, format: ExportFormat) -> Result<Vec<u8>, String> {
    match format {
        ExportFormat::WebPage => Ok(player::html(p).into_bytes()),
        ExportFormat::Puz => puz::puz(p),
        ExportFormat::Ipuz => serde_json::to_vec_pretty(&p.to_ipuz()).map_err(|e| e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xw_engine::grid::Grid;
    use xw_engine::puzzle::Puzzle;

    fn filled() -> Puzzle {
        let mut p = Puzzle::new(3, 3);
        p.title = "Sunday: prize/puzzle?".into();
        p.grid = Grid::from_rows(&["cat", "a#o", "bye"]);
        p
    }

    #[test]
    fn file_names_come_from_the_title() {
        assert_eq!(file_stem("Sunday: prize/puzzle?"), "Sunday- prize-puzzle-");
        assert_eq!(file_stem("   "), "crossword");
        assert_eq!(ExportFormat::WebPage.file_name("Prize 12"), "Prize 12.html");
        assert_eq!(ExportFormat::Puz.file_name("Prize 12"), "Prize 12.puz");
        assert_eq!(ExportFormat::Ipuz.file_name(""), "crossword.ipuz");
    }

    #[test]
    fn every_format_renders() {
        let p = filled();
        assert!(render(&p, ExportFormat::WebPage)
            .unwrap()
            .starts_with(b"<!DOCTYPE html>"));
        assert_eq!(
            &render(&p, ExportFormat::Puz).unwrap()[2..14],
            b"ACROSS&DOWN\0"
        );
        let ipuz: serde_json::Value =
            serde_json::from_slice(&render(&p, ExportFormat::Ipuz).unwrap()).unwrap();
        assert_eq!(ipuz["kind"][0], "http://ipuz.org/crossword#1");
    }

    #[test]
    fn unfinished_grids_still_export_except_puz() {
        let mut p = filled();
        p.grid.cells[0].letter = None;
        assert!(render(&p, ExportFormat::WebPage).is_ok());
        assert!(render(&p, ExportFormat::Ipuz).is_ok());
        assert!(render(&p, ExportFormat::Puz)
            .unwrap_err()
            .contains("Fill every square"));
    }
}
