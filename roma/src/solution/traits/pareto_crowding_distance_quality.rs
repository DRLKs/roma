/// Multi-objective quality metadata based on Pareto rank and crowding distance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParetoCrowdingDistanceQuality {
    /// Objective values in minimization form.
    pub objectives: Vec<f64>,
    /// Non-dominated sorting rank (lower is better).
    pub rank: Option<usize>,
    /// Crowding distance used as tie-breaker within same rank.
    pub crowding_distance: Option<f64>,
}
