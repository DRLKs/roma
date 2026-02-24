use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::problem::traits::Problem;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(90.0)
        .add_item(12.0, 24.0)
        .add_item(22.0, 33.0)
        .add_item(41.0, 80.0)
        .build();

    let mut solution = problem.create_solution();
    problem.evaluate(&mut solution);

    println!("Hill-climbing observer demo placeholder: fitness={}", solution.value());
}
