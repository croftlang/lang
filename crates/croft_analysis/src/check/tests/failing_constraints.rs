//! Constraints with `or`: a parameter's constraint that, when broken, fails
//! the call with an error the caller handles, instead of being a mistake.

use super::{assert_error, errors};

/// A clinic with an `Overdose` constraint and an `Overdosed` error, and
/// `extra` added inside it.
fn clinic(extra: &str) -> String {
    format!(
        "context Clinic {{\n\
         error Overdosed\n\
         constraint Overdose(mg: Int, max: Int) = mg <= max\n\
         {extra}\n}}"
    )
}

#[test]
fn a_constraint_with_an_error_fails_the_call_instead_of_being_a_compile_error() {
    let source = clinic(
        "fn give(mg: Int where Overdose(4000) or Overdosed): Result<Int, Overdosed> = mg\n\
         fn treat(): Result<Int, Overdosed> = give(5000)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_constraint_with_an_error_can_be_written_in_the_contract() {
    let source = clinic(
        "fn give(mg: Int): Result<Int, Overdosed> {\n mg where Overdose(4000) or Overdosed\n} = mg\n\
         fn treat(): Result<Int, Overdosed> = give(5000)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn the_error_must_be_one_the_function_can_fail_with() {
    let source = clinic("fn give(mg: Int where Overdose(4000) or Overdosed): Int = mg");

    assert_error(
        &source,
        "this body cannot fail: give it a `Result` return type",
    );
}

#[test]
fn a_field_constraint_cannot_have_an_error() {
    let source = clinic("value Dose {\n mg: Int where Overdose(4000) or Overdosed\n}");

    assert_error(
        &source,
        "only a parameter's constraint can fail with an error: a field is part of a value \
         that is always made, so `Overdose(4000)` must hold",
    );
}

#[test]
fn a_result_constraint_cannot_have_an_error() {
    let source = clinic("fn give(mg: Int): Int where Overdose(4000) or Overdosed = mg");

    assert_error(
        &source,
        "only a parameter's constraint can fail with an error: a result is given back by the \
         function itself, so `Overdose(4000)` must hold",
    );
}

#[test]
fn a_loop_counter_constraint_cannot_have_an_error() {
    let source = clinic(
        "service Count(): Unit = {\n loop (let row where Range(0, 9) or Overdosed) {\n break\n }\n}",
    );

    assert_error(
        &source,
        "only a parameter's constraint can fail with an error: a loop counter only takes the \
         values its constraints allow, so `Range(0, 9)` must hold",
    );
}
