use crate::quality_indicator::quality_indicator_trait::QualityIndicator;
use crate::quality_indicator::decimal_quality_indicator::DecimalQualityIndicator;
use crate::solutions::solution_trait::{Solution, SolutionInfo, SolutionBuilder};

pub struct BinarySolution {
    solution_info: SolutionInfo<bool>,
    quality: DecimalQualityIndicator,
}

impl BinarySolution {
    /// Flip a bit at the given index
    pub fn flip_bit(&mut self, index: usize) -> Result<(), String> {
        if let Some(bit) = self.solution_info.get_variables_mut().get_mut(index) {
            *bit = !*bit;
            self.quality.invalidate();
            Ok(())
        } else {
            Err(format!("Index {} out of bounds", index))
        }
    }
    
    /// Get the number of true bits
    pub fn count_ones(&self) -> usize {
        self.solution_info.get_variables().iter().filter(|&&x| x).count()
    }
    
    /// Get the number of false bits
    pub fn count_zeros(&self) -> usize {
        self.get_number_of_variables() - self.count_ones()
    }
}

impl Solution<bool> for BinarySolution {
    type Quality = DecimalQualityIndicator;

    fn new(solution_info: SolutionInfo<bool>) -> Self {
        BinarySolution {
            solution_info,
            quality: DecimalQualityIndicator::new(),
        }
    }
    
    fn get_solution_info(&self) -> &SolutionInfo<bool> {
        &self.solution_info
    }
    
    fn get_solution_info_mut(&mut self) -> &mut SolutionInfo<bool> {
        &mut self.solution_info
    }

    fn get_quality(&self) -> Option<&DecimalQualityIndicator> {
        Some(&self.quality)
    }

    fn set_quality(&mut self, quality: DecimalQualityIndicator) {
        self.quality = quality;
    }

    fn is_valid(&self) -> bool {
        // Las soluciones binarias siempre son válidas
        true
    }
}

// Builder para soluciones binarias
pub struct BinarySolutionBuilder;

impl SolutionBuilder<bool> for BinarySolutionBuilder {
    type Solution = BinarySolution;
    
    fn build(solution_info: SolutionInfo<bool>) -> Self::Solution {
        BinarySolution::new(solution_info)
    }
}

// Implementación específica de operaciones para soluciones binarias
impl BinarySolution {
    /// Crear una solución binaria aleatoria
    pub fn random(size: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let variables: Vec<bool> = (0..size).map(|_| rng.gen_bool(0.5)).collect();
        Self::new(SolutionInfo::new(variables))
    }
    
    /// Crear una solución binaria con todos los bits en false
    pub fn zeros(size: usize) -> Self {
        let variables = vec![false; size];
        Self::new(SolutionInfo::new(variables))
    }
    
    /// Crear una solución binaria con todos los bits en true
    pub fn ones(size: usize) -> Self {
        let variables = vec![true; size];
        Self::new(SolutionInfo::new(variables))
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use super::*;
    use crate::solutions::solution_trait::{Solution};

    #[test]
    fn test_compare_binary_solutions() {
        let mut binarySolution1: BinarySolution = BinarySolution::random(10);
        let mut binarySolution2: BinarySolution = BinarySolution::random(10);

        let mut quality1 = DecimalQualityIndicator::new();
        let mut quality2 = DecimalQualityIndicator::new();

        quality1.set_fitness_indicator(Some(19.0));
        binarySolution1.set_quality(quality1);

        quality2.set_fitness_indicator(Some(18.0));
        binarySolution2.set_quality(quality2);

        let result: bool = binarySolution1.dominates(&binarySolution2);

        assert!(result);
    }
}
