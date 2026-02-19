//! Geist Pixel–style block font for the terminal.
//!
//! Terminal approximation of **Geist Pixel (Square)** used on the landing page
//! (`GeistPixel-Square.woff2`). Same square, geometric look using block characters (█).
//! See: <https://vercel.com/font> and `apps/landing/src/css/base.css`.

const SPACE: char = ' ';
const ROWS: usize = 7;

/// Geist Pixel–style glyphs: 7 rows, lowercase only.
/// Charset: `l o c u s .` — just enough for "locus."
fn glyph(ch: char) -> Option<[&'static str; 7]> {
    match ch {
        // Tall "l" (as you already want)
        'l' => Some(["██ ", " █  ", " █  ", " █  ", " █  ", " █  ", "████"]),

        // Small letters (half height, base aligned)
        'o' => Some(["    ", "    ", " ██ ", "█  █", "█  █", "█  █", " ██ "]),

        'c' => Some(["    ", "    ", " ██ ", "█  █", "█   ", "█  █", " ██ "]),

        'u' => Some(["    ", "    ", "█  █", "█  █", "█  █", "█  █", " ██ "]),

        's' => Some(["    ", "    ", " ██ ", "█   ", " ██ ", "   █", "███ "]),

        '.' => Some(["    ", "    ", "    ", "    ", "    ", " ██ ", " ██ "]),

        _ => None,
    }
}

/// Returns pixel-art lines (Geist Pixel style) for the given text.
/// Supports only lowercase `l, o, c, u, s, .`
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
