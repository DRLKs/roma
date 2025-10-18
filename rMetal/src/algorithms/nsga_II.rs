use super::algorithm_trait::Algorithm;

struct NsgaII {
}

impl Algorithm for NsgaII {
    fn run(&self) {
        // Implementación del algoritmo NSGA-II
        println!("Running NSGA-II algorithm");
    }

    fn validate_parameters(&self) -> bool {
        todo!()
    }
}