use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::problem::traits::Problem;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(50.0)
        .add_item(10.0, 60.0)
        .add_item(20.0, 100.0)
        .add_item(30.0, 120.0)
        .build();

    let mut solution = problem.create_solution();
    problem.evaluate(&mut solution);

    println!("Internal demo placeholder: fitness={}", solution.value());
}
