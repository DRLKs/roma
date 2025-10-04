use crate::solutions::solution_trait::{Solution, SolutionInfo, SolutionBuilder};

#[derive(Clone, Debug)]
pub struct PermutationSolution<T: Clone> {
    solution_info: SolutionInfo<T>,
    fitness: Option<f64>,
    length: usize,
}

impl<T: Clone> PermutationSolution<T> {
    pub fn get_length(&self) -> usize {
        self.length
    }
    
    /// Intercambiar dos elementos en la permutación
    pub fn swap(&mut self, i: usize, j: usize) -> Result<(), String> {
        if i >= self.length || j >= self.length {
            return Err("Index out of bounds".to_string());
        }
        
        let variables = self.solution_info.get_variables_mut();
        variables.swap(i, j);
        self.fitness = None; // Invalidar fitness después de la mutación
        Ok(())
    }
}

impl<T: Clone + std::cmp::PartialEq> Solution<T> for PermutationSolution<T> {
    type Fitness = f64;
    
    fn new(solution_info: SolutionInfo<T>) -> Self {
        let length = solution_info.get_variables().len();
        PermutationSolution {
            solution_info,
            fitness: None,
            length,
        }
    }
    
    fn get_solution_info(&self) -> &SolutionInfo<T> {
        &self.solution_info
    }
    
    fn get_solution_info_mut(&mut self) -> &mut SolutionInfo<T> {
        &mut self.solution_info
    }
    
    fn get_fitness(&self) -> Option<&Self::Fitness> {
        self.fitness.as_ref()
    }

    fn set_fitness(&mut self, fitness: Self::Fitness) {
        self.fitness = Some(fitness);
    }

    fn is_valid(&self) -> bool {
        let variables = self.solution_info.get_variables();
        
        // Verificar longitud primero
        if variables.len() != self.length {
            return false;
        }
        
        // Verificar que no hay duplicados comparando cada elemento con todos los demás
        for i in 0..variables.len() {
            for j in (i + 1)..variables.len() {
                if variables[i] == variables[j] {
                    return false; // Encontramos un duplicado
                }
            }
        }
        
        true
    }
}

// Implementación del builder pattern
pub struct PermutationSolutionBuilder;

impl<T: Clone + std::cmp::PartialEq> SolutionBuilder<T> for PermutationSolutionBuilder {
    type Solution = PermutationSolution<T>;
    
    fn build(solution_info: SolutionInfo<T>) -> Self::Solution {
        PermutationSolution::new(solution_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solutions::solution_trait::{Solution, SolutionInfo};

    #[test]
    fn test_is_valid_with_valid_permutation() {
        // Test con permutación válida de enteros
        let valid_variables = vec![0, 1, 2, 3];
        let solution_info = SolutionInfo::new(valid_variables);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Una permutación sin duplicados debería ser válida");
    }

    #[test]
    fn test_is_valid_with_duplicates() {
        // Test con duplicados
        let invalid_variables = vec![0, 1, 2, 1]; // 1 está duplicado
        let solution_info = SolutionInfo::new(invalid_variables);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(!solution.is_valid(), "Una permutación con duplicados debería ser inválida");
    }

    #[test]
    fn test_is_valid_with_wrong_length() {
        // Test manipulando la longitud internamente
        let variables = vec![0, 1, 2];
        let solution_info = SolutionInfo::new(variables);
        let mut solution = PermutationSolution::new(solution_info);
        
        // Modificar variables para que no coincida con la longitud esperada
        solution.get_solution_info_mut().get_variables_mut().push(3);
        
        assert!(!solution.is_valid(), "Longitud incorrecta debería hacer la solución inválida");
    }

    #[test]
    fn test_is_valid_empty_permutation() {
        // Test con permutación vacía
        let empty_variables: Vec<i32> = vec![];
        let solution_info = SolutionInfo::new(empty_variables);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Una permutación vacía debería ser válida");
    }

    #[test]
    fn test_is_valid_single_element() {
        // Test con un solo elemento
        let single_variable = vec![42];
        let solution_info = SolutionInfo::new(single_variable);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Una permutación de un elemento debería ser válida");
    }

    #[test]
    fn test_is_valid_with_strings() {
        // Test con strings para probar el genérico
        let string_variables = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let solution_info = SolutionInfo::new(string_variables);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Una permutación de strings sin duplicados debería ser válida");
        
        // Test con strings duplicados
        let duplicate_strings = vec!["a".to_string(), "b".to_string(), "a".to_string()];
        let solution_info = SolutionInfo::new(duplicate_strings);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(!solution.is_valid(), "Una permutación de strings con duplicados debería ser inválida");
    }

    #[test]
    fn test_is_valid_after_swap() {
        // Test para verificar que is_valid funciona después de hacer swap
        let variables = vec![0, 1, 2, 3];
        let solution_info = SolutionInfo::new(variables);
        let mut solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Debería ser válida inicialmente");
        
        // Hacer swap válido
        solution.swap(0, 1).unwrap();
        assert!(solution.is_valid(), "Debería seguir siendo válida después del swap");
    }

    #[test]
    fn test_is_valid_large_permutation() {
        // Test con permutación más grande
        let large_variables: Vec<usize> = (0..100).collect();
        let solution_info = SolutionInfo::new(large_variables);
        let solution = PermutationSolution::new(solution_info);
        
        assert!(solution.is_valid(), "Una permutación grande sin duplicados debería ser válida");
    }
}