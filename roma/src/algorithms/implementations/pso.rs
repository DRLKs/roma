use crate::algorithms::checkpoint::{ExecutionStateSnapshot, StepStateCheckpoint};
use crate::algorithms::termination::TerminationCriteria;
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::parallel::parallel_map_indexed;
use crate::utils::random::{seed_from_time, Random};
use crate::utils::statistics::calculate_population_statistics;

/// Configuration parameters for Binary PSO.
///
/// `PSO` in this library is currently specialized for `Solution<bool>`.
#[derive(Clone)]
pub struct PSOParameters {
    pub swarm_size: usize,
    /// Inertia factor (`w`) that controls how much previous velocity is kept.
    pub inertia_weight: f64,
    /// Cognitive acceleration (`c1`) towards each particle's personal best.
    pub cognitive_coefficient: f64,
    /// Social acceleration (`c2`) towards the global best.
    pub social_coefficient: f64,
    /// Maximum absolute velocity value used to clamp updates.
    pub velocity_clamp: f64,
    pub num_threads: Option<usize>,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
}

impl PSOParameters {
    pub fn new(
        swarm_size: usize,
        inertia_weight: f64,
        cognitive_coefficient: f64,
        social_coefficient: f64,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            swarm_size,
            inertia_weight,
            cognitive_coefficient,
            social_coefficient,
            velocity_clamp: 4.0,
            num_threads: None,
            termination_criteria,
            random_seed: None,
        }
    }

    pub fn with_threads(mut self, num_threads: usize) -> Self {
        self.num_threads = Some(num_threads.max(1));
        self
    }

    pub fn with_parallel(mut self) -> Self {
        self.num_threads = None;
        self
    }

    pub fn sequential(mut self) -> Self {
        self.num_threads = Some(1);
        self
    }

    pub fn with_velocity_clamp(mut self, velocity_clamp: f64) -> Self {
        self.velocity_clamp = velocity_clamp;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

pub struct PSO {
    parameters: PSOParameters,
    solution_set: Option<VectorSolutionSet<bool>>,
    observers: Vec<Box<dyn AlgorithmObserver<bool>>>,
}

pub struct PSOState {
    particles: Vec<Solution<bool>>,
    /// Real-valued velocity vectors associated with particle dimensions.
    velocities: Vec<Vec<f64>>,
    personal_best: Vec<Solution<bool>>,
    global_best: Solution<bool>,
    rng: Random,
    iteration: usize,
    evaluations: usize,
}

impl StepStateCheckpoint<bool, f64> for PSOState {
    fn random_seed(&self) -> u64 {
        self.rng.state()
    }

    fn evaluations(&self) -> usize {
        self.evaluations
    }

    fn iteration(&self) -> usize {
        self.iteration
    }

    fn to_payload(&self) -> String {
        let encoded_particles = self
            .particles
            .iter()
            .map(|p| p.encode())
            .collect::<Vec<_>>()
            .join(",");

        let encoded_velocities = self
            .velocities
            .iter()
            .map(|v| {
                v.iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .collect::<Vec<_>>()
            .join(",");

        let encoded_p_bests = self
            .personal_best
            .iter()
            .map(|p| p.encode())
            .collect::<Vec<_>>()
            .join(",");

        let encoded_g_best = self.global_best.encode();

        format!(
            "iter={};eval={};particles=[{}];vels=[{}];pbests=[{}];gbest={}",
            self.iteration,
            self.evaluations,
            encoded_particles,
            encoded_velocities,
            encoded_p_bests,
            encoded_g_best
        )
    }

    fn from_payload(payload: &str) -> Self {
        let parts: std::collections::HashMap<&str, &str> = payload
            .split(';')
            .filter_map(|s| {
                let mut kv = s.splitn(2, '=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();

        let split_list = |key: &str| {
            parts
                .get(key)
                .map(|s| s.trim_matches(|c| c == '[' || c == ']').split(','))
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty())
        };

        let iteration = parts.get("iter").and_then(|s| s.parse().ok()).unwrap_or(0);
        let evaluations = parts.get("eval").and_then(|s| s.parse().ok()).unwrap_or(0);
        let particles = split_list("particles")
            .filter_map(|s| Solution::decode(s).ok())
            .collect();

        let personal_best = split_list("pbests")
            .filter_map(|s| Solution::decode(s).ok())
            .collect();

        let global_best = parts
            .get("gbest")
            .and_then(|s| Solution::decode(s).ok())
            .expect("Error crítico: No se encontró el global_best en el payload");

        let velocities = split_list("vels")
            .map(|v_str| {
                v_str
                    .split('|')
                    .filter_map(|f| f.parse::<f64>().ok())
                    .collect::<Vec<f64>>()
            })
            .collect();

        Self {
            particles,
            velocities,
            personal_best,
            global_best,
            rng: Random::new(seed_from_time()),
            iteration,
            evaluations,
        }
    }
}

impl Observable<bool> for PSO {
    fn add_observer(&mut self, observer: Box<dyn AlgorithmObserver<bool>>) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl PSO {
    /// Logistic transfer function used by Binary PSO.
    ///
    /// Maps velocity to a probability in `(0, 1)`, then the bit value is sampled
    /// from that probability.
    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }

    fn resolve_num_threads(requested: Option<usize>) -> usize {
        match requested {
            Some(v) => v.max(1),
            None => std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }

    fn evaluate_particles(
        problem: &(impl Problem<bool> + Sync),
        particles: &[Solution<bool>],
        requested_threads: Option<usize>,
    ) -> Vec<Solution<bool>> {
        let thread_count = Self::resolve_num_threads(requested_threads);

        if thread_count <= 1 {
            let mut evaluated = particles.to_vec();
            for particle in &mut evaluated {
                problem.evaluate(particle);
            }
            return evaluated;
        }

        parallel_map_indexed(particles, Some(thread_count), 1, |_, particle| {
            let mut evaluated = particle.copy();
            problem.evaluate(&mut evaluated);
            evaluated
        })
    }
}

impl Algorithm<bool> for PSO {
    type SolutionSet = VectorSolutionSet<bool>;
    type Parameters = PSOParameters;
    type StepState = PSOState;

    fn new(parameters: Self::Parameters) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "PSO"
    }

    fn termination_criteria(&self) -> TerminationCriteria {
        self.parameters.termination_criteria.clone()
    }

    fn observers_mut(&mut self) -> &mut Vec<Box<dyn AlgorithmObserver<bool>>> {
        &mut self.observers
    }

    fn set_solution_set(&mut self, solution_set: Self::SolutionSet) {
        self.solution_set = Some(solution_set);
    }

    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.swarm_size == 0 {
            return Err("swarm_size must be > 0".to_string());
        }

        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !self.parameters.inertia_weight.is_finite() {
            return Err("inertia_weight must be finite".to_string());
        }

        if self.parameters.cognitive_coefficient < 0.0 || self.parameters.social_coefficient < 0.0 {
            return Err("cognitive_coefficient and social_coefficient must be >= 0".to_string());
        }

        if self.parameters.velocity_clamp <= 0.0 || !self.parameters.velocity_clamp.is_finite() {
            return Err("velocity_clamp must be > 0 and finite".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(&self, problem: &(impl Problem<bool> + Sync)) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));

        let mut particles = Vec::with_capacity(self.parameters.swarm_size);
        let mut velocities = Vec::with_capacity(self.parameters.swarm_size);

        for _ in 0..self.parameters.swarm_size {
            let particle = problem.create_solution(&mut rng);

            let dimension = particle.num_variables();
            let velocity: Vec<f64> = (0..dimension)
                .map(|_| (rng.next_f64() * 2.0 - 1.0) * self.parameters.velocity_clamp)
                .collect();

            particles.push(particle);
            velocities.push(velocity);
        }

        particles = Self::evaluate_particles(problem, &particles, self.parameters.num_threads);

        let personal_best = particles.clone();

        let global_best = personal_best
            .iter()
            .cloned()
            .reduce(|a, b| {
                if problem.is_better_fitness(b.quality_value(), a.quality_value()) {
                    b
                } else {
                    a
                }
            })
            .expect("swarm should not be empty when initialized");

        PSOState {
            particles,
            velocities,
            personal_best,
            global_best,
            rng,
            iteration: 0,
            evaluations: self.parameters.swarm_size,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<bool> + Sync),
        state: &mut Self::StepState,
    ) {
        state.iteration += 1;

        for i in 0..state.particles.len() {
            let dimension = state.particles[i].num_variables();
            for d in 0..dimension {
                // Binary PSO update:
                // 1) Build velocity from inertia + cognitive + social components.
                // 2) Clamp velocity for numeric stability.
                // 3) Convert velocity to a Bernoulli probability via sigmoid.
                // 4) Sample new bit value from that probability.
                let x = if *state.particles[i]
                    .get_variable(d)
                    .expect("index must be valid within particle dimension")
                {
                    1.0
                } else {
                    0.0
                };
                let p = if *state.personal_best[i]
                    .get_variable(d)
                    .expect("index must be valid within particle dimension")
                {
                    1.0
                } else {
                    0.0
                };
                let g = if *state
                    .global_best
                    .get_variable(d)
                    .expect("index must be valid within particle dimension")
                {
                    1.0
                } else {
                    0.0
                };

                let r1 = state.rng.next_f64();
                let r2 = state.rng.next_f64();

                let v = self.parameters.inertia_weight * state.velocities[i][d]
                    + self.parameters.cognitive_coefficient * r1 * (p - x)
                    + self.parameters.social_coefficient * r2 * (g - x);

                state.velocities[i][d] = v.clamp(
                    -self.parameters.velocity_clamp,
                    self.parameters.velocity_clamp,
                );

                let flip_probability = Self::sigmoid(state.velocities[i][d]);
                let _ = state.particles[i].set_variable(d, state.rng.next_f64() < flip_probability);
            }
        }

        state.particles =
            Self::evaluate_particles(problem, &state.particles, self.parameters.num_threads);
        state.evaluations += state.particles.len();

        for i in 0..state.particles.len() {
            if problem.is_better_fitness(
                state.particles[i].quality_value(),
                state.personal_best[i].quality_value(),
            ) {
                state.personal_best[i] = state.particles[i].copy();
            }

            if problem.is_better_fitness(
                state.personal_best[i].quality_value(),
                state.global_best.quality_value(),
            ) {
                state.global_best = state.personal_best[i].copy();
            }
        }
    }

    fn build_snapshot(
        &self,
        problem: &(impl Problem<bool> + Sync),
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot {
        let stats = calculate_population_statistics(&state.particles, problem);

        if stats.best_index.is_none() {
            return ExecutionStateSnapshot {
                iteration: state.iteration,
                evaluations: state.evaluations,
                best_fitness: state.global_best.quality_value(),
                worst_fitness: state.global_best.quality_value(),
                average_fitness: state.global_best.quality_value(),
                best_solution_presentation: problem.format_solution(&state.global_best),
            };
        }

        ExecutionStateSnapshot {
            iteration: state.iteration,
            evaluations: state.evaluations,
            best_fitness: state.global_best.quality_value(),
            average_fitness: stats.average_fitness,
            worst_fitness: stats.worst_fitness,
            best_solution_presentation: problem.format_solution(&state.global_best),
        }
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.global_best);
        result
    }

    fn checkpoint_algorithm_parameters(&self) -> String {
        format!(
            "swarm_size={};inertia_weight={:.6};cognitive_coefficient={:.6};social_coefficient={:.6};velocity_clamp={:.6}",
            self.parameters.swarm_size,
            self.parameters.inertia_weight,
            self.parameters.cognitive_coefficient,
            self.parameters.social_coefficient,
            self.parameters.velocity_clamp,
        )
    }
}

impl<P> ExperimentalCase<bool, f64, P> for PSOParameters
where
    P: Problem<bool, f64> + Sync,
{
    fn algorithm_name(&self) -> &str {
        "PSO"
    }

    fn case_name(&self) -> String {
        format!(
            "{}(swarm={}, w={:.3}, c1={:.3}, c2={:.3})",
            "PSO",
            self.swarm_size,
            self.inertia_weight,
            self.cognitive_coefficient,
            self.social_coefficient,
        )
    }

    fn parameters(&self) -> Vec<CaseParameter> {
        vec![
            CaseParameter::new("swarm_size", self.swarm_size.to_string()),
            CaseParameter::new("inertia_weight", format!("{:.6}", self.inertia_weight)),
            CaseParameter::new(
                "cognitive_coefficient",
                format!("{:.6}", self.cognitive_coefficient),
            ),
            CaseParameter::new(
                "social_coefficient",
                format!("{:.6}", self.social_coefficient),
            ),
            CaseParameter::new("velocity_clamp", format!("{:.6}", self.velocity_clamp)),
            CaseParameter::new(
                "termination_criteria",
                format!("{:?}", self.termination_criteria),
            ),
        ]
    }

    fn run(&self, problem: &P) -> Result<Box<dyn SolutionSet<bool, f64>>, String> {
        let result = PSO::new(self.clone()).run(problem)?;
        Ok(Box::new(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::termination::TerminationCriterion;
    use crate::problem::implementations::knapsack_problem::KnapsackBuilder;
    use crate::problem::traits::Problem;
    use crate::solution::Solution;
    use crate::utils::random::Random;

    struct MinOnesProblem {
        num_variables: usize,
    }

    impl Problem<bool> for MinOnesProblem {
        fn new() -> Self {
            Self { num_variables: 0 }
        }

        fn evaluate(&self, solution: &mut Solution<bool>) {
            let ones = solution.variables().iter().filter(|&&x| x).count() as f64;
            solution.set_quality(ones);
        }

        fn create_solution(&self, rng: &mut Random) -> Solution<bool> {
            let vars: Vec<bool> = (0..self.num_variables).map(|_| rng.coin_flip()).collect();
            Solution::new(vars)
        }

        fn set_problem_description(&mut self, _description: String) {}

        fn get_problem_description(&self) -> String {
            "MinOnesProblem".to_string()
        }

        fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
            crate::solution::traits::evaluator::minimizing_fitness
        }

        fn dominates(&self, solution_a: &Solution<bool, f64>, solution_b: &Solution<bool, f64>) -> bool {
            solution_a.quality().copied().unwrap_or(f64::INFINITY)
                < solution_b.quality().copied().unwrap_or(f64::INFINITY)
        }
    }

    #[test]
    fn pso_runs_on_knapsack() {
        let problem = KnapsackBuilder::new()
            .with_capacity(50.0)
            .add_item(10.0, 30.0)
            .add_item(15.0, 40.0)
            .add_item(25.0, 55.0)
            .add_item(30.0, 60.0)
            .build();

        let params = PSOParameters::new(
            20,
            0.72,
            1.49,
            1.49,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(20)]),
        )
        .with_seed(42);

        let mut pso = PSO::new(params);
        let result = pso.run(&problem).expect("PSO should run");
        assert_eq!(result.size(), 1);
        assert!(result.get(0).expect("expected one solution").has_quality());
    }

    #[test]
    fn pso_is_deterministic_with_same_seed() {
        let problem = KnapsackBuilder::new()
            .with_capacity(30.0)
            .add_item(7.0, 12.0)
            .add_item(9.0, 18.0)
            .add_item(12.0, 25.0)
            .build();

        let params = PSOParameters::new(
            16,
            0.7,
            1.4,
            1.4,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(25)]),
        )
        .with_seed(12345);

        let mut run_a = PSO::new(params.clone());
        let mut run_b = PSO::new(params);

        let quality_a = run_a
            .run(&problem)
            .expect("first run should succeed")
            .get(0)
            .expect("result should contain one solution")
            .quality_value();

        let quality_b = run_b
            .run(&problem)
            .expect("second run should succeed")
            .get(0)
            .expect("result should contain one solution")
            .quality_value();

        assert_eq!(quality_a, quality_b);
    }

    #[test]
    fn pso_rejects_non_positive_velocity_clamp() {
        let problem = KnapsackBuilder::new()
            .with_capacity(20.0)
            .add_item(5.0, 10.0)
            .build();

        let params = PSOParameters::new(
            10,
            0.72,
            1.49,
            1.49,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(5)]),
        )
        .with_velocity_clamp(0.0)
        .with_seed(1);

        let mut algorithm = PSO::new(params);
        let error = match algorithm.run(&problem) {
            Ok(_) => panic!("PSO with non-positive velocity clamp should fail validation"),
            Err(message) => message,
        };

        assert!(error.contains("velocity_clamp"));
    }

    #[test]
    fn sigmoid_stays_bounded_for_extreme_velocities() {
        let low = PSO::sigmoid(-1000.0);
        let mid = PSO::sigmoid(0.0);
        let high = PSO::sigmoid(1000.0);

        assert!(low.is_finite());
        assert!(mid.is_finite());
        assert!(high.is_finite());
        assert!((0.0..=1.0).contains(&low));
        assert!((0.0..=1.0).contains(&mid));
        assert!((0.0..=1.0).contains(&high));
        assert!(low < mid);
        assert!(mid < high);
        assert_eq!(mid, 0.5);
    }

    #[test]
    fn pso_honors_minimization_direction_from_problem() {
        let problem = MinOnesProblem { num_variables: 12 };

        let params = PSOParameters::new(
            20,
            0.7,
            1.4,
            1.4,
            TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(30)]),
        )
        .with_seed(99);

        let mut algorithm = PSO::new(params);
        let best = algorithm
            .run(&problem)
            .expect("PSO should run under minimization")
            .get(0)
            .expect("result should contain one solution")
            .quality_value();

        assert!(best >= 0.0);
        assert!(best <= 12.0);
    }
}
