use crate::algorithms::runtime::ExecutionContext;
use crate::algorithms::termination::{ExecutionStateSnapshot, TerminationCriteria};
use crate::algorithms::traits::Algorithm;
use crate::observer::traits::AlgorithmObserver;
use crate::observer::Observable;
use crate::operator::traits::{CrossoverOperator, MutationOperator, SelectionOperator};
use crate::problem::traits::Problem;
use crate::solution::{Solution,ParetoCrowdingDistanceQuality};
use crate::solution_set::implementations::vector_solution_set::VectorSolutionSet;
use crate::utils::checkpoint::StepStateCheckpoint;
use crate::utils::parallel::parallel_map_indexed;
use crate::utils::random::{Random, seed_from_time};
use std::cmp::Ordering;

pub struct NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    pub population_size: usize,
    pub crossover_probability: f64,
    pub mutation_probability: f64,
    pub crossover_operator: C,
    pub mutation_operator: M,
    pub selection_operator: Sel,
    pub num_threads: Option<usize>,
    pub random_seed: Option<u64>,
    pub termination_criteria: TerminationCriteria,
}

impl<C, M, Sel> NSGAIIParameters<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    pub fn new(
        population_size: usize,
        crossover_probability: f64,
        mutation_probability: f64,
        crossover_operator: C,
        mutation_operator: M,
        selection_operator: Sel,
        termination_criteria: TerminationCriteria,
    ) -> Self {
        Self {
            population_size,
            crossover_probability,
            mutation_probability,
            crossover_operator,
            mutation_operator,
            selection_operator,
            num_threads: None,
            random_seed: None,
            termination_criteria,
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

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.random_seed = Some(seed);
        self
    }
}

pub struct NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    parameters: NSGAIIParameters<C, M, Sel>,
    solution_set: Option<VectorSolutionSet<f64, ParetoCrowdingDistanceQuality>>,
    observers: Vec<Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>>,
}

pub struct NSGAIIState {
    population: Vec<crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>>,
    rng: Random,
    generation: usize,
    evaluations: usize,
}

impl StepStateCheckpoint<f64, ParetoCrowdingDistanceQuality> for NSGAIIState {
    fn random_seed(&self) -> u64 {
        self.rng.state()
    }

    fn evaluations(&self) -> usize {
        self.evaluations
    }

    fn iteration(&self) -> usize {
        self.generation
    }

    fn from_payload(payload: &str) -> Self {

        let parts: std::collections::HashMap<&str, &str> = payload
            .split(';')
            .filter_map(|s| {
                let mut kv = s.splitn(2, '=');
                Some((kv.next()?, kv.next()?))
            })
            .collect();


        let generation = parts.get("iter")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let evaluations = parts.get("eval")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        let random_seed = parts.get("seed")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(seed_from_time);


        let population = parts.get("pop")
            .map(|pop_str| {
                pop_str.trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .filter_map(|sol_str| Solution::decode(sol_str).ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            population,
            rng: Random::new(random_seed),
            generation,
            evaluations,
    }
}

    fn to_payload(&self) -> String {
        let population_encoded = self.population
        .iter()
        .map(|sol| sol.encode())
        .collect::<Vec<String>>()
        .join(","); 

        format!(
            "iter={};eval={};seed={};pop=[{}]",
            self.iteration(),
            self.evaluations(),
            self.random_seed(),
            population_encoded
        )
    }
}

impl<C, M, Sel> NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    fn resolve_num_threads(requested: Option<usize>) -> usize {
        match requested {
            Some(v) => v.max(1),
            None => std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
        }
    }

    fn evaluate_population(
        problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync),
        population: Vec<crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>>,
        requested_threads: Option<usize>,
    ) -> Vec<crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>> {
        let thread_count = Self::resolve_num_threads(requested_threads);

        if thread_count <= 1 {
            let mut evaluated = population;
            for solution in &mut evaluated {
                problem.evaluate(solution);
            }
            return evaluated;
        }

        parallel_map_indexed(&population, Some(thread_count), 1, |_, solution| {
            let mut evaluated = solution.copy();
            problem.evaluate(&mut evaluated);
            evaluated
        })
    }

    fn non_dominated_sort(
        population: &[crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>],
    ) -> Vec<Vec<usize>> {
        let n = population.len();
        let mut domination_counts = vec![0usize; n];
        let mut dominates_list: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut fronts: Vec<Vec<usize>> = Vec::new();
        let mut first_front = Vec::new();

        for p in 0..n {
            for q in 0..n {
                if p == q {
                    continue;
                }

                if population[p].dominates(&population[q]) {
                    dominates_list[p].push(q);
                } else if population[q].dominates(&population[p]) {
                    domination_counts[p] += 1;
                }
            }

            if domination_counts[p] == 0 {
                first_front.push(p);
            }
        }

        if !first_front.is_empty() {
            fronts.push(first_front);
        }

        let mut i = 0usize;
        while i < fronts.len() {
            let mut next_front = Vec::new();
            for &p in &fronts[i] {
                for &q in &dominates_list[p] {
                    domination_counts[q] = domination_counts[q].saturating_sub(1);
                    if domination_counts[q] == 0 {
                        next_front.push(q);
                    }
                }
            }

            if !next_front.is_empty() {
                fronts.push(next_front);
            }
            i += 1;
        }

        fronts
    }

    fn assign_front_crowding(
        front: &[usize],
        population: &mut [crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>],
    ) {
        if front.is_empty() {
            return;
        }

        for &idx in front {
            population[idx].set_crowding_distance(0.0);
        }

        let objectives_len = population[front[0]]
            .get_objectives()
            .map(|o| o.len())
            .unwrap_or(0);

        if objectives_len == 0 {
            return;
        }

        if front.len() <= 2 {
            for &idx in front {
                population[idx].set_crowding_distance(f64::INFINITY);
            }
            return;
        }

        for objective in 0..objectives_len {
            let mut sorted = front.to_vec();
            sorted.sort_by(|&a, &b| {
                let av = population[a]
                    .get_objective(objective)
                    .unwrap_or(f64::INFINITY);
                let bv = population[b]
                    .get_objective(objective)
                    .unwrap_or(f64::INFINITY);
                av.partial_cmp(&bv).unwrap_or(Ordering::Equal)
            });

            let first = sorted[0];
            let last = *sorted.last().expect("sorted front should not be empty");
            population[first].set_crowding_distance(f64::INFINITY);
            population[last].set_crowding_distance(f64::INFINITY);

            let min_val = population[first]
                .get_objective(objective)
                .unwrap_or(f64::INFINITY);
            let max_val = population[last]
                .get_objective(objective)
                .unwrap_or(f64::INFINITY);

            if !min_val.is_finite()
                || !max_val.is_finite()
                || (max_val - min_val).abs() <= f64::EPSILON
            {
                continue;
            }

            for w in 1..(sorted.len() - 1) {
                let idx = sorted[w];
                let prev = population[sorted[w - 1]]
                    .get_objective(objective)
                    .unwrap_or(min_val);
                let next = population[sorted[w + 1]]
                    .get_objective(objective)
                    .unwrap_or(max_val);

                if !prev.is_finite() || !next.is_finite() {
                    continue;
                }

                let current = population[idx].crowding_distance().unwrap_or(0.0);
                if current.is_finite() {
                    let delta = (next - prev) / (max_val - min_val);
                    population[idx].set_crowding_distance(current + delta);
                }
            }
        }
    }

    fn annotate_population(
        population: &mut [crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>],
    ) -> Vec<Vec<usize>> {
        let fronts = Self::non_dominated_sort(population);
        for (rank, front) in fronts.iter().enumerate() {
            for &idx in front {
                population[idx].set_rank(rank);
            }
            Self::assign_front_crowding(front, population);
        }
        fronts
    }

    fn rank_crowding_cmp(
        a: &crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>,
        b: &crate::solution::Solution<f64, ParetoCrowdingDistanceQuality>,
    ) -> Ordering {
        let ar = a.rank().unwrap_or(usize::MAX);
        let br = b.rank().unwrap_or(usize::MAX);

        if ar != br {
            return ar.cmp(&br);
        }

        let ac = a.crowding_distance().unwrap_or(0.0);
        let bc = b.crowding_distance().unwrap_or(0.0);
        bc.partial_cmp(&ac).unwrap_or(Ordering::Equal)
    }
}

impl<C, M, Sel> Observable<f64, ParetoCrowdingDistanceQuality> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    fn add_observer(
        &mut self,
        observer: Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>,
    ) {
        self.observers.push(observer);
    }

    fn clear_observers(&mut self) {
        self.observers.clear();
    }
}

impl<C, M, Sel> Algorithm<f64, ParetoCrowdingDistanceQuality> for NSGAII<C, M, Sel>
where
    C: CrossoverOperator<f64, ParetoCrowdingDistanceQuality>,
    M: MutationOperator<f64, ParetoCrowdingDistanceQuality>,
    Sel: SelectionOperator<f64, ParetoCrowdingDistanceQuality>,
{
    type SolutionSet = VectorSolutionSet<f64, ParetoCrowdingDistanceQuality>;
    type Parameters = NSGAIIParameters<C, M, Sel>;
    type StepState = NSGAIIState;

    fn new(parameters: NSGAIIParameters<C, M, Sel>) -> Self {
        Self {
            parameters,
            solution_set: None,
            observers: Vec::new(),
        }
    }

    fn algorithm_name(&self) -> &str {
        "NSGA-II"
    }

    fn termination_criteria(&self) -> TerminationCriteria {
        self.parameters.termination_criteria.clone()
    }

    fn observers_mut(
        &mut self,
    ) -> &mut Vec<Box<dyn AlgorithmObserver<f64, ParetoCrowdingDistanceQuality>>> {
        &mut self.observers
    }

    fn set_solution_set(&mut self, solution_set: Self::SolutionSet) {
        self.solution_set = Some(solution_set);
    }

    fn validate_parameters(&self) -> Result<(), String> {
        if self.parameters.population_size == 0 {
            return Err("population_size must be > 0".to_string());
        }

        if self.parameters.termination_criteria.is_empty() {
            return Err("termination_criteria must not be empty".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.crossover_probability) {
            return Err("crossover_probability must be in [0,1]".to_string());
        }

        if !(0.0..=1.0).contains(&self.parameters.mutation_probability) {
            return Err("mutation_probability must be in [0,1]".to_string());
        }

        Ok(())
    }

    fn get_solution_set(&self) -> Option<&Self::SolutionSet> {
        self.solution_set.as_ref()
    }

    fn initialize_step_state(
        &self,
        problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync),
    ) -> Self::StepState {
        let mut rng = Random::new(self.parameters.random_seed.unwrap_or_else(seed_from_time));

        let population: Vec<_> = (0..self.parameters.population_size)
            .map(|_| problem.create_solution(&mut rng))
            .collect();

        let mut population =
            Self::evaluate_population(problem, population, self.parameters.num_threads);
        Self::annotate_population(&mut population);

        NSGAIIState {
            population,
            rng,
            generation: 0,
            evaluations: self.parameters.population_size,
        }
    }

    fn step(
        &self,
        problem: &(impl Problem<f64, ParetoCrowdingDistanceQuality> + Sync),
        state: &mut Self::StepState,
        _context: &ExecutionContext<f64, ParetoCrowdingDistanceQuality>,
    ) {
        state.generation += 1;
        let direction = problem.get_improvement_direction();
        let mut offspring = Vec::with_capacity(self.parameters.population_size);

        while offspring.len() < self.parameters.population_size {
            let parent1 = self.parameters.selection_operator.execute(
                &state.population,
                &mut state.rng,
                direction,
            );

            let parent2 = self.parameters.selection_operator.execute(
                &state.population,
                &mut state.rng,
                direction,
            );

            let mut children = if state.rng.next_f64() < self.parameters.crossover_probability {
                self.parameters
                    .crossover_operator
                    .execute(&parent1, &parent2, &mut state.rng)
            } else {
                vec![parent1.copy(), parent2.copy()]
            };

            for child in &mut children {
                self.parameters.mutation_operator.execute(
                    child,
                    self.parameters.mutation_probability,
                    &mut state.rng,
                );
            }

            offspring.extend(children);
        }

        offspring.truncate(self.parameters.population_size);
        offspring = Self::evaluate_population(problem, offspring, self.parameters.num_threads);
        state.evaluations += offspring.len();
        state.population.extend(offspring);

        let fronts = Self::annotate_population(&mut state.population);
        let mut next_population = Vec::with_capacity(self.parameters.population_size);

        for front in fronts {
            if next_population.len() + front.len() <= self.parameters.population_size {
                next_population.extend(front.into_iter().map(|idx| state.population[idx].copy()));
            } else {
                let remaining = self.parameters.population_size - next_population.len();
                if remaining == 0 {
                    break;
                }

                let mut candidates: Vec<_> = front
                    .into_iter()
                    .map(|idx| state.population[idx].copy())
                    .collect();
                candidates.sort_by(Self::rank_crowding_cmp);
                next_population.extend(candidates.into_iter().take(remaining));
                break;
            }
        }

        state.population = next_population;
    }

    fn snapshot(
        &self,
        state: &Self::StepState,
    ) -> ExecutionStateSnapshot<f64, ParetoCrowdingDistanceQuality> {
        let worst = state
            .population
            .iter()
            .filter_map(|s| s.get_objective(0))
            .fold(f64::NEG_INFINITY, f64::max);
        let worst = if worst.is_finite() { worst } else { 0.0 };

        let avg = if state.population.is_empty() {
            0.0
        } else {
            let values: Vec<f64> = state
                .population
                .iter()
                .filter_map(|s| s.get_objective(0))
                .collect();
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        };

        let best_solution = state
            .population
            .iter()
            .min_by(|a, b| Self::rank_crowding_cmp(a, b))
            .map(|solution| solution.copy())
            .expect("population should not be empty when reporting progress");
        let best = best_solution.get_objective(0).unwrap_or(0.0);

        ExecutionStateSnapshot::new(
            0,
            state.generation,
            state.evaluations,
            best_solution,
            best,
            avg,
            worst,
        )
    }

    fn finalize_step_state(&self, state: Self::StepState) -> Self::SolutionSet {
        VectorSolutionSet::from_vec(state.population)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operator::crossover_operator_implementations::sbx_crossover::SBXCrossover;
    use crate::operator::mutation_operator_implementations::polynomial_mutation::PolynomialMutation;
    use crate::operator::selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;
    use crate::solution::MultiObjectiveRealSolutionBuilder;

    type Nsga2Test = NSGAII<SBXCrossover, PolynomialMutation, MultiObjectiveTournamentSelection>;

    #[test]
    fn annotate_population_assigns_expected_ranks() {
        let mut population = vec![
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.2, 0.2])
                .build(),
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.3, 0.4])
                .build(),
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.4, 0.3])
                .build(),
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.1, 0.5])
                .build(),
        ];

        let fronts = Nsga2Test::annotate_population(&mut population);

        assert_eq!(fronts.len(), 2);
        assert_eq!(population[0].rank(), Some(0));
        assert_eq!(population[3].rank(), Some(0));
        assert_eq!(population[1].rank(), Some(1));
        assert_eq!(population[2].rank(), Some(1));
    }

    #[test]
    fn crowding_distance_is_infinite_on_front_boundaries() {
        let mut population = vec![
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.1, 0.9])
                .build(),
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.5, 0.5])
                .build(),
            MultiObjectiveRealSolutionBuilder::from_variables(vec![0.0, 0.0])
                .with_objectives(vec![0.9, 0.1])
                .build(),
        ];

        let fronts = Nsga2Test::annotate_population(&mut population);
        assert_eq!(fronts.len(), 1);

        // Boundary points should have infinite crowding distance.
        assert!(population[0]
            .crowding_distance()
            .expect("crowding must be assigned")
            .is_infinite());
        assert!(population[2]
            .crowding_distance()
            .expect("crowding must be assigned")
            .is_infinite());

        // Interior point should be finite and positive.
        let interior = population[1]
            .crowding_distance()
            .expect("crowding must be assigned");
        assert!(interior.is_finite());
        assert!(interior > 0.0);
    }
}
