use crate::quality_indicator::traits::QualityIndicator;
use crate::quality_indicator::implementations::decimal_quality_indicator::DecimalQualityIndicator;
use crate::solutions::traits::{Solution, SolutionInfo};
use crate::utils::random::{Random, seed_from_time };

pub struct BinarySolution {
    solution_info: SolutionInfo<bool>,
    quality: DecimalQualityIndicator,
}

impl BinarySolution {

    /// Create one random binary solution
    /// # Arguments
    ///
    /// * `size` - Size of the solution vector
    /// * `seed` - Seed of the random struct.
    ///  [`None`] will give a pseudorandom seed
    ///
    pub fn random(size: usize, seed: Option<u64> ) -> Self {

        let mut rng: Random;
        if seed.is_some() {
            rng = Random::new(seed.unwrap());
        }else {
            rng = Random::new(seed_from_time());
        }
        let variables: Vec<bool> = (0..size).map(|_| rng.coin_flip()).collect();
        Self::new(SolutionInfo::new(variables))
    }

    /// Create a binary solution with all values set to zero
    pub fn zeros(size: usize) -> Self {
        let variables = vec![false; size];
        Self::new(SolutionInfo::new(variables))
    }

    /// Create a binary solution with all values set to true
    pub fn ones(size: usize) -> Self {
        let variables = vec![true; size];
        Self::new(SolutionInfo::new(variables))
    }
    
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

impl Eq for BinarySolution {}

impl PartialEq<Self> for BinarySolution {
    fn eq(&self, other: &Self) -> bool {
        if self.solution_info.get_variables().len() != other.solution_info.get_variables().len() {
            false
        }else if self.count_ones() != other.count_ones() {
            false
        }else if self.count_zeros() != other.count_zeros() {
            false
        }else{
            
            for value1 in self.solution_info.get_variables() {
                
                for value2 in other.solution_info.get_variables() {
                    
                    if value1 != value2 {
                        return false;
                    }
                        
                }
            }
            
            return true;            
        }
        
    }
}

impl Solution<bool> for BinarySolution {
    type Quality = DecimalQualityIndicator;

    fn new(solution_info: SolutionInfo<bool>) -> Self {
        BinarySolution {
            solution_info,
            quality: DecimalQualityIndicator::new(None),
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

    fn value(&self) -> f64 {
        self.quality.get_fitness().unwrap_or(0.0)
    }

    fn is_valid(&self) -> bool {
        // Quizás esta función no tiene sentido
        true
    }
}

impl Clone for BinarySolution {
    fn clone(&self) -> Self {
        Self {
            solution_info: self.solution_info.clone(),
            quality: self.quality.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solutions::traits::{Solution};

    #[test]
    fn test_compare_binary_solutions() {
        let mut binary_solution1: BinarySolution = BinarySolution::random(10, None);
        let mut binary_solution2: BinarySolution = BinarySolution::random(10, None);

        let quality1 = DecimalQualityIndicator::new(Option::from(19.0));
        let quality2 = DecimalQualityIndicator::new(Option::from(18.0));
        
        binary_solution1.set_quality(quality1);
        binary_solution2.set_quality(quality2);

        let result: bool = binary_solution1.dominates(&binary_solution2);

        assert!(result);
    }

    #[test]
    fn test_ones_binary_solutions() {
        let size: usize = 10;
        let ones_binary_solution: BinarySolution = BinarySolution::ones(size);

        let mut ok = true;
        let mut ctt = 0;
        while ok && ctt < size {
            ok = *ones_binary_solution.get_variable(ctt).unwrap();
            ctt += 1;
        }
        assert!(ok);
    }

    #[test]
    fn test_zeros_binary_solutions() {
        let size: usize = 10;
        let zeros_binary_solution: BinarySolution = BinarySolution::zeros(size);

        let mut ok = true;
        let mut ctt = 0;
        while ok && ctt < size {
            ok = !*zeros_binary_solution.get_variable(ctt).unwrap();
            ctt += 1;
        }
        assert!(ok);
    }
}
