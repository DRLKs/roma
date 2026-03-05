use std::time::{Duration, Instant};
use crate::solution::Solution;

/// Defines stopping criteria for optimization algorithms.
///
/// A run stops when any configured criterion is satisfied.
#[derive(Clone, Debug)]
pub enum TerminationCriterion {
    /// Maximum number of iterations (generations, steps, etc.).
    MaxIterations(usize),
    /// Maximum number of objective-function evaluations.
    MaxEvaluations(usize),
    /// Convergence criterion: stop when the relative change in best quality
    /// is below `threshold` for `patience` consecutive iterations.
    Convergence { threshold: f64, patience: usize },
    /// Wall-clock time limit.
    TimeLimit(Duration),
    /// Stop when best quality does not improve for `patience` iterations.
    NoImprovement { patience: usize },
}

/// Aggregates termination criteria in a single structure.
#[derive(Clone, Debug)]
pub struct TerminationCriteria {
    criteria: Vec<TerminationCriterion>,
}

impl TerminationCriteria {
    pub fn new(criteria: Vec<TerminationCriterion>) -> Self {
        Self { criteria }
    }

    pub fn is_empty(&self) -> bool {
        self.criteria.is_empty()
    }

    pub fn all(&self) -> &[TerminationCriterion] {
        self.criteria.as_slice()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImprovementDirection {
    Maximize,
    Minimize,
}

#[derive(Clone, Debug)]
pub enum TerminationReason {
    Criterion(TerminationCriterion),
}

/// Shared execution snapshot emitted by algorithms and consumed by
/// termination logic/observers.
#[derive(Clone, Debug)]
pub struct ExecutionStateSnapshot<T, Q = f64>
where
    T: Clone,
    Q: Clone,
{
    pub seq_id: u64,
    pub iteration: usize,
    pub evaluations: usize,
    pub best_solution: Solution<T, Q>,
    /// Cached scalar metric for termination/monitoring.
    pub best_fitness: f64,
    pub average_fitness: f64,
    pub worst_fitness: f64,
}

impl<T, Q> ExecutionStateSnapshot<T, Q>
where
    T: Clone,
    Q: Clone,
{
    pub fn new(
        seq_id: u64,
        iteration: usize,
        evaluations: usize,
        best_solution: Solution<T, Q>,
        best_fitness: f64,
        average_fitness: f64,
        worst_fitness: f64,
    ) -> Self {
        Self {
            seq_id,
            iteration,
            evaluations,
            best_solution,
            best_fitness,
            average_fitness,
            worst_fitness,
        }
    }
}

/// Internal state used to track stopping-criteria progress.
#[derive(Clone, Debug)]
pub struct TerminationState {
    pub start_time: Instant,
    pub current_iterations: usize,
    pub current_evaluations: usize,
    /// Best-quality history used by convergence and no-improvement criteria.
    pub best_quality_history: Vec<f64>,
    pub last_improvement_iteration: usize,
}

#[derive(Clone, Debug)]
pub struct TerminationController {
    criteria: TerminationCriteria,
    direction: ImprovementDirection,
    state: TerminationState,
    reason: Option<TerminationReason>,
}

impl TerminationController {
    pub fn new(criteria: TerminationCriteria, direction: ImprovementDirection) -> Self {
        Self {
            criteria,
            direction,
            state: TerminationState::new(),
            reason: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.criteria.is_empty()
    }

    pub fn on_iteration(&mut self, iteration: usize) {
        self.state.current_iterations = iteration;
    }

    pub fn on_evaluations(&mut self, evaluations: usize) {
        self.state.current_evaluations = evaluations;
    }

    pub fn on_best_quality(&mut self, quality: f64, iteration: usize) {
        self.state.update_best_quality(quality, iteration, self.direction);
    }

    pub fn on_snapshot<T, Q>(&mut self, snapshot: &ExecutionStateSnapshot<T, Q>)
    where
        T: Clone,
        Q: Clone,
    {
        self.on_iteration(snapshot.iteration);
        self.on_evaluations(snapshot.evaluations);
        self.on_best_quality(snapshot.best_fitness, snapshot.iteration);
    }

    pub fn should_terminate(&mut self) -> bool {
        for criterion in self.criteria.all() {
            if self.state.check_criterion(criterion) {
                self.reason = Some(TerminationReason::Criterion(criterion.clone()));
                return true;
            }
        }
        false
    }

    pub fn reason(&self) -> Option<&TerminationReason> {
        self.reason.as_ref()
    }
}

impl TerminationState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            current_iterations: 0,
            current_evaluations: 0,
            best_quality_history: Vec::new(),
            last_improvement_iteration: 0,
        }
    }

    /// Updates state with a newly observed best quality value.
    pub fn update_best_quality(
        &mut self,
        new_quality: f64,
        iteration: usize,
        direction: ImprovementDirection,
    ) {
        self.best_quality_history.push(new_quality);
        if self.best_quality_history.len() > 1 {
            let prev = self.best_quality_history[self.best_quality_history.len() - 2];
            let improved = match direction {
                ImprovementDirection::Maximize => new_quality > prev,
                ImprovementDirection::Minimize => new_quality < prev,
            };

            if improved {
                self.last_improvement_iteration = iteration;
            }
        }
    }

    fn check_criterion(&self, criterion: &TerminationCriterion) -> bool {
        match criterion {
            TerminationCriterion::MaxIterations(max) => self.current_iterations >= *max,
            TerminationCriterion::MaxEvaluations(max) => self.current_evaluations >= *max,
            TerminationCriterion::TimeLimit(duration) => self.start_time.elapsed() >= *duration,
            TerminationCriterion::Convergence { threshold, patience } => {
                if self.best_quality_history.len() < *patience + 1 {
                    false
                } else {
                    let recent = &self.best_quality_history[self.best_quality_history.len() - patience..];
                    let max_recent = recent.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    let min_recent = recent.iter().cloned().fold(f64::INFINITY, f64::min);
                    let range = max_recent - min_recent;
                    let avg = recent.iter().sum::<f64>() / recent.len() as f64;

                    if avg.abs() <= f64::EPSILON {
                        range <= *threshold
                    } else {
                        range / avg.abs() < *threshold
                    }
                }
            }
            TerminationCriterion::NoImprovement { patience } => {
                self.current_iterations
                    .saturating_sub(self.last_improvement_iteration)
                    >= *patience
            }
        }
    }
}