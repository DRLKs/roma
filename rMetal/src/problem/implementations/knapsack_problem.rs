use crate::problem::problem_trait::Problem;
use crate::solutions::implementations::binary_solution::BinarySolution;
use crate::solutions::solution_trait::Solution;
use crate::utils::random::Random;

/// Knapsack Problem: maximize the value of items in a knapsack without exceeding capacity
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
    pub fn with_data(capacity: f64, weights: Vec<f64>, values: Vec<f64>) -> Self {
        assert_eq!(weights.len(), values.len(), "Weights and values must have the same length");
        let number_of_items = weights.len();
        
        KnapsackProblem {
            description: format!("Knapsack Problem with {} items and capacity {}", number_of_items, capacity),
            number_of_items,
            capacity,
            weights,
            values,
        }
    }

    /// Calculates the total weight of selected items
    fn calculate_weight(&self, solution: &BinarySolution) -> f64 {
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
    fn calculate_value(&self, solution: &BinarySolution) -> f64 {
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

impl Problem<BinarySolution, bool> for KnapsackProblem {
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

    fn evaluate(&self, solution: &mut BinarySolution) {
        let weight = self.calculate_weight(solution);
        let value = self.calculate_value(solution);
        
        // If weight exceeds capacity, apply penalty
        let _fitness = if weight > self.capacity {
            -(weight - self.capacity) * 1000.0 // Heavy penalty for infeasible solutions
        } else {
            value // Maximize value
        };
        
        // Set the quality indicator (you'll need to implement this based on your Quality type)
        // For now, this is a placeholder - adjust based on your BinarySolution quality implementation
        // solution.set_quality(...);
    }

    fn set_problem_description(&mut self, description: String) {
        self.description = description;
    }

    fn get_problem_description(&self) -> String {
        self.description.clone()
    }

    fn create_solution(&self) -> BinarySolution {
        BinarySolution::random(self.number_of_items, None)
    }

    fn neighbor(&self, solution: &BinarySolution) -> BinarySolution {
        let mut neighbor = solution.copy();
        let mut rng = Random::new(crate::utils::random::seed_from_time());
        
        // Flip one random bit
        let index = rng.range(self.number_of_items as u64) as usize;
        if let Some(value) = neighbor.get_variable(index) {
            let _ = neighbor.set_variable(index, !value);
        }
        
        neighbor
    }
}