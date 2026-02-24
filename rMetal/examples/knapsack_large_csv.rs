use rMetal::problem::implementations::knapsack_problem::KnapsackBuilder;
use rMetal::problem::traits::Problem;


fn main() {
    let problem = KnapsackBuilder::new()
        .with_capacity(120.0)
        .add_item(10.0, 20.0)
        .add_item(25.0, 40.0)
        .add_item(40.0, 70.0)
        .build();

    let mut solution = problem.create_solution();
    problem.evaluate(&mut solution);

    println!("Large CSV demo placeholder: fitness={}", solution.value());
}
