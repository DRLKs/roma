use crate::problem::traits::Problem;
use crate::solution::Solution;
use crate::solution::traits::ScalarQuality;
use crate::utils::random::Random;

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

impl KnapsackProblem   
{
    /// Creates a new KnapsackProblem with specified items
    pub fn with_data(capacity: f64, weights: Vec<f64>, values: Vec<f64>, random_seed: Option<u64>) -> Self {
        assert_eq!(weights.len(), values.len(), "Weights and values must have the same length");
        let number_of_items = weights.len();
        let _ = random_seed;
        KnapsackProblem {
            description: format!("Knapsack Problem with {} items and capacity {}", number_of_items, capacity),
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

    fn create_solution(&self, _rng: &mut Random) -> Solution<bool, ScalarQuality> {
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_description_test() {
        let mut knapsack_problem = KnapsackProblem::new();

        let description = "Test Problem".to_string();
        knapsack_problem.set_problem_description(description.clone());

        assert_eq!( knapsack_problem.description, description );
    }

    #[test]
    fn test_knapsack_creation_with_data() {
        let weights = vec![10.0, 20.0, 30.0];
        let values = vec![100.0, 200.0, 300.0];
        let capacity = 50.0;

        let problem = KnapsackProblem::with_data(capacity, weights, values, Some(100));

        let solution = problem.create_solution(&mut Random::new(10));
        assert_eq!(solution.num_variables(), 3);
    }

    #[test]
    fn test_knapsack_builder() {
        let problem = KnapsackBuilder::new()
            .with_capacity(50.0)
            .add_item(10.0, 100.0)
            .add_item(20.0, 200.0)
            .build();

        let solution = problem.create_solution(&mut Random::new(10));
        assert_eq!(solution.num_variables(), 2);
    }

    #[test]
    fn test_knapsack_with_builder() {
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
}