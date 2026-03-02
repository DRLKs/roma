use std::time::{Duration, Instant};

/// Enum que define diferentes criterios de parada para los algoritmos de optimización.
/// Los algoritmos pueden terminar cuando se cumple cualquiera de estos criterios.
#[derive(Clone, Debug)]
pub enum TerminationCriterion {
    /// Número máximo de iteraciones (generaciones, pasos, etc.)
    MaxIterations(usize),
    /// Número máximo de evaluaciones de la función objetivo
    MaxEvaluations(usize),
    /// Convergencia: el algoritmo para cuando el cambio relativo en la mejor calidad
    /// es menor que el umbral durante 'patience' iteraciones consecutivas
    Convergence { threshold: f64, patience: usize },
    /// Límite de tiempo de ejecución
    TimeLimit(Duration),
    /// La mejor solución no mejora durante 'patience' iteraciones
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

/// Estado interno para rastrear el progreso de los criterios de parada
#[derive(Clone, Debug)]
pub struct TerminationState {
    pub start_time: Instant,
    pub current_iterations: usize,
    pub current_evaluations: usize,
    pub best_quality_history: Vec<f64>, // Para convergencia y no mejora
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

    /// Actualiza el estado con la nueva mejor calidad encontrada
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
                    range / avg.abs() < *threshold // Cambio relativo
                }
            }
            TerminationCriterion::NoImprovement { patience } => {
                self.current_iterations - self.last_improvement_iteration >= *patience
            }
        }
    }
}