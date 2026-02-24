use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::problem::traits::Problem;

fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(60.0)
        .add_item(10.0, 10.0)
        .add_item(20.0, 30.0)
        .add_item(25.0, 35.0)
        .build();

    let mut solution = problem.create_solution();
    problem.evaluate(&mut solution);

    println!("NSGA-II demo placeholder: fitness={}", solution.value());
}
