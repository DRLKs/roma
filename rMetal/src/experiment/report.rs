/// Optimization direction used to sort experiment comparisons.
use crate::ImprovementDirection;

#[derive(Debug, Clone)]
pub struct ExperimentRunResult {
    pub algorithm_name: String,
    pub case_name: String,
    pub run_index: usize,
    pub best_value: f64,
}

#[derive(Debug, Clone)]
pub struct ExperimentFailure {
    pub algorithm_name: String,
    pub case_name: String,
    pub run_index: usize,
    pub error: String,
}

#[derive(Debug, Clone)]
pub struct ExperimentSummary {
    pub algorithm_name: String,
    pub case_name: String,
    pub parameters_text: String,
    /// Number of valid runs
    pub runs_ok: usize,
    pub best: f64,
    pub mean: f64,
    pub worst: f64,
    pub std_dev: f64,
}

#[derive(Debug, Clone)]
pub struct ExperimentReport {
    pub objective: ImprovementDirection,
    pub runs_per_case: usize,
    pub run_results: Vec<ExperimentRunResult>,
    pub failures: Vec<ExperimentFailure>,
    pub summaries: Vec<ExperimentSummary>,
}

use std::fmt::Write;

impl ExperimentReport {
    /// Builds a plain-text report ready for terminal output.
    pub fn to_text_table(&self) -> String {
        let mut out = String::new();

        let _ = writeln!(out, "=== Experiment Report ===");
        let _ = writeln!(out, "Objective: {:?}", self.objective);
        let _ = writeln!(out, "Runs per case: {}", self.runs_per_case);
        let _ = writeln!(out, "Successful runs: {}", self.run_results.len());
        let _ = writeln!(out, "Failed runs: {}", self.failures.len());
        let _ = writeln!(out);

        if self.summaries.is_empty() {
            let _ = writeln!(out, "No successful runs to summarize.");
            return out;
        }

        let _ = writeln!(out, "-- Case summaries --");
        for (i, s) in self.summaries.iter().enumerate() {
            let _ = writeln!(out, "{}. {}", i + 1, s.case_name);
            let _ = writeln!(out, "   Algorithm : {}", s.algorithm_name);
            let _ = writeln!(out, "   Params    : {}", s.parameters_text);
            let _ = writeln!(out, "   Runs ok   : {}/{}", s.runs_ok, self.runs_per_case);
            let _ = writeln!(out, "   Best      : {:.6}", s.best);
            let _ = writeln!(out, "   Mean      : {:.6}", s.mean);
            let _ = writeln!(out, "   Worst     : {:.6}", s.worst);
            let _ = writeln!(out, "   Std dev   : {:.6}", s.std_dev);
            let _ = writeln!(out);
        }

        if !self.failures.is_empty() {
            let _ = writeln!(out, "-- Failures --");
            for failure in &self.failures {
                let _ = writeln!(
                    out,
                    "- {} | run={} | {}",
                    failure.case_name, failure.run_index, failure.error
                );
            }
        }

        out
    }
}
