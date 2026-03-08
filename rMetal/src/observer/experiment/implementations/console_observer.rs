use crate::observer::experiment::traits::ExperimentObserver;
use crate::observer::experiment::ExperimentEvent;

/// Console observer specialized for experiment execution.
pub struct ExperimentConsoleObserver {
    name: String,
    verbose_runs: bool,
}

impl ExperimentConsoleObserver {
    pub fn new(verbose_runs: bool) -> Self {
        Self {
            name: "ExperimentConsoleObserver".to_string(),
            verbose_runs,
        }
    }
}

impl ExperimentObserver for ExperimentConsoleObserver {
    fn update(&mut self, event: &ExperimentEvent) {
        match event {
            ExperimentEvent::Start {
                name,
                objective,
                runs_per_case,
                total_cases,
            } => {
                println!(
                    "Experiment '{}' started | objective={:?} | runs/case={} | cases={}",
                    name, objective, runs_per_case, total_cases
                );
            }
            ExperimentEvent::CaseStarted {
                algorithm,
                configuration,
                problem,
            } => {
                println!(
                    "  Case started: algorithm={} configuration={} problem={}",
                    algorithm, configuration, problem
                );
            }
            ExperimentEvent::RunCompleted {
                algorithm,
                configuration,
                problem,
                run_index,
                seed,
                best_value,
            } => {
                if self.verbose_runs {
                    println!(
                        "    Run {:>3} | alg={} cfg={} problem={} seed={} best={:.6}",
                        run_index, algorithm, configuration, problem, seed, best_value
                    );
                }
            }
            ExperimentEvent::End { report } => {
                println!(
                    "Experiment '{}' finished. Cases={}, runs/case={}, total runs={}",
                    report.name,
                    report.summaries.len(),
                    report.runs_per_case,
                    report.run_results.len()
                );

                println!("\nComparison (sorted by mean):");
                for s in report.comparison() {
                    println!(
                        "  {:>4} {:>36} | best={:8.3} mean={:8.3} std={:7.3} worst={:8.3} runs={} | problem={}",
                        s.algorithm,
                        s.configuration,
                        s.best,
                        s.mean,
                        s.std_dev,
                        s.worst,
                        s.runs,
                        s.problem
                    );
                }
            }
            ExperimentEvent::Error {
                algorithm,
                configuration,
                problem,
                message,
            } => {
                eprintln!(
                    "  Case skipped: algorithm={} configuration={} problem={} | error={}",
                    algorithm, configuration, problem, message
                );
            }
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
