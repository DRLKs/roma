use crate::observer::AlgorithmEvent;
use crate::observer::traits::{AlgorithmObserver};

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
                if self.verbose || state.iteration % 10 == 0 {
                    println!(
                        "Generation {}: Evaluations={}, Best={:.4}, Avg={:.4}, Worst={:.4}",
                        state.iteration,
                        state.evaluations,
                        state.best_fitness,
                        state.average_fitness,
                        state.worst_fitness
                    );
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
