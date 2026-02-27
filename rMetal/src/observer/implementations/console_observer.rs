use crate::observer::AlgorithmEvent;
use crate::observer::traits::{AlgorithmObserver};

/// Simple console observer that prints algorithm progress to stdout
pub struct ConsoleObserver {
    name: String,
    verbose: bool,
}

impl ConsoleObserver {
    pub fn new(verbose: bool) -> Self {
        ConsoleObserver {
            name: "ConsoleObserver".to_string(),
            verbose,
        }
    }
}

impl<T> AlgorithmObserver<T> for ConsoleObserver
where
    T: Clone + Send + 'static,
{
    fn update(&mut self, event: &AlgorithmEvent<T>) {
        match event {
            AlgorithmEvent::Start { algorithm_name } => {
                println!("  Starting algorithm: {}", algorithm_name);
            }
            AlgorithmEvent::GenerationCompleted {
                generation,
                evaluations,
                best_fitness,
                worst_fitness,
                average_fitness,
            } => {
                if self.verbose || generation % 10 == 0 {
                    println!(
                        "Generation {}: Evaluations={}, Best={:.4}, Avg={:.4}, Worst={:.4}",
                        generation, evaluations, best_fitness, average_fitness, worst_fitness
                    );
                }
            }
            AlgorithmEvent::BestSolutionUpdate { generation, solution } => {
                if self.verbose {
                    println!(
                        "  New best solution found at generation {}: fitness={:.4}",
                        generation,
                        solution.fitness().unwrap_or(f64::NEG_INFINITY)
                    );
                }
            }
            AlgorithmEvent::End {
                total_generations,
                total_evaluations,
            } => {
                println!(
                    "  Algorithm finished: {} generations, {} evaluations",
                    total_generations, total_evaluations
                );
            }
            AlgorithmEvent::Error { message } => {
                eprintln!("  Error: {}", message);
                eprintln!("   Algorithm execution stopped due to validation error.");
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
