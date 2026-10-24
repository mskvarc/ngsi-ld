//! The attribute paths of build problems.

use crate::model::{AttributeName, BuildError, BuildProblem, ProblemKind};
use pretty_assertions::assert_eq;

#[test]
fn bp_01_a_problem_moved_out_of_nested_attributes_is_listed_outermost_first() {
    let inner: AttributeName = "b".parse().unwrap();
    let outer: AttributeName = "a".parse().unwrap();
    let problems = BuildError::single(BuildProblem::top(ProblemKind::JsonShape)).within(&inner);
    let problems = BuildError::new(problems).within(&outer);
    let [problem] = problems.as_slice() else { panic!("{problems:?}") };
    assert_eq!(problem.within().map(ToString::to_string).collect::<Vec<_>>(), ["a", "b"]);
    assert_eq!(problem.within().next_back(), Some(&inner));
    let error = BuildError::new(problems);
    assert_eq!(
        error.to_string(),
        "1 problem(s): a: b: the json of a JsonProperty must be an object or an array of objects"
    );
}

#[test]
fn bp_01_a_problem_of_the_entity_has_no_path() {
    assert_eq!(BuildProblem::top(ProblemKind::JsonShape).within().len(), 0);
}
