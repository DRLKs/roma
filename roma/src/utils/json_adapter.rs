use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

#[derive(Debug, Clone)]
enum JsonValue {
    Object(BTreeMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Number(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
enum PathToken {
    Key(String),
    Index(usize),
}

struct JsonParser<'a> {
    input: &'a [u8],
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            index: 0,
        }
    }

    fn parse(mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        let value = self.parse_value()?;
        self.skip_whitespace();

        if self.index != self.input.len() {
            return Err("Unexpected trailing characters in JSON".to_string());
        }

        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(JsonValue::String(self.parse_string()?)),
            Some(b't') => self.parse_true(),
            Some(b'f') => self.parse_false(),
            Some(b'n') => self.parse_null(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            _ => Err("Invalid JSON value".to_string()),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.consume_byte(b'{')?;
        self.skip_whitespace();

        let mut map = BTreeMap::new();
        if self.peek_byte() == Some(b'}') {
            self.index += 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.consume_byte(b':')?;
            self.skip_whitespace();
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();

            match self.peek_byte() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("Expected ',' or '}' in JSON object".to_string()),
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.consume_byte(b'[')?;
        self.skip_whitespace();

        let mut values = Vec::new();
        if self.peek_byte() == Some(b']') {
            self.index += 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            self.skip_whitespace();
            values.push(self.parse_value()?);
            self.skip_whitespace();

            match self.peek_byte() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err("Expected ',' or ']' in JSON array".to_string()),
            }
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.consume_byte(b'"')?;
        let mut result = String::new();

        while let Some(ch) = self.next_byte() {
            match ch {
                b'"' => return Ok(result),
                b'\\' => {
                    let escaped = self
                        .next_byte()
                        .ok_or_else(|| "Unexpected end of input in string escape".to_string())?;
                    match escaped {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000C}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let code_point = self.parse_unicode_escape()?;
                            let Some(decoded) = char::from_u32(code_point) else {
                                return Err("Invalid unicode escape sequence".to_string());
                            };
                            result.push(decoded);
                        }
                        _ => return Err("Invalid escape sequence in JSON string".to_string()),
                    }
                }
                _ => result.push(ch as char),
            }
        }

        Err("Unterminated JSON string".to_string())
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, String> {
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self
                .next_byte()
                .ok_or_else(|| "Unexpected end while parsing unicode escape".to_string())?;
            value = (value << 4)
                + match byte {
                    b'0'..=b'9' => (byte - b'0') as u32,
                    b'a'..=b'f' => 10 + (byte - b'a') as u32,
                    b'A'..=b'F' => 10 + (byte - b'A') as u32,
                    _ => return Err("Invalid unicode escape digit".to_string()),
                };
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.index;

        if self.peek_byte() == Some(b'-') {
            self.index += 1;
        }

        self.consume_digits()?;

        if self.peek_byte() == Some(b'.') {
            self.index += 1;
            self.consume_digits()?;
        }

        if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            self.index += 1;
            if matches!(self.peek_byte(), Some(b'+') | Some(b'-')) {
                self.index += 1;
            }
            self.consume_digits()?;
        }

        let text = std::str::from_utf8(&self.input[start..self.index])
            .map_err(|_| "Invalid number encoding".to_string())?
            .to_string();

        Ok(JsonValue::Number(text))
    }

    fn consume_digits(&mut self) -> Result<(), String> {
        let mut consumed = false;
        while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
            consumed = true;
            self.index += 1;
        }

        if !consumed {
            return Err("Expected at least one digit".to_string());
        }

        Ok(())
    }

    fn parse_true(&mut self) -> Result<JsonValue, String> {
        self.consume_literal(b"true")?;
        Ok(JsonValue::Bool(true))
    }

    fn parse_false(&mut self) -> Result<JsonValue, String> {
        self.consume_literal(b"false")?;
        Ok(JsonValue::Bool(false))
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        self.consume_literal(b"null")?;
        Ok(JsonValue::Null)
    }

    fn consume_literal(&mut self, literal: &[u8]) -> Result<(), String> {
        for expected in literal {
            let byte = self
                .next_byte()
                .ok_or_else(|| "Unexpected end while parsing JSON literal".to_string())?;
            if &byte != expected {
                return Err("Invalid JSON literal".to_string());
            }
        }
        Ok(())
    }

    fn consume_byte(&mut self, expected: u8) -> Result<(), String> {
        let byte = self
            .next_byte()
            .ok_or_else(|| format!("Expected byte '{}', found end of input", expected as char))?;
        if byte != expected {
            return Err(format!(
                "Expected byte '{}', found '{}'",
                expected as char, byte as char
            ));
        }

        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let out = self.peek_byte();
        if out.is_some() {
            self.index += 1;
        }
        out
    }
}

fn parse_json_path(path: &str) -> Result<Vec<PathToken>, String> {
    if path.trim().is_empty() {
        return Ok(Vec::new());
    }

    let bytes = path.as_bytes();
    let mut index = 0usize;
    let mut tokens = Vec::new();

    while index < bytes.len() {
        if bytes[index] == b'.' {
            index += 1;
            continue;
        }

        if bytes[index] == b'[' {
            index += 1;
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }

            if start == index {
                return Err("Array index in path cannot be empty".to_string());
            }

            if index >= bytes.len() || bytes[index] != b']' {
                return Err("Missing closing ']' in path".to_string());
            }

            let number_str = std::str::from_utf8(&bytes[start..index])
                .map_err(|_| "Invalid UTF-8 in array index".to_string())?;
            let idx = number_str
                .parse::<usize>()
                .map_err(|_| "Invalid array index in path".to_string())?;

            tokens.push(PathToken::Index(idx));
            index += 1;
            continue;
        }

        let start = index;
        while index < bytes.len() && bytes[index] != b'.' && bytes[index] != b'[' {
            index += 1;
        }

        let key = std::str::from_utf8(&bytes[start..index])
            .map_err(|_| "Invalid UTF-8 in path key".to_string())?
            .trim();

        if key.is_empty() {
            return Err("Path key cannot be empty".to_string());
        }

        tokens.push(PathToken::Key(key.to_string()));
    }

    Ok(tokens)
}

fn resolve_path<'a>(root: &'a JsonValue, path: &str) -> Result<Option<&'a JsonValue>, String> {
    let tokens = parse_json_path(path)?;
    let mut current = root;

    for token in tokens {
        match token {
            PathToken::Key(key) => {
                let JsonValue::Object(map) = current else {
                    return Ok(None);
                };
                let Some(next) = map.get(&key) else {
                    return Ok(None);
                };
                current = next;
            }
            PathToken::Index(index) => {
                let JsonValue::Array(list) = current else {
                    return Ok(None);
                };
                let Some(next) = list.get(index) else {
                    return Ok(None);
                };
                current = next;
            }
        }
    }

    Ok(Some(current))
}

fn scalar_to_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(x) => Some(x.clone()),
        JsonValue::Number(x) => Some(x.clone()),
        JsonValue::Bool(x) => Some(x.to_string()),
        JsonValue::Null => Some("null".to_string()),
        JsonValue::Object(_) | JsonValue::Array(_) => None,
    }
}

fn flatten_to_map(value: &JsonValue, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_to_map(child, &next_prefix, out);
            }
        }
        JsonValue::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                let next_prefix = format!("{}[{}]", prefix, i);
                flatten_to_map(child, &next_prefix, out);
            }
        }
        _ => {
            if !prefix.is_empty() {
                if let Some(text) = scalar_to_string(value) {
                    out.insert(prefix.to_string(), text);
                }
            }
        }
    }
}

fn parse_json_file(path: &Path) -> std::io::Result<JsonValue> {
    let text = fs::read_to_string(path)?;
    JsonParser::new(&text)
        .parse()
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid JSON: {}", e)))
}

/// Reads a scalar value from a JSON string using a path expression.
pub fn get_json_value_from_str(json: &str, key_path: &str) -> std::io::Result<Option<String>> {
    let root = JsonParser::new(json)
        .parse()
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid JSON: {}", e)))?;

    let value = resolve_path(&root, key_path)
        .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path: {}", e)))?;

    Ok(value.and_then(scalar_to_string))
}

/// Reads a scalar value from JSON using a path expression like `config.algorithm.population` or `items[0].weight`.
pub fn get_json_value(path: &Path, key_path: &str) -> std::io::Result<Option<String>> {
    let root = parse_json_file(path)?;
    let value = resolve_path(&root, key_path)
        .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path: {}", e)))?;

    Ok(value.and_then(scalar_to_string))
}

/// Reads an array of JSON objects and flattens each object into a map.
///
/// If `records_path` is empty, the root value is expected to be an array.
/// Otherwise, the value at the provided path must be an array.
pub fn read_json_records(
    path: &Path,
    records_path: &str,
) -> std::io::Result<Vec<HashMap<String, String>>> {
    let root = parse_json_file(path)?;
    let target = if records_path.trim().is_empty() {
        Some(&root)
    } else {
        resolve_path(&root, records_path)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path: {}", e)))?
    };

    let Some(value) = target else {
        return Ok(Vec::new());
    };

    let JsonValue::Array(items) = value else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "JSON records path must point to an array",
        ));
    };

    let mut records = Vec::new();
    for item in items {
        let JsonValue::Object(map) = item else {
            continue;
        };
        let mut record = HashMap::new();
        flatten_to_map(&JsonValue::Object(map.clone()), "", &mut record);
        records.push(record);
    }

    Ok(records)
}
