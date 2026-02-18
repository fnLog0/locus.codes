//! Grid layout component: split an area into rows and columns.

use locus_constant::theme::dark;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

/// Grid layout: divides an area into uniform rows and columns with optional spacing and borders.
#[derive(Debug, Clone)]
pub struct Grid {
    /// Number of rows.
    pub rows: usize,
    /// Number of columns.
    pub cols: usize,
    /// Horizontal gap between columns (0 = no gap).
    pub h_gap: u16,
    /// Vertical gap between rows (0 = no gap).
    pub v_gap: u16,
    /// If true, draw grid lines (borders) around each cell.
    pub draw_borders: bool,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            rows: 1,
            cols: 1,
            h_gap: 0,
            v_gap: 0,
            draw_borders: false,
        }
    }
}

impl Grid {
    /// New grid with the given number of rows and columns.
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows: rows.max(1),
            cols: cols.max(1),
            ..Default::default()
        }
    }

    /// Set horizontal and vertical gap between cells.
    pub fn with_gap(mut self, h_gap: u16, v_gap: u16) -> Self {
        self.h_gap = h_gap;
        self.v_gap = v_gap;
        self
    }

    /// Set whether to draw borders around each cell.
    pub fn with_borders(mut self, draw: bool) -> Self {
        self.draw_borders = draw;
        self
    }

    /// Split `area` into a grid of cell Rects. Returns row-major `[row][col]`.
    pub fn cell_rects(&self, area: Rect) -> Vec<Vec<Rect>> {
        if area.width == 0 || area.height == 0 || self.rows == 0 || self.cols == 0 {
            return vec![];
        }

        let total_h_gap = self.h_gap * (self.cols.saturating_sub(1)) as u16;
        let total_v_gap = self.v_gap * (self.rows.saturating_sub(1)) as u16;
        let cell_width = area
            .width
            .saturating_sub(total_h_gap)
            .saturating_div(self.cols as u16);
        let cell_height = area
            .height
            .saturating_sub(total_v_gap)
            .saturating_div(self.rows as u16);

        let mut out = Vec::with_capacity(self.rows);
        for row in 0..self.rows {
            let y = area.y
                + (row as u16)
                    .saturating_mul(cell_height)
                    .saturating_add((row as u16).saturating_mul(self.v_gap));
            let mut row_rects = Vec::with_capacity(self.cols);
            for col in 0..self.cols {
                let x = area.x
                    + (col as u16)
                        .saturating_mul(cell_width)
                        .saturating_add((col as u16).saturating_mul(self.h_gap));
                row_rects.push(Rect {
                    x,
                    y,
                    width: cell_width,
                    height: cell_height,
                });
            }
            out.push(row_rects);
        }
        out
    }

    /// Return the Rect for cell at `(row, col)`, or None if out of range.
    pub fn cell(&self, area: Rect, row: usize, col: usize) -> Option<Rect> {
        let rects = self.cell_rects(area);
        rects.get(row).and_then(|r| r.get(col)).copied()
    }

    /// Render the grid onto `frame`: if `draw_borders` is true, draw a border around each cell.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let rects = self.cell_rects(area);
        if !self.draw_borders {
            return;
        }
        let border = Color::Rgb(dark::MUTED_FG.0, dark::MUTED_FG.1, dark::MUTED_FG.2);
        let style = Style::default().fg(border);
        for row in &rects {
            for &cell in row {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(style);
                frame.render_widget(block, cell);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_cell_rects_2x2() {
        let grid = Grid::new(2, 2);
        let area = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        let rects = grid.cell_rects(area);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].len(), 2);
        assert_eq!(rects[1].len(), 2);
        assert_eq!(rects[0][0].width, 10);
        assert_eq!(rects[0][0].height, 5);
    }

    #[test]
    fn grid_cell_index() {
        let grid = Grid::new(2, 3);
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 10,
        };
        let r = grid.cell(area, 1, 2);
        assert!(r.is_some());
        let r = grid.cell(area, 2, 0);
        assert!(r.is_none());
    }
}
