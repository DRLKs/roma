use crate::observer::traits::{AlgorithmEvent, AlgorithmObserver};
use crate::solutions::traits::Solution;
use crate::utils::chart::{ChartBuilder, Series};
use std::path::PathBuf;

/// Observer that generates charts showing algorithm progress
pub struct ChartObserver {
    name: String,
    output_path: PathBuf,
    
    // Data collection
    generations: Vec<usize>,
    evaluations: Vec<usize>,
    best_fitness_history: Vec<f64>,
    average_fitness_history: Vec<f64>,
    worst_fitness_history: Vec<f64>,
    
    // Configuration
    chart_width: u32,
    chart_height: u32,
}

impl ChartObserver {
    /// Creates a new ChartObserver
    /// 
    /// # Arguments
    /// * `output_path` - Directory where charts will be saved
    pub fn new(output_path: PathBuf) -> Self {
        ChartObserver {
            name: "ChartObserver".to_string(),
            output_path,
            generations: Vec::new(),
            evaluations: Vec::new(),
            best_fitness_history: Vec::new(),
            average_fitness_history: Vec::new(),
            worst_fitness_history: Vec::new(),
            chart_width: 1200,
            chart_height: 800,
        }
    }

    /// Sets the chart dimensions
    pub fn with_dimensions(mut self, width: u32, height: u32) -> Self {
        self.chart_width = width;
        self.chart_height = height;
        self
    }

    /// Generates a convergence chart showing fitness evolution over generations
    fn generate_convergence_chart(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.generations.is_empty() {
            return Ok(());
        }

        let output_file = self.output_path.join("convergence.svg");

        // Preparar datos como (f64, f64)
        let best_data: Vec<(f64, f64)> = self.generations.iter()
            .zip(self.best_fitness_history.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        let avg_data: Vec<(f64, f64)> = self.generations.iter()
            .zip(self.average_fitness_history.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        let worst_data: Vec<(f64, f64)> = self.generations.iter()
            .zip(self.worst_fitness_history.iter())
            .map(|(generation, fitness)| (*generation as f64, *fitness))
            .collect();

        // Crear series
        let best_series = Series::new("Best", best_data).with_color("#2563eb");
        let avg_series = Series::new("Average", avg_data).with_color("#10b981");
        let worst_series = Series::new("Worst", worst_data).with_color("#dc2626");

        // Crear y guardar gráfico
        let chart = ChartBuilder::new()
            .title("Convergence")
            .x_label("Generation")
            .y_label("Fitness")
            .size(self.chart_width, self.chart_height)
            .add_series(best_series)
            .add_series(avg_series)
            .add_series(worst_series)
            .build();

        chart.save(output_file)?;

        Ok(())
    }

    /// Generates a chart showing evaluations over time
    fn generate_evaluations_chart(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.generations.is_empty() {
            return Ok(());
        }

        let output_file = self.output_path.join("evaluations.svg");

        // Preparar datos
        let data: Vec<(f64, f64)> = self.generations.iter()
            .zip(self.evaluations.iter())
            .map(|(generation, evals)| (*generation as f64, *evals as f64))
            .collect();

        let series = Series::new("Evaluations", data).with_color("#2563eb");

        // Crear y guardar gráfico
        let chart = ChartBuilder::new()
            .title("Total Evaluations Over Time")
            .x_label("Generation")
            .y_label("Total Evaluations")
            .size(self.chart_width, self.chart_height)
            .add_series(series)
            .build();

        chart.save(output_file)?;

        Ok(())
    }
}

impl<T, S> AlgorithmObserver<T, S> for ChartObserver
where
    S: Solution<T>,
    T: Clone,
{
    fn update(&mut self, event: &AlgorithmEvent<T, S>) {
        match event {
            AlgorithmEvent::Start { algorithm_name } => {
                println!("  ChartObserver: Monitoring algorithm '{}'", algorithm_name);
                println!("   Charts will be saved to: {:?}", self.output_path);
                
                // Create output directory if it doesn't exist
                std::fs::create_dir_all(&self.output_path).ok();
                
                // Clear previous data
                self.generations.clear();
                self.evaluations.clear();
                self.best_fitness_history.clear();
                self.average_fitness_history.clear();
                self.worst_fitness_history.clear();
            }
            AlgorithmEvent::GenerationCompleted {
                generation,
                evaluations,
                best_fitness,
                average_fitness,
                worst_fitness,
            } => {
                self.generations.push(*generation);
                self.evaluations.push(*evaluations);
                self.best_fitness_history.push(*best_fitness);
                self.average_fitness_history.push(*average_fitness);
                self.worst_fitness_history.push(*worst_fitness);
            }
            AlgorithmEvent::End { .. } => {
                // Generate all charts
                println!("  Generating charts...");
                
                if let Err(e) = self.generate_convergence_chart() {
                    eprintln!("Error generating convergence chart: {}", e);
                }
                
                if let Err(e) = self.generate_evaluations_chart() {
                    eprintln!("Error generating evaluations chart: {}", e);
                }
                
                println!("  Charts saved to: {:?}", self.output_path);
            }
            AlgorithmEvent::Error { message } => {
                eprintln!("  ChartObserver: Error detected - {}", message);
                eprintln!("   No charts will be generated due to early termination.");
            }
            _ => {}
        }
    }

    fn finalize(&mut self) {
        // Final chart generation if not already done
        self.generate_convergence_chart().ok();
        self.generate_evaluations_chart().ok();
    }

    fn name(&self) -> &str {
        &self.name
    }
}
