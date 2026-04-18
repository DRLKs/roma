use rmetal::algorithms::{
    Algorithm, HillClimbing, HillClimbingParameters, TerminationCriteria, TerminationCriterion,
};
use rmetal::observer::{ChartObserver, ConsoleObserver, HtmlReportObserver, Observable};
use rmetal::operator::{MutationOperator, Operator};
use rmetal::problem::KnapsackBuilder;
use rmetal::solution::Solution;
use rmetal::utils::{
    latest_checkpoint_record_for_algorithm, resolve_checkpoint_dir, CheckpointPathConfig,
    CheckpointRunStatus, Random,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct PanicMutation {
    calls: Arc<AtomicUsize>,
    panic_on_call: usize,
}

impl PanicMutation {
    fn new(panic_on_call: usize) -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
            panic_on_call,
        }
    }
}

impl Operator for PanicMutation {
    fn name(&self) -> &str {
        "BitFlipMutation"
    }
}

impl MutationOperator<bool> for PanicMutation {
    fn execute(&self, solution: &mut Solution<bool>, probability: f64, rng: &mut Random) {
        let call_number = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        if call_number >= self.panic_on_call {
            panic!(
                "intentional failure from PanicMutation at call {}",
                call_number
            );
        }

        for index in 0..solution.num_variables() {
            if rng.next_f64() < probability {
                let value = solution
                    .get_variable(index)
                    .copied()
                    .expect("index must be valid within mutation loop");
                solution.set_variable(index, !value);
            }
        }
    }
}

fn demo_output_base() -> PathBuf {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("failure_demo")
        .join(format!("run_{}", run_id))
}

fn main() {
    let output_base = demo_output_base();
    let chart_output = output_base.join("charts");
    let report_output = output_base.join("reports");

    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();

    let parameters = HillClimbingParameters::new(
        PanicMutation::new(9),
        0.10,
        TerminationCriteria::new(vec![TerminationCriterion::MaxIterations(150)]),
    )
    .with_seed(42);

    let mut algorithm = HillClimbing::new(parameters);
    algorithm.add_observer(Box::new(ConsoleObserver::new(true)));
    algorithm.add_observer(Box::new(
        ChartObserver::new(chart_output).with_flat_output(),
    ));
    algorithm.add_observer(Box::new(
        HtmlReportObserver::new(report_output.clone()).with_flat_output(),
    ));

    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = algorithm.run(&problem);
    }));

    match run_result {
        Ok(_) => {
            println!("Unexpected: demo did not panic. Increase panic_on_call to force failure.")
        }
        Err(_) => {
            let report_path = report_output.join("report.html");
            println!("Failure injected as expected.");
            println!("Report generated at: {}", report_path.display());
            println!("Open it and verify status is 'Failed' with the panic message.");

            let checkpoint_dir = resolve_checkpoint_dir(&CheckpointPathConfig::default());
            println!("Checkpoint directory: {}", checkpoint_dir.display());

            if let Some(record) =
                latest_checkpoint_record_for_algorithm(&checkpoint_dir, "HillClimbing")
                    .ok()
                    .flatten()
            {
                println!(
                    "Recovered checkpoint: run={} status={} seq={} iter={} evals={} best={:.6}",
                    record.run_id,
                    record.status.as_str(),
                    record.seq_id,
                    record.iteration,
                    record.evaluations,
                    record.best_fitness
                );

                match record.status {
                    CheckpointRunStatus::Failed => {
                        println!("Latest checkpoint status: failed (good candidate to resume).");
                    }
                    status => {
                        println!("Latest checkpoint status: {}", status.as_str());
                    }
                }
            } else {
                println!("No checkpoint found for HillClimbing.");
            }
        }
    }
}
