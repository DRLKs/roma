use crate::solution::Solution;

/// Problem/domain-provided codec used to serialize and restore solutions.
///
/// The core `Solution<T, Q>` remains lightweight and encoding stays opt-in.
pub trait SolutionCodec<T, Q = f64>: Send + Sync
where
    T: Clone,
    Q: Clone + Default,
{
    /// Stable identifier for codec versioning and compatibility checks.
    fn codec_id(&self) -> &'static str;

    /// Encodes one solution into a compact string payload.
    fn encode_solution(&self, solution: &Solution<T, Q>) -> Result<String, String>;

    /// Decodes one solution from a previously encoded payload.
    fn decode_solution(&self, payload: &str) -> Result<Solution<T, Q>, String>;
}
