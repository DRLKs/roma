use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

#[derive(Debug, Clone)]
enum YamlValue {
    Object(BTreeMap<String, YamlValue>),
    Array(Vec<YamlValue>),
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

#[derive(Debug, Clone)]
struct YamlLine {
    indent: usize,
    content: String,
}

struct YamlParser {
    lines: Vec<YamlLine>,
    index: usize,
}

impl YamlParser {
    fn from_text(text: &str) -> Self {
        let mut lines = Vec::new();
        for raw_line in text.lines() {
            let line = strip_yaml_comment(raw_line).trim_end().to_string();
            if line.trim().is_empty() {
                continue;
            }

            let indent = line.chars().take_while(|c| *c == ' ').count();
            let content = line[indent..].to_string();

            lines.push(YamlLine { indent, content });
        }

        Self { lines, index: 0 }
    }

    fn parse(mut self) -> Result<YamlValue, String> {
        if self.lines.is_empty() {
            return Ok(YamlValue::Null);
        }

        let first_indent = self.lines[0].indent;
        self.parse_block(first_indent)
    }

    fn parse_block(&mut self, indent: usize) -> Result<YamlValue, String> {
        let Some(line) = self.current_line() else {
            return Ok(YamlValue::Null);
        };

        if line.indent < indent {
            return Ok(YamlValue::Null);
        }

        if line.indent != indent {
            return Err("Unexpected indentation while parsing YAML block".to_string());
        }

        if line.content.starts_with("- ") {
            self.parse_sequence(indent)
        } else {
            self.parse_mapping(indent)
        }
    }

    fn parse_mapping(&mut self, indent: usize) -> Result<YamlValue, String> {
        let mut map = BTreeMap::new();

        while let Some(line) = self.current_line().cloned() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err("Invalid indentation inside YAML mapping".to_string());
            }
            if line.content.starts_with("- ") {
                break;
            }

            let (key, value_inline) = parse_key_value(&line.content)?;
            self.index += 1;

            let value = if let Some(inline) = value_inline {
                parse_scalar(inline)
            } else {
                let next = self.current_line();
                if let Some(next_line) = next {
                    if next_line.indent > indent {
                        self.parse_block(next_line.indent)?
                    } else {
                        YamlValue::Null
                    }
                } else {
                    YamlValue::Null
                }
            };

            map.insert(key, value);
        }

        Ok(YamlValue::Object(map))
    }

    fn parse_sequence(&mut self, indent: usize) -> Result<YamlValue, String> {
        let mut list = Vec::new();

        while let Some(line) = self.current_line().cloned() {
            if line.indent < indent {
                break;
            }
            if line.indent > indent {
                return Err("Invalid indentation inside YAML sequence".to_string());
            }
            if !line.content.starts_with("- ") {
                break;
            }

            let rest = line.content[2..].trim();
            self.index += 1;

            if rest.is_empty() {
                let next = self.current_line();
                if let Some(next_line) = next {
                    if next_line.indent > indent {
                        list.push(self.parse_block(next_line.indent)?);
                    } else {
                        list.push(YamlValue::Null);
                    }
                } else {
                    list.push(YamlValue::Null);
                }
                continue;
            }

            if is_inline_mapping(rest) {
                let (first_key, first_inline_value) = parse_key_value(rest)?;
                let mut map = BTreeMap::new();
                let first_value = if let Some(inline) = first_inline_value {
                    parse_scalar(inline)
                } else {
                    let next = self.current_line();
                    if let Some(next_line) = next {
                        if next_line.indent > indent {
                            self.parse_block(next_line.indent)?
                        } else {
                            YamlValue::Null
                        }
                    } else {
                        YamlValue::Null
                    }
                };
                map.insert(first_key, first_value);

                if let Some(next_line) = self.current_line() {
                    if next_line.indent > indent && !next_line.content.starts_with("- ") {
                        let extra = self.parse_mapping(next_line.indent)?;
                        if let YamlValue::Object(extra_map) = extra {
                            for (k, v) in extra_map {
                                map.insert(k, v);
                            }
                        }
                    }
                }

                list.push(YamlValue::Object(map));
                continue;
            }

            list.push(parse_scalar(rest));
        }

        Ok(YamlValue::Array(list))
    }

    fn current_line(&self) -> Option<&YamlLine> {
        self.lines.get(self.index)
    }
}

fn strip_yaml_comment(line: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut out = String::new();

    for ch in line.chars() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
                out.push(ch);
            }
            '"' if !in_single => {
                in_double = !in_double;
                out.push(ch);
            }
            '#' if !in_single && !in_double => break,
            _ => out.push(ch),
        }
    }

    out
}

fn parse_key_value(text: &str) -> Result<(String, Option<&str>), String> {
    let Some((raw_key, raw_value)) = text.split_once(':') else {
        return Err(format!("Invalid YAML key/value entry: '{}'", text));
    };

    let key = raw_key.trim();
    if key.is_empty() {
        return Err("YAML key cannot be empty".to_string());
    }

    let value = raw_value.trim();
    if value.is_empty() {
        Ok((key.to_string(), None))
    } else {
        Ok((key.to_string(), Some(value)))
    }
}

fn is_inline_mapping(text: &str) -> bool {
    let Some((key, _)) = text.split_once(':') else {
        return false;
    };
    !key.trim().is_empty()
}

fn parse_scalar(text: &str) -> YamlValue {
    let trimmed = text.trim();

    if trimmed.eq_ignore_ascii_case("null") || trimmed == "~" {
        return YamlValue::Null;
    }
    if trimmed.eq_ignore_ascii_case("true") {
        return YamlValue::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return YamlValue::Bool(false);
    }
    if trimmed.parse::<f64>().is_ok() {
        return YamlValue::Number(trimmed.to_string());
    }

    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        let inner = &trimmed[1..trimmed.len() - 1];
        return YamlValue::String(inner.to_string());
    }

    YamlValue::String(trimmed.to_string())
}

fn parse_path(path: &str) -> Result<Vec<PathToken>, String> {
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

fn resolve_path<'a>(root: &'a YamlValue, path: &str) -> Result<Option<&'a YamlValue>, String> {
    let tokens = parse_path(path)?;
    let mut current = root;

    for token in tokens {
        match token {
            PathToken::Key(key) => {
                let YamlValue::Object(map) = current else {
                    return Ok(None);
                };
                let Some(next) = map.get(&key) else {
                    return Ok(None);
                };
                current = next;
            }
            PathToken::Index(index) => {
                let YamlValue::Array(list) = current else {
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

fn scalar_to_string(value: &YamlValue) -> Option<String> {
    match value {
        YamlValue::String(x) => Some(x.clone()),
        YamlValue::Number(x) => Some(x.clone()),
        YamlValue::Bool(x) => Some(x.to_string()),
        YamlValue::Null => Some("null".to_string()),
        YamlValue::Object(_) | YamlValue::Array(_) => None,
    }
}

fn flatten_to_map(value: &YamlValue, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        YamlValue::Object(map) => {
            for (key, child) in map {
                let next_prefix = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_to_map(child, &next_prefix, out);
            }
        }
        YamlValue::Array(items) => {
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

fn parse_yaml_file(path: &Path) -> std::io::Result<YamlValue> {
    let text = fs::read_to_string(path)?;
    YamlParser::from_text(&text)
        .parse()
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Invalid YAML: {}", e)))
}

/// Reads all scalar values from a YAML file, flattening nested keys with dot notation.
///
/// Arrays are represented with index notation, for example `items[0].weight`.
pub fn read_yaml_flat(path: &Path) -> std::io::Result<HashMap<String, String>> {
    let root = parse_yaml_file(path)?;
    let mut out = HashMap::new();
    flatten_to_map(&root, "", &mut out);
    Ok(out)
}

/// Reads a scalar value from YAML using a path expression like `config.algorithm.population` or `items[0].weight`.
pub fn get_yaml_value(path: &Path, key_path: &str) -> std::io::Result<Option<String>> {
    let root = parse_yaml_file(path)?;
    let value = resolve_path(&root, key_path)
        .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path: {}", e)))?;

    Ok(value.and_then(scalar_to_string))
}

/// Reads multiple scalar values from YAML by path.
pub fn get_yaml_values(path: &Path, key_paths: &[&str]) -> std::io::Result<HashMap<String, String>> {
    let root = parse_yaml_file(path)?;
    let mut out = HashMap::new();

    for key_path in key_paths {
        let value = resolve_path(&root, key_path)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path '{}': {}", key_path, e)))?;

        if let Some(text) = value.and_then(scalar_to_string) {
            out.insert((*key_path).to_string(), text);
        }
    }

    Ok(out)
}

/// Reads a YAML sequence of objects and flattens each object into a map.
///
/// If `records_path` is empty, the root value is expected to be a sequence.
/// Otherwise, the value at the provided path must be a sequence.
pub fn read_yaml_records(path: &Path, records_path: &str) -> std::io::Result<Vec<HashMap<String, String>>> {
    let root = parse_yaml_file(path)?;
    let target = if records_path.trim().is_empty() {
        Some(&root)
    } else {
        resolve_path(&root, records_path)
            .map_err(|e| Error::new(ErrorKind::InvalidInput, format!("Invalid path: {}", e)))?
    };

    let Some(value) = target else {
        return Ok(Vec::new());
    };

    let YamlValue::Array(items) = value else {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "YAML records path must point to a sequence",
        ));
    };

    let mut records = Vec::new();
    for item in items {
        let YamlValue::Object(map) = item else {
            continue;
        };
        let mut record = HashMap::new();
        flatten_to_map(&YamlValue::Object(map.clone()), "", &mut record);
        records.push(record);
    }

    Ok(records)
}
