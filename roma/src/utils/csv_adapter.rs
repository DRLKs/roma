use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Reads a CSV file and returns the data as a vector of vectors of strings.
/// Each inner vector represents a row, and each string represents a cell value.
///
/// # Arguments
///
/// * `path` - The path to the CSV file
/// * `delimiter` - The delimiter character (e.g., ',' or ';')
/// * `skip_header` - Whether to skip the first line (header row)
///
/// # Returns
///
/// A Result containing the CSV data or an IO error
pub fn read_csv(
    path: &Path,
    delimiter: char,
    skip_header: bool,
) -> std::io::Result<Vec<Vec<String>>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut data = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        // Skip header if requested
        if skip_header && index == 0 {
            continue;
        }

        let line = line?;
        let row: Vec<String> = line
            .split(delimiter)
            .map(|s| s.trim().to_string())
            .collect();
        data.push(row);
    }

    Ok(data)
}

/// Reads a CSV file as records (header -> value).
///
/// The first row is treated as the header. Empty headers are ignored.
/// If a row has fewer columns than the header, missing values are skipped.
pub fn read_csv_records(
    path: &Path,
    delimiter: char,
) -> std::io::Result<Vec<HashMap<String, String>>> {
    let rows = read_csv(path, delimiter, false)?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let header = &rows[0];
    let mut records = Vec::new();

    for row in rows.iter().skip(1) {
        let mut record = HashMap::new();
        for (idx, key) in header.iter().enumerate() {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }

            if let Some(value) = row.get(idx) {
                record.insert(key.to_string(), value.trim().to_string());
            }
        }
        records.push(record);
    }

    Ok(records)
}
