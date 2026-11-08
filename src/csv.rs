//! CSV as Excel reads and writes it: a separator found from the header (`;` on German systems,
//! `,` or tab elsewhere), fields in quotes where they hold the separator, quotes or line breaks.

/// The separator that splits a line of the first ten into the most fields (the first line may
/// be a title).
pub fn guess_separator(text: &str) -> char {
    let most = |separator: char| {
        text.lines()
            .take(10)
            .map(|line| line.matches(separator).count())
            .max()
            .unwrap_or(0)
    };
    // on a tie the earlier wins: `;` is what Excel writes on the maintainer's systems
    [';', ',', '\t']
        .into_iter()
        .rev()
        .max_by_key(|separator| most(*separator))
        .unwrap_or(';')
}

/// Every row of `text`, fields unquoted.
pub fn parse(text: &str, separator: char) -> Vec<Vec<String>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match (quoted, character) {
            (true, '"') if characters.peek() == Some(&'"') => {
                field.push('"');
                characters.next();
            }
            (true, '"') => quoted = false,
            (true, other) => field.push(other),
            (false, '"') if field.is_empty() => quoted = true,
            (false, '\r') => {}
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (false, other) if other == separator => row.push(std::mem::take(&mut field)),
            (false, other) => field.push(other),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows.retain(|row| row.iter().any(|field| !field.trim().is_empty()));
    rows
}

/// One line with `;`, as Excel on the maintainer's systems expects, ending in CRLF.
pub fn line(fields: &[String]) -> String {
    let mut line = fields
        .iter()
        .map(|field| quote(field))
        .collect::<Vec<_>>()
        .join(";");
    line.push_str("\r\n");
    line
}

fn quote(field: &str) -> String {
    // a leading = + - @ would run as a formula in Excel
    let field = if field.starts_with(['=', '+', '@'])
        || (field.starts_with('-') && field.parse::<f64>().is_err())
    {
        format!("'{field}")
    } else {
        field.to_string()
    };
    if field.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field
    }
}

/// A whole file: a byte order mark so Excel reads UTF-8, then the lines.
pub fn file(rows: &[Vec<String>]) -> String {
    let mut text = String::from('\u{feff}');
    for row in rows {
        text.push_str(&line(row));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_fields_with_separators_and_line_breaks_parse() {
        let text =
            "\u{feff}Name;Address\r\n\"Flow; supply\";40001\n\"Say \"\"hi\"\"\nthere\";2\n\n";
        let rows = parse(text, guess_separator(text));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1], ["Flow; supply", "40001"]);
        assert_eq!(rows[2][0], "Say \"hi\"\nthere");
    }

    #[test]
    fn the_separator_is_found_from_the_header() {
        assert_eq!(guess_separator("a,b,c\n1;2"), ',');
        assert_eq!(guess_separator("A title\na;b;c"), ';');
        assert_eq!(guess_separator("a\tb\tc"), '\t');
        assert_eq!(guess_separator("a;b"), ';');
    }

    #[test]
    fn written_fields_are_quoted_and_safe_from_formulas() {
        assert_eq!(
            line(&["a;b".into(), "=SUM(A1)".into(), "-3.5".into()]),
            "\"a;b\";'=SUM(A1);-3.5\r\n"
        );
    }
}
