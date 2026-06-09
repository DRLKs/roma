use crate::observer::traits::AlgorithmObserver;
use crate::observer::{AlgorithmEvent, ObserverState};

const HOW_ITERATIONS_TO_PRINT: usize = 150;

/// Simple console observer that prints algorithm progress to stdout
pub struct ConsoleObserver {
    name: String,
    verbose: bool,
    last_snapshot_seq: Option<u64>,
}

impl ConsoleObserver {
    pub fn new(verbose: bool) -> Self {
        ConsoleObserver {
            name: "ConsoleObserver".to_string(),
            verbose,
            last_snapshot_seq: None,
        }
    }

    fn format_progress_line(state: &ObserverState) -> String {
        format!(
            "Generation {}: Evaluations={}, Best={:.4}, Avg={:.4}, Worst={:.4}",
            state.iteration,
            state.evaluations,
            state.best_fitness,
            state.average_fitness,
            state.worst_fitness
        )
    }

    fn format_best_solution_line(state: &ObserverState) -> Option<String> {
        let presentation = state.best_solution_presentation.trim();
        if presentation.is_empty() {
            None
        } else {
            Some(format!("Best solution: {}", presentation))
        }
    }
}

impl<T, Q> AlgorithmObserver<T, Q> for ConsoleObserver
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    fn update(&mut self, event: &AlgorithmEvent<T, Q>) {
        match event {
            AlgorithmEvent::Start { algorithm_name } => {
                println!("  Starting algorithm: {}", algorithm_name);
                self.last_snapshot_seq = None;
            }
            AlgorithmEvent::ExecutionStateUpdated { state } => {
                if let Some(last_seq) = self.last_snapshot_seq {
                    if state.seq_id <= last_seq {
                        return;
                    }
                }

                self.last_snapshot_seq = Some(state.seq_id);
                if self.verbose || state.iteration % HOW_ITERATIONS_TO_PRINT == 0 {
                    println!("{}", Self::format_progress_line(state));
                    if let Some(best_solution_line) = Self::format_best_solution_line(state) {
                        println!("  {}", best_solution_line);
                    }
                }
            }
            AlgorithmEvent::End {
                total_generations,
                total_evaluations,
                termination_reason,
            } => {
                println!(
                    "  Algorithm finished: {} generations, {} evaluations",
                    total_generations, total_evaluations
                );
                if self.verbose {
                    if let Some(reason) = termination_reason {
                        println!("  Termination reason: {:?}", reason);
                    }
                }
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::ConsoleObserver;
    use crate::observer::ObserverState;

    fn best_solution(presentation: &str) -> ObserverState {
        ObserverState::new(0, 5, 120, 2.0, 1.5, 0.5, presentation.to_string())
    }

    #[test]
    fn formats_progress_line_with_metrics() {
        let line = ConsoleObserver::format_progress_line(&best_solution("selected=2/3"));

        assert_eq!(
            line,
            "Generation 5: Evaluations=120, Best=2.0000, Avg=1.5000, Worst=0.5000"
        );
    }

    #[test]
    fn formats_best_solution_line_with_problem_presentation() {
        let line = ConsoleObserver::format_best_solution_line(&best_solution("selected=2/3"));

        assert_eq!(line.as_deref(), Some("Best solution: selected=2/3"));
    }

    #[test]
    fn omits_blank_best_solution_presentations() {
        let line = ConsoleObserver::format_best_solution_line(&best_solution("   \n  "));

        assert_eq!(line, None);
    }
}
