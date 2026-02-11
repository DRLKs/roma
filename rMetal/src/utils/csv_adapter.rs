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
