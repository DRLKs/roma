use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::utils::random::Random;
use std::collections::HashMap;

const PENALTY: f64 = 0.5; // Heavy penalty for infeasible solutions

/// Knapsack Problem: maximize the value of items in a knapsack without exceeding capacity
#[derive(Clone)]
pub struct KnapsackProblem {
    description: String,
    number_of_items: usize,
    capacity: f64,
    weights: Vec<f64>,
    values: Vec<f64>,
}

impl KnapsackProblem {
    /// Creates a new KnapsackProblem with specified items
    pub fn with_data(
        capacity: f64,
        weights: Vec<f64>,
        values: Vec<f64>,
        random_seed: Option<u64>,
    ) -> Self {
        assert_eq!(
            weights.len(),
            values.len(),
            "Weights and values must have the same length"
        );
        let number_of_items = weights.len();
        let _ = random_seed;
        KnapsackProblem {
            description: format!(
                "Knapsack Problem with {} items and capacity {}",
                number_of_items, capacity
            ),
            number_of_items,
            capacity,
            weights,
            values,
        }
    }

    /// Calculates the total weight of selected items
    fn calculate_weight(&self, solution: &Solution<bool>) -> f64 {
        let mut total_weight = 0.0;
        for i in 0..self.number_of_items {
            if let Some(&selected) = solution.get_variable(i) {
                if selected {
                    total_weight += self.weights[i];
                }
            }
        }
        total_weight
    }

    /// Calculates the total value of selected items
    fn calculate_value(&self, solution: &Solution<bool>) -> f64 {
        let mut total_value = 0.0;
        for i in 0..self.number_of_items {
            if let Some(&selected) = solution.get_variable(i) {
                if selected {
                    total_value += self.values[i];
                }
            }
        }
        total_value
    }
}

impl Problem<bool> for KnapsackProblem {
    fn new() -> Self {
        // Default constructor with empty problem
        KnapsackProblem {
            description: "Knapsack Problem".to_string(),
            number_of_items: 0,
            capacity: 0.0,
            weights: vec![],
            values: vec![],
        }
    }

    fn evaluate(&self, solution: &mut Solution<bool>) {
        let weight = self.calculate_weight(solution);
        let value = self.calculate_value(solution);

        // If weight exceeds capacity, apply penalty
        let _fitness = if weight > self.capacity {
            -(weight - self.capacity) * PENALTY // Penalty for infeasible solutions
        } else {
            value // Maximize value_fitness
        };

        solution.set_quality(_fitness);
    }

    fn dominates(&self, solution_a: &Solution<bool, f64>, solution_b: &Solution<bool, f64>) -> bool {
        let fitness_a = solution_a.quality().copied().unwrap_or(f64::NEG_INFINITY);
        let fitness_b = solution_b.quality().copied().unwrap_or(f64::NEG_INFINITY);
        fitness_a > fitness_b
    }

    fn create_solution(&self, _rng: &mut Random) -> Solution<bool> {
        let mut variables: Vec<bool> = vec![];
        for _ in 0..self.number_of_items {
            variables.push(_rng.coin_flip());
        }
        Solution::new(variables)
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn better_fitness_fn(&self) -> fn(f64, f64) -> bool {
        crate::solution::traits::evaluator::maximizing_fitness
    }

    fn format_solution(&self, solution: &Solution<bool>) -> String {
        let selected_items = solution
            .variables()
            .iter()
            .filter(|&&selected| selected)
            .count();
        let total_weight = self.calculate_weight(solution);
        let total_value = self.calculate_value(solution);
        let feasible = total_weight <= self.capacity;
        let quality_text = solution
            .try_quality_value()
            .map(|value| format!("{:.3}", value))
            .unwrap_or_else(|| "not evaluated".to_string());

        format!(
            "selected={}/{}, weight={:.3}/{:.3}, value={:.3}, feasible={}, quality={}",
            selected_items,
            self.number_of_items,
            total_weight,
            self.capacity,
            total_value,
            feasible,
            quality_text
        )
    }

    fn get_problem_parameters_payload(&self) -> String {
        format!(
            "capacity={:.3}, items=[{}]",
            self.capacity,
            self.weights
                .iter()
                .zip(self.values.iter())
                .map(|(w, v)| format!("(weight={:.3}, value={:.3})", w, v))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

pub struct KnapsackBuilder {
    capacity: f64,
    weights: Vec<f64>,
    values: Vec<f64>,
}

impl KnapsackBuilder {
    pub fn new() -> Self {
        Self {
            capacity: 100.0,
            weights: vec![],
            values: vec![],
        }
    }

    pub fn with_capacity(mut self, capacity: f64) -> Self {
        self.capacity = capacity;
        self
    }

    pub fn add_item(mut self, weight: f64, value: f64) -> Self {
        self.weights.push(weight);
        self.values.push(value);
        self
    }

    pub fn add_items(mut self, items: Vec<(f64, f64)>) -> Self {
        for (weight, value) in items {
            self.weights.push(weight);
            self.values.push(value);
        }
        self
    }

    pub fn build(self) -> KnapsackProblem {
        KnapsackProblem::with_data(self.capacity, self.weights, self.values, None)
    }
}

impl Default for KnapsackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds a `KnapsackProblem` from generic record maps.
///
/// This function is intended for data-ingestion flows where CSV/JSON/YAML adapters return
/// `Vec<HashMap<String, String>>`. Callers provide the key mapping in code (`weight_key`,
/// `value_key`) and this function converts valid rows into knapsack items.
///
/// Returns the built problem and the number of successfully loaded items.
pub fn build_knapsack_from_records(
    records: &[HashMap<String, String>],
    capacity: f64,
    row_limit: usize,
    weight_key: &str,
    value_key: &str,
) -> Result<(KnapsackProblem, usize), String> {
    if records.is_empty() {
        return Err("Input data has no records".to_string());
    }

    let mut builder = KnapsackBuilder::new().with_capacity(capacity);
    let mut loaded_items = 0usize;

    for record in records.iter().take(row_limit) {
        let Some(weight_text) = record.get(weight_key) else {
            continue;
        };
        let Some(value_text) = record.get(value_key) else {
            continue;
        };

        let Ok(weight) = weight_text.parse::<f64>() else {
            continue;
        };
        let Ok(value) = value_text.parse::<f64>() else {
            continue;
        };

        builder = builder.add_item(weight, value);
        loaded_items += 1;
    }

    if loaded_items == 0 {
        return Err(format!(
            "No valid records found. Ensure keys '{}' and '{}' exist and are numeric",
            weight_key, value_key
        ));
    }

    Ok((builder.build(), loaded_items))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_problem_description() {
        let mut knapsack_problem = KnapsackProblem::new();

        let description = "Test Problem".to_string();
        knapsack_problem.set_problem_description(description.clone());

        assert_eq!(knapsack_problem.description, description);
    }

    #[test]
    fn with_data_creates_solution_with_expected_variable_count() {
        let weights = vec![10.0, 20.0, 30.0];
        let values = vec![100.0, 200.0, 300.0];
        let capacity = 50.0;

        let problem = KnapsackProblem::with_data(capacity, weights, values, Some(100));

        let solution = problem.create_solution(&mut Random::new(10));
        assert_eq!(solution.num_variables(), 3);
    }

    #[test]
    fn builder_creates_solution_with_expected_variable_count() {
        let problem = KnapsackBuilder::new()
            .with_capacity(50.0)
            .add_item(10.0, 100.0)
            .add_item(20.0, 200.0)
            .build();

        let solution = problem.create_solution(&mut Random::new(10));
        assert_eq!(solution.num_variables(), 2);
    }

    #[test]
    fn evaluate_assigns_positive_quality_for_profitable_feasible_selection() {
        let problem = KnapsackBuilder::new()
            .with_capacity(100.0)
            .add_item(10.0, 50.0)
            .add_item(20.0, 100.0)
            .add_item(30.0, 150.0)
            .build();

        let sol = problem.create_solution(&mut Random::new(10));
        assert_eq!(sol.num_variables(), 3);

        let mut variables: Vec<bool> = vec![];
        for _ in 0..3 {
            variables.push(true);
        }
        let mut solution = Solution::new(variables);

        problem.evaluate(&mut solution);

        assert!(solution.quality().copied().unwrap() > 0.0);
    }

    #[test]
    fn knapsack_uses_maximizing_fitness() {
        let problem = KnapsackBuilder::new().build();
        assert!(problem.is_better_fitness(10.0, 5.0));
        assert!(!problem.is_better_fitness(5.0, 10.0));
    }

    #[test]
    fn format_solution_reports_domain_summary() {
        let problem = KnapsackBuilder::new()
            .with_capacity(50.0)
            .add_item(10.0, 100.0)
            .add_item(20.0, 200.0)
            .add_item(30.0, 300.0)
            .build();

        let mut solution = Solution::new(vec![true, false, true]);
        problem.evaluate(&mut solution);

        let formatted = problem.format_solution(&solution);

        assert!(formatted.contains("selected=2/3"));
        assert!(formatted.contains("weight=40.000/50.000"));
        assert!(formatted.contains("value=400.000"));
        assert!(formatted.contains("feasible=true"));
        assert!(formatted.contains("quality=400.000"));
    }
}
