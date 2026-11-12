//! JSON for /status.json and the structured data, written by hand: the
//! shapes are small and fixed, and it keeps serde out of the dependencies.

use std::fmt::Write as _;

/// JSON string literal. `<`, `>` and `&` are escaped too, so the result is
/// safe inside a `<script>` element (no `</script>` can appear).
pub(crate) fn string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '<' | '>' | '&' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// JSON array of strings.
pub(crate) fn array<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    let items: Vec<String> = items.into_iter().map(string).collect();
    format!("[{}]", items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped_and_safe_inside_script() {
        assert_eq!(string("</script>\"&"), r#""\u003c/script\u003e\"\u0026""#);
        assert_eq!(string("a\\b\nc\u{1}"), r#""a\\b\nc\u0001""#);
        assert_eq!(array(["a", "b"]), r#"["a","b"]"#);
    }
}
