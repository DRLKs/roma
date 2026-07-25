use roma_lib::problem::{Problem, RastriginProblem, SolutionComparison, ZDT1Problem};
use roma_lib::{AckleyProblem, ParetoCrowdingDistanceQuality, Solution};

fn scalar_solution(value: f64) -> Solution<f64> {
    let mut solution = Solution::new(vec![0.0]);
    solution.set_quality(value);
    solution
}

fn pareto_solution(objectives: Vec<f64>) -> Solution<f64, ParetoCrowdingDistanceQuality> {
    let mut solution = Solution::new(vec![0.0]);
    solution.set_quality(ParetoCrowdingDistanceQuality {
        objectives,
        rank: None,
        crowding_distance: None,
    });
    solution
}

#[test]
fn scalar_dominance_and_scalar_ordering_share_one_preference_rule() {
    let problem = RastriginProblem::new_default();
    let near_zero = scalar_solution(-1.0);
    let farther = scalar_solution(2.0);

    assert_eq!(
        problem.compare_solutions(&near_zero, &farther),
        SolutionComparison::Better
    );
    assert!(problem.dominates(&near_zero, &farther));
    assert!(problem.is_better_fitness(-1.0, 2.0));
    assert_eq!(
        problem
            .compare_solutions(&near_zero, &farther)
            .scalar_ordering(),
        std::cmp::Ordering::Less
    );
}

#[test]
fn scalar_comparison_handles_unevaluated_candidates_consistently() {
    let problem = AckleyProblem::new_default();
    let evaluated = scalar_solution(1.0);
    let unevaluated = Solution::new(vec![0.0]);

    assert_eq!(
        problem.compare_solutions(&evaluated, &unevaluated),
        SolutionComparison::Better
    );
    assert!(problem.dominates(&evaluated, &unevaluated));
}

#[test]
fn pareto_incomparability_remains_distinct_from_equivalence() {
    let problem = ZDT1Problem::new_default();
    let left = pareto_solution(vec![0.2, 0.8]);
    let right = pareto_solution(vec![0.8, 0.2]);
    let equal = pareto_solution(vec![0.2, 0.8]);

    assert_eq!(
        problem.compare_solutions(&left, &right),
        SolutionComparison::Incomparable
    );
    assert!(!problem.dominates(&left, &right));
    assert_eq!(
        problem.compare_solutions(&left, &equal),
        SolutionComparison::Equivalent
    );
}
