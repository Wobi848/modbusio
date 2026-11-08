//! Excel files from the export tables: one sheet per table, a bold head row, frozen, with filters,
//! numbers as numbers.

use rust_xlsxwriter::{Format, Workbook, XlsxError};

use crate::export::Table;

/// Sheet names: at most 31 characters, none of `[]:*?/\`, each once.
fn sheet_name(title: &str, taken: &[String]) -> String {
    let clean: String = title
        .chars()
        .filter(|character| !"[]:*?/\\".contains(*character))
        .take(28)
        .collect();
    let clean = if clean.trim().is_empty() {
        "Sheet".to_string()
    } else {
        clean
    };
    std::iter::once(clean.clone())
        .chain((2..).map(|number| format!("{clean} {number}")))
        .find(|name| !taken.contains(name))
        .expect("some number is free")
}

/// A field that is a number; leading zeros ("007", a serial number) keep it text.
fn number(field: &str) -> Option<f64> {
    let digits = field.trim_start_matches('-');
    if digits.len() > 1 && digits.starts_with('0') && !digits.starts_with("0.") {
        return None;
    }
    field
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

pub fn write(tables: &[Table]) -> Result<Vec<u8>, XlsxError> {
    let mut workbook = Workbook::new();
    let bold = Format::new().set_bold();
    let mut taken = Vec::new();
    for table in tables {
        let name = sheet_name(&table.title, &taken);
        taken.push(name.clone());
        let sheet = workbook.add_worksheet();
        sheet.set_name(&name)?;
        for (column, head) in table.columns.iter().enumerate() {
            sheet.write_string_with_format(0, column as u16, head, &bold)?;
        }
        for (row, fields) in table.rows.iter().enumerate() {
            for (column, field) in fields.iter().enumerate() {
                let (row, column) = (row as u32 + 1, column as u16);
                match number(field) {
                    Some(number) => sheet.write_number(row, column, number)?,
                    None => sheet.write_string(row, column, field)?,
                };
            }
        }
        sheet.set_freeze_panes(1, 0)?;
        if !table.columns.is_empty() {
            sheet.autofilter(
                0,
                0,
                table.rows.len() as u32,
                table.columns.len() as u16 - 1,
            )?;
        }
        sheet.autofit();
    }
    workbook.save_to_buffer()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_names_are_valid_and_unique() {
        let taken = vec!["Points".to_string()];
        assert_eq!(sheet_name("Points", &taken), "Points 2");
        assert_eq!(sheet_name("a/b:c", &[]), "abc");
        assert!(sheet_name(&"x".repeat(40), &[]).len() <= 31);
    }

    #[test]
    fn a_table_becomes_an_excel_file() {
        let mut table = Table::new("Points", &["Name", "Value"]);
        table.rows.push(vec!["Flow".into(), "63.2".into()]);
        let file = write(&[table]).expect("written");
        assert_eq!(&file[..2], b"PK", "an xlsx file is a zip");
    }
}
