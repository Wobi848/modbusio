//! Copied unchanged from bacnetio (concept section 14, answer 6).
//!
//! Plain text tables for the terminal.

/// Columns padded to their widest cell, two spaces apart, indented by two.
pub fn print(header: &[&str], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = header.iter().map(|title| title.chars().count()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let header: Vec<String> = header.iter().map(|title| title.to_string()).collect();
    for row in std::iter::once(&header).chain(rows) {
        let cells: Vec<String> = row
            .iter()
            .zip(&widths)
            .map(|(cell, &width)| format!("{cell:<width$}"))
            .collect();
        println!("  {}", cells.join("  ").trim_end());
    }
}
