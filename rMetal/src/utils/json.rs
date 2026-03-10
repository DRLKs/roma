/// Escapes a string so it can be safely embedded inside JSON string values.
pub(crate) trait JsonNodeSerializable {
    fn to_json_node(&self, indent_level: usize) -> String;
}

/// Escapes a string so it can be safely embedded inside JSON string values.
pub fn escape_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Escapes and quotes a raw string value as JSON string literal.
pub fn json_string(value: &str) -> String {
    format!("\"{}\"", escape_string(value))
}

/// Serializes finite floating values, returning `null` for NaN/Infinity.
pub fn f64_to_json(value: f64) -> String {
    if value.is_finite() {
        value.to_string()
    } else {
        "null".to_string()
    }
}

/// Serializes an unsigned integer value as JSON number.
pub fn usize_to_json(value: usize) -> String {
    value.to_string()
}

/// Serializes an unsigned 64-bit value as JSON number.
pub fn u64_to_json(value: u64) -> String {
    value.to_string()
}

/// Builds one JSON field pair, for example: `"name": "value"`.
pub fn json_field(key: &str, value: String) -> String {
    format!("{}: {}", json_string(key), value)
}

fn indent(level: usize) -> String {
    " ".repeat(level)
}

/// Pretty-prints a JSON object from a list of serialized fields.
pub fn json_object(fields: &[String], indent_level: usize) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }

    let base = indent(indent_level);
    let inner = indent(indent_level + 2);
    let body = fields
        .iter()
        .map(|f| format!("{}{}", inner, f))
        .collect::<Vec<_>>()
        .join(",\n");

    format!("{{\n{}\n{}}}", body, base)
}

/// Pretty-prints a JSON array from already serialized JSON nodes.
pub fn json_array(items: &[String], indent_level: usize) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }

    let base = indent(indent_level);
    let inner = indent(indent_level + 2);
    let body = items
        .iter()
        .map(|item| {
            let mut lines = item.lines();
            let first = lines.next().unwrap_or_default();
            let mut out = format!("{}{}", inner, first);
            for line in lines {
                out.push('\n');
                out.push_str(&inner);
                out.push_str(line);
            }
            out
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!("[\n{}\n{}]", body, base)
}