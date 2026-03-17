use crate::algorithms::runtime::ExecutionContext;
use crate::algorithms::termination::{
    ExecutionStateSnapshot,
    ImprovementDirection,
    TerminationCriteria,
};
use crate::algorithms::traits::Algorithm;
use crate::experiment::traits::{CaseParameter, ExperimentalCase};
use crate::observer::traits::{AlgorithmObserver, Observable};
use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::solution_set::traits::SolutionSet;
use crate::utils::random::{seed_from_time, Random};

#[derive(Clone)]
pub struct PSOParameters {
    pub swarm_size: usize,
    pub inertia_weight: f64,
    pub cognitive_coefficient: f64,
    pub social_coefficient: f64,
    pub velocity_clamp: f64,
    pub termination_criteria: TerminationCriteria,
    pub random_seed: Option<u64>,
    pub is_maximization: bool,
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
            termination_criteria,
            random_seed: None,
            is_maximization: true,
        }
    }

    pub fn with_velocity_clamp(mut self, velocity_clamp: f64) -> Self {
        self.velocity_clamp = velocity_clamp;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }

    pub fn minimization(mut self) -> Self {
        self.is_maximization = false;
        self
    }

    pub fn maximization(mut self) -> Self {
        self.is_maximization = true;
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
    velocities: Vec<Vec<f64>>,
    personal_best: Vec<Solution<bool>>,
    global_best: Solution<bool>,
    rng: Random,
    iteration: usize,
    evaluations: usize,
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
    fn is_better(&self, candidate: f64, reference: f64) -> bool {
        if self.parameters.is_maximization {
            candidate > reference
        } else {
            candidate < reference
        }
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
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

    fn improvement_direction(&self) -> ImprovementDirection {
        if self.parameters.is_maximization {
            ImprovementDirection::Maximize
        } else {
            ImprovementDirection::Minimize
        }
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

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<bool> + Sync),
        _context: &ExecutionContext<bool>,
    ) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));

        let mut particles = Vec::with_capacity(self.parameters.swarm_size);
        let mut velocities = Vec::with_capacity(self.parameters.swarm_size);

        for _ in 0..self.parameters.swarm_size {
            let mut particle = problem.create_solution(&mut rng);
            problem.evaluate(&mut particle);

            let dimension = particle.num_variables();
            let velocity: Vec<f64> = (0..dimension)
                .map(|_| (rng.next_f64() * 2.0 - 1.0) * self.parameters.velocity_clamp)
                .collect();

            particles.push(particle);
            velocities.push(velocity);
        }

        let personal_best = particles.clone();

        let global_best = personal_best
            .iter()
            .cloned()
            .reduce(|a, b| {
                if self.is_better(b.quality_value(), a.quality_value()) {
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
        _context: &ExecutionContext<bool>,
    ) {
        state.iteration += 1;

        for i in 0..state.particles.len() {
            let dimension = state.particles[i].num_variables();
            for d in 0..dimension {
                let x = if state.particles[i].variables[d] { 1.0 } else { 0.0 };
                let p = if state.personal_best[i].variables[d] { 1.0 } else { 0.0 };
                let g = if state.global_best.variables[d] { 1.0 } else { 0.0 };

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
                state.particles[i].variables[d] = state.rng.next_f64() < flip_probability;
            }

            state.particles[i].invalidate();
            problem.evaluate(&mut state.particles[i]);
            state.evaluations += 1;

            if self.is_better(
                state.particles[i].quality_value(),
                state.personal_best[i].quality_value(),
            ) {
                state.personal_best[i] = state.particles[i].copy();
            }

            if self.is_better(
                state.personal_best[i].quality_value(),
                state.global_best.quality_value(),
            ) {
                state.global_best = state.personal_best[i].copy();
            }
        }
    }

    fn snapshot(&self, state: &Self::StepState) -> ExecutionStateSnapshot<bool> {
        if state.particles.is_empty() {
            return ExecutionStateSnapshot::new(
                0,
                state.iteration,
                state.evaluations,
                state.global_best.copy(),
                state.global_best.quality_value(),
                0.0,
                0.0,
            );
        }

        let values: Vec<f64> = state.particles.iter().map(|s| s.quality_value()).collect();
        let average = values.iter().sum::<f64>() / values.len() as f64;

        let (best_value, worst_value) = if self.parameters.is_maximization {
            (
                values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                values.iter().cloned().fold(f64::INFINITY, f64::min),
            )
        } else {
            (
                values.iter().cloned().fold(f64::INFINITY, f64::min),
                values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            )
        };

        ExecutionStateSnapshot::new(
            0,
            state.iteration,
            state.evaluations,
            state.global_best.copy(),
            best_value,
            average,
            worst_value,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        let mut result = VectorSolutionSet::new();
        result.add_solution(state.global_best);
        result
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
            "{}(swarm={}, w={:.3}, c1={:.3}, c2={:.3}, direction={})",
            "PSO",
            self.swarm_size,
            self.inertia_weight,
            self.cognitive_coefficient,
            self.social_coefficient,
            if self.is_maximization { "max" } else { "min" }
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
            CaseParameter::new("social_coefficient", format!("{:.6}", self.social_coefficient)),
            CaseParameter::new("velocity_clamp", format!("{:.6}", self.velocity_clamp)),
            CaseParameter::new(
                "direction",
                if self.is_maximization {
                    "maximize"
                } else {
                    "minimize"
                },
            ),
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
        .with_seed(42)
        .maximization();

        let mut pso = PSO::new(params);
        let result = pso.run(&problem).expect("PSO should run");
        assert_eq!(result.solutions().len(), 1);
        assert!(result.solutions()[0].has_quality());
    }
}
