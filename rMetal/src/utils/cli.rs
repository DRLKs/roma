/// Reads a reproducibility seed from CLI arguments.
///
/// Supported formats:
/// - `--seed 123`
/// - `--seed=123`
/// - `-s 123`
///
/// If no valid seed is provided, returns `default_seed`.
pub fn seed_from_cli_or(default_seed: u64) -> u64 {
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--seed" || arg == "-s" {
            if let Some(value) = args.next() {
                if let Ok(seed) = value.parse::<u64>() {
                    return seed;
                }
            }
            continue;
        }

        if let Some(value) = arg.strip_prefix("--seed=") {
            if let Ok(seed) = value.parse::<u64>() {
                return seed;
            }
        }
    }

    default_seed
}
