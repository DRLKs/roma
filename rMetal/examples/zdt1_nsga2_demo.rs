use rMetal::algorithms::implementations::nsga2::{NSGAII, NSGAIIParameters};
use rMetal::algorithms::traits::Algorithm;
use rMetal::operator::crossover_operator_implementations::sbx_crossover::SBXCrossover;
use rMetal::operator::mutation_operator_implementations::polynomial_mutation::PolynomialMutation;
use rMetal::operator::selection_operator_implementations::multi_objective_tournament_selection::MultiObjectiveTournamentSelection;
use rMetal::problem::implementations::zdt1_problem::ZDT1Problem;
use rMetal::solution_set::traits::SolutionSet;

fn main() {
    let problem = ZDT1Problem::new(30);

    let parameters = NSGAIIParameters::new(
        60,
        40,
        0.9,
        1.0 / 30.0,
        SBXCrossover::new_default(),
        PolynomialMutation::new_default(),
        MultiObjectiveTournamentSelection::new(),
    );

    let mut algorithm = NSGAII::new(parameters);
    let result = algorithm.run(&problem);

    if let Some(best) = result.get(0) {
        println!(
            "NSGA-II finished. population={}, best objectives={:?}",
            result.size(),
            best.objectives()
        );
    } else {
        println!("NSGA-II finished with empty population");
    }
}
