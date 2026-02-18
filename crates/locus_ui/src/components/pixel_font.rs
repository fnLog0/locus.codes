//! Geist Pixel–style block font for the terminal.
//!
//! Terminal approximation of **Geist Pixel (Square)** used on the landing page
//! (`GeistPixel-Square.woff2`). Same square, geometric look using block characters (█).
//! See: <https://vercel.com/font> and `apps/landing/src/css/base.css`.

const SPACE: char = ' ';
const ROWS: usize = 6;

/// Geist Pixel–style glyphs: 6 rows, lowercase proportions.
/// Letters sit in middle/baseline (no full-height caps). Dot on bottom row.
fn glyph(ch: char) -> Option<[&'static str; 6]> {
    match ch {
        // lowercase l: single vertical stroke, no serif
        'l' => Some([" █  ", " █  ", " █  ", " █  ", " █  ", " █  "]),
        // lowercase o: small round, not full height
        'o' => Some(["    ", " ██ ", "█  █", "█  █", " ██ ", "    "]),
        // lowercase c
        'c' => Some(["    ", " ██ ", "█   ", "█   ", " ██ ", "    "]),
        // lowercase u: bowl on baseline
        'u' => Some(["    ", "█  █", "█  █", "█  █", "█  █", " ██ "]),
        // lowercase s
        's' => Some(["    ", " ██ ", "█   ", " ██ ", "  █ ", " ██ "]),
        '.' => Some(["    ", "    ", "    ", "    ", "    ", " █  "]),
        'd' => Some(["    ", "██  ", "█ █ ", "█ █ ", "█ █ ", " ██ "]),
        'e' => Some(["    ", "███ ", "█   ", "██  ", "█   ", "███ "]),
        _ => None,
    }
}

/// Returns pixel-art lines (Geist Pixel style) for the given text.
/// Supports only characters in "locus.": l, o, c, u, s, . (and d, e for "codes").
pub fn pixel_lines(text: &str) -> Vec<String> {
    let mut rows: Vec<String> = (0..ROWS).map(|_| String::new()).collect();
    let mut has_content = false;
    for ch in text.chars() {
        if let Some(g) = glyph(ch) {
            if has_content {
                for row in &mut rows {
                    row.push(SPACE);
                }
            }
            for (r, row) in rows.iter_mut().enumerate() {
                row.push_str(g[r]);
            }
            has_content = true;
        }
    }
    rows.into_iter().filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_lines_locus_dot() {
        let lines = pixel_lines("locus.");
        assert_eq!(lines.len(), ROWS);
    }
}
