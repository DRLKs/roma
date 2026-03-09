use crate::observer::traits::{AlgorithmObserver};
use crate::utils::chart::{ChartBuilder, Series};
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use crate::observer::AlgorithmEvent;

/// Observer that generates charts showing algorithm progress
pub struct ChartObserver {
    name: String,
    base_output_path: PathBuf,
    run_output_path: Option<PathBuf>,
    use_run_subdirectory: bool,
    
    // Data collection
    generations: Vec<usize>,
    evaluations: Vec<usize>,
    best_fitness_history: Vec<f64>,
    average_fitness_history: Vec<f64>,
    worst_fitness_history: Vec<f64>,
    last_snapshot_seq: Option<u64>,
    
    // Configuration
    chart_width: u32,
    chart_height: u32,
}

impl ChartObserver {
    /// Creates a new ChartObserver
    /// 
    /// The observer creates a structured path per run using this format:
    /// `<base>/<algorithm_slug>/run_<timestamp_ms>_<pid>/`.
    ///
    /// # Arguments
    /// * `base_output_path` - Root directory where run folders will be created
    pub fn new(base_output_path: PathBuf) -> Self {
        ChartObserver {
            name: "ChartObserver".to_string(),
            base_output_path,
            run_output_path: None,
            use_run_subdirectory: true,
            generations: Vec::new(),
            evaluations: Vec::new(),
            best_fitness_history: Vec::new(),
            average_fitness_history: Vec::new(),
            worst_fitness_history: Vec::new(),
            last_snapshot_seq: None,
            chart_width: 1200,
            chart_height: 800,
        }
    }

    /// Creates a `ChartObserver` with a standard base directory.
    pub fn new_default() -> Self {
        Self::new(PathBuf::from("output/charts"))
    }

    /// Disables automatic per-run subdirectories.
    ///
    /// When disabled, charts are written directly inside the base directory.
    pub fn with_flat_output(mut self) -> Self {
        self.use_run_subdirectory = false;
        self
    }

    /// Sets the chart dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.chart_width = width;
        self.chart_height = height;
        self
    }

    fn sanitize_folder_component(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut prev_is_sep = false;

        for ch in raw.chars() {
            let normalized = if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            };

            if normalized == '_' {
                if prev_is_sep {
                    continue;
                }
                prev_is_sep = true;
                out.push('_');
            } else {
                prev_is_sep = false;
                out.push(normalized);
            }
        }

        let trimmed = out.trim_matches('_');
        if trimmed.is_empty() {
            "algorithm".to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn build_run_output_path(&self, algorithm_name: &str) -> PathBuf {
        if !self.use_run_subdirectory {
            return self.base_output_path.clone();
        }

        let algorithm_folder = Self::sanitize_folder_component(algorithm_name);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let pid = std::process::id();

        self.base_output_path
            .join(algorithm_folder)
            .join(format!("run_{}_{}", timestamp_ms, pid))
    }

    fn resolve_output_path(&self) -> PathBuf {
        self.run_output_path
            .clone()
            .unwrap_or_else(|| self.base_output_path.clone())
    }

    fn prepare_output_directory(&mut self, algorithm_name: &str) {
        let output_path = self.build_run_output_path(algorithm_name);
        std::fs::create_dir_all(&output_path).ok();
        self.run_output_path = Some(output_path);
    }

    fn base_chart_builder(&self, title: &str, x_label: &str, y_label: &str) -> ChartBuilder {
        ChartBuilder::new()
            .title(title)
            .x_label(x_label)
            .y_label(y_label)
            .size(self.chart_width, self.chart_height)
            .x_min(0.0)
            .x_clamp_non_negative()
    }

    /// Consolidate data from multiple threads by taking the best value for each generation
    fn consolidate_data(&self) -> (Vec<usize>, Vec<f64>, Vec<f64>, Vec<f64>) {
        use std::collections::HashMap;
        
        let mut gen_map: HashMap<usize, (f64, Vec<f64>, Vec<f64>)> = HashMap::new();
        
        for i in 0..self.generations.len() {
            let generation = self.generations[i];
            let best = self.best_fitness_history[i];
            let avg = self.average_fitness_history[i];
            let worst = self.worst_fitness_history[i];
            
            gen_map.entry(generation)
                .and_modify(|(b, avgs, worsts)| {
                    *b = b.max(best);
                    avgs.push(avg);
                    worsts.push(worst);
                })
                .or_insert((best, vec![avg], vec![worst]));
        }
        
        let mut generations: Vec<usize> = gen_map.keys().copied().collect();
        generations.sort();
        
        let mut best_fitness = Vec::new();
        let mut avg_fitness = Vec::new();
        let mut worst_fitness = Vec::new();
        
        for generation in &generations {
            if let Some((best, avgs, worsts)) = gen_map.get(generation) {
                best_fitness.push(*best);
                // Average of averages from all threads for this generation
                avg_fitness.push(avgs.iter().sum::<f64>() / avgs.len() as f64);
                // Worst of worsts
                worst_fitness.push(*worsts.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0));
            }
        }
        
        (generations, best_fitness, avg_fitness, worst_fitness)
    }

    /// Generates a convergence chart showing fitness evolution over generations
    fn generate_convergence_chart(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.generations.is_empty() {
            return Ok(());
        }

        let output_file = self.resolve_output_path().join("convergence.svg");
        
        let (generations, best_fitness, avg_fitness, worst_fitness) = self.consolidate_data();

        let best_data: Vec<(f64, f64)> = generations.iter()
            .zip(best_fitness.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        let avg_data: Vec<(f64, f64)> = generations.iter()
            .zip(avg_fitness.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        let worst_data: Vec<(f64, f64)> = generations.iter()
            .zip(worst_fitness.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        let best_series = Series::new("Best", best_data).with_color("#2563eb");
        let avg_series = Series::new("Average", avg_data).with_color("#10b981");
        let worst_series = Series::new("Worst", worst_data).with_color("#dc2626");

        let min_solution_value = best_fitness
            .iter()
            .copied()
            .chain(avg_fitness.iter().copied())
            .chain(worst_fitness.iter().copied())
            .fold(f64::INFINITY, f64::min);

        let chart = self
            .base_chart_builder("Convergence", "Generation", "Fitness")
            .y_min(min_solution_value)
            .add_series(best_series)
            .add_series(avg_series)
            .add_series(worst_series)
            .build();

        chart.save(output_file)?;

        Ok(())
    }

    /// Generates a chart showing best fitness as a function of evaluations.
    ///
    /// This chart only includes the best metric (no average/worst series).
    fn generate_best_by_evaluations_chart(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.evaluations.is_empty() || self.best_fitness_history.is_empty() {
            return Ok(());
        }

        use std::collections::BTreeMap;

        let output_file = self.resolve_output_path().join("best_by_evaluations.svg");

        // Keep one point per evaluation value. If repeated, keep the latest observed best.
        let mut points_by_evaluations: BTreeMap<usize, f64> = BTreeMap::new();
        for (evaluations, best) in self
            .evaluations
            .iter()
            .copied()
            .zip(self.best_fitness_history.iter().copied())
        {
            points_by_evaluations.insert(evaluations, best);
        }

        let data: Vec<(f64, f64)> = points_by_evaluations
            .iter()
            .map(|(evaluations, best)| (*evaluations as f64, *best))
            .collect();

        let min_solution_value = data
            .iter()
            .map(|(_, best)| *best)
            .fold(f64::INFINITY, f64::min);

        let series = Series::new("Best", data).with_color("#2563eb");

        let chart = self
            .base_chart_builder("Best Fitness by Evaluations", "Evaluations", "Best Fitness")
            .y_min(min_solution_value)
            .add_series(series)
            .build();

        chart.save(output_file)?;

        Ok(())
    }
}

impl<T, Q> AlgorithmObserver<T, Q> for ChartObserver
where
    T: Clone + Send + 'static,
    Q: Clone + Send + 'static,
{
    fn update(&mut self, event: &AlgorithmEvent<T, Q>) {
        match event {
            AlgorithmEvent::Start { algorithm_name } => {
                println!("  ChartObserver: Monitoring algorithm '{}'", algorithm_name);
                self.prepare_output_directory(algorithm_name);
                println!(
                    "   Charts will be saved to: {}",
                    self.resolve_output_path().display()
                );
                
                self.generations.clear();
                self.evaluations.clear();
                self.best_fitness_history.clear();
                self.average_fitness_history.clear();
                self.worst_fitness_history.clear();
                self.last_snapshot_seq = None;
            }
            AlgorithmEvent::ExecutionStateUpdated { state } => {
                if let Some(last_seq) = self.last_snapshot_seq {
                    if state.seq_id <= last_seq {
                        return;
                    }
                }

                self.last_snapshot_seq = Some(state.seq_id);
                self.generations.push(state.iteration);
                self.evaluations.push(state.evaluations);
                self.best_fitness_history.push(state.best_fitness);
                self.average_fitness_history.push(state.average_fitness);
                self.worst_fitness_history.push(state.worst_fitness);
            }
            AlgorithmEvent::End { .. } => {
                println!("  Generating charts...");
                
                if let Err(e) = self.generate_convergence_chart() {
                    eprintln!("Error generating convergence chart: {}", e);
                }
                
                if let Err(e) = self.generate_best_by_evaluations_chart() {
                    eprintln!("Error generating best-by-evaluations chart: {}", e);
                }
                
                println!(
                    "  Charts saved to: {}",
                    self.resolve_output_path().display()
                );
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        self.generate_convergence_chart().ok();
        self.generate_best_by_evaluations_chart().ok();
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solution::Solution;

    #[test]
    fn creates_structured_run_directory_on_start() {
        let base = std::env::temp_dir().join(format!(
            "rmetal_chart_observer_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = ChartObserver::new(base.clone());
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "My GA/Experiment #1".to_string(),
        });

        let run_path = observer
            .run_output_path
            .clone()
            .expect("Run output path should be configured after Start event");

        assert!(run_path.starts_with(&base));
        assert!(run_path.exists());

        let algorithm_folder = run_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .expect("Algorithm folder should exist");
        assert_eq!(algorithm_folder, "my_ga_experiment_1");
    }

    #[test]
    fn writes_chart_files_inside_run_directory() {
        let base = std::env::temp_dir().join(format!(
            "rmetal_chart_observer_files_test_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        let mut observer = ChartObserver::new(base);
        observer.update(&AlgorithmEvent::<bool>::Start {
            algorithm_name: "NSGA-II".to_string(),
        });
        observer.update(&AlgorithmEvent::<bool>::ExecutionStateUpdated {
            state: crate::algorithms::termination::ExecutionStateSnapshot::new(
                0,
                1,
                10,
                {
                    let mut solution = Solution::<bool>::new(vec![true, false]);
                    solution.set_quality(1.0);
                    solution
                },
                1.0,
                0.8,
                0.5,
            ),
        });
        observer.update(&AlgorithmEvent::<bool>::End {
            total_generations: 1,
            total_evaluations: 10,
            termination_reason: None,
        });

        let run_path = observer
            .run_output_path
            .clone()
            .expect("Run output path should exist");

        assert!(run_path.join("convergence.svg").exists());
        assert!(run_path.join("best_by_evaluations.svg").exists());
    }
}
