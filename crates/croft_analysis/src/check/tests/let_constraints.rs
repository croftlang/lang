//! Constraints on a `let`, `let items: List<Int> where Length(0, 200) = []`,
//! and the `Length` a list or map needs when `set` changes it.

use super::{assert_error, errors};

/// A context with a service whose body is `body`.
fn service(body: &str) -> String {
    format!(
        "context Counting {{\n\
         use {{ Map }} from Std.Collections\n\
         error Missing\n\
         service Count(extra: Int): Int = {{\n\
         {body}\n\
         }}\n}}"
    )
}

#[test]
fn a_list_that_set_changes_needs_a_length() {
    let body = "let items: List<Int> = []\n\
                loop (let i where Range(0, 199)) {\n set items = items.add(i)\n }\n\
                items.length";

    assert_error(
        &service(body),
        "`items` is changed with `set`, so say how many items it can hold with a `Length`, \
         such as `let items: List<Int> where Length(0, 100) = ...`",
    );
}

#[test]
fn a_list_without_a_written_type_needs_a_length_too() {
    let body = "let items = [1]\nset items = items.add(extra)\nitems.length";

    assert_error(
        &service(body),
        "`items` is changed with `set`, so say how many items it can hold with a `Length`, \
         such as `let items: List<Int> where Length(0, 100) = ...`",
    );
}

#[test]
fn a_map_that_set_changes_needs_a_length() {
    let body =
        "let counts: Map<Int, Int> = [1: 1]\nset counts = counts.set(extra, 1)\ncounts.length";

    assert_error(
        &service(body),
        "`counts` is changed with `set`, so say how many entries it can hold with a `Length`, \
         such as `let counts: Map<Int, Int> where Length(0, 100) = ...`",
    );
}

#[test]
fn a_list_with_a_length_or_never_set_is_accepted() {
    let body = "let items: List<Int> where Length(0, 200) = []\n\
                loop (let i where Range(0, 199)) {\n set items = items.add(i)\n }\n\
                let fixed = [1, 2]\n\
                items.length + fixed.length";

    assert_eq!(errors(&service(body)), Vec::<String>::new());
}

#[test]
fn a_first_value_that_breaks_a_constraint_is_an_error() {
    let body =
        "let items: List<Int> where Length(1, 3) = []\nset items = items.add(1)\nitems.length";

    assert_error(
        &service(body),
        "`items` starts as `[]`, which breaks `Length(1, 3)`",
    );
}

#[test]
fn a_known_value_set_that_breaks_a_constraint_is_an_error() {
    let body =
        "let items: List<Int> where Length(0, 3) = []\nset items = [1, 2, 3, 4]\nitems.length";

    assert_error(
        &service(body),
        "`items` is set to `[1, 2, 3, 4]`, which breaks `Length(0, 3)`",
    );
}

#[test]
fn a_range_on_a_number_is_checked_too() {
    let body = "let left: Int where Range(0, 9) = 12\nleft";

    assert_error(
        &service(body),
        "`left` starts as `12`, which breaks `Range(0, 9)`",
    );
}

#[test]
fn a_loop_that_adds_more_than_the_length_allows_is_an_error() {
    let body = "let items: List<Int> where Length(0, 100) = [7]\n\
                loop (let i where Range(1, 100)) {\n set items = items.add(i)\n }\n\
                items.length";

    assert_error(
        &service(body),
        "this loop runs 100 times and adds to `items` each time, so it ends up with at least \
         101 items, more than `Length(0, 100)` allows",
    );
}

#[test]
fn a_loop_that_can_stop_early_is_left_to_the_program() {
    let guarded = "let items: List<Int> where Length(0, 10) = []\n\
                   loop (let i where Range(1, 100)) {\n if i > extra {\n break\n }\n set items = items.add(i)\n }\n\
                   items.length";

    assert_eq!(errors(&service(guarded)), Vec::<String>::new());

    // Emptied somewhere else, so how long it gets depends on the order.
    let emptied = "let items: List<Int> where Length(0, 10) = []\n\
                   loop (let i where Range(1, 100)) {\n set items = items.add(i)\n }\n\
                   set items = []\n\
                   items.length";

    assert_eq!(errors(&service(emptied)), Vec::<String>::new());
}

#[test]
fn a_let_constraint_cannot_have_an_error() {
    let body = "let items: List<Int> where Length(0, 3) or Missing = []\nitems.length";

    assert_error(
        &service(body),
        "only a parameter's constraint can fail with an error: a `let` name has nowhere to \
         fail, so `Length(0, 3)` must hold",
    );
}

#[test]
fn a_let_length_must_be_known_when_compiling() {
    let body =
        "let items: List<Int> where Length(0, extra) = []\nset items = items.add(1)\nitems.length";

    assert_error(
        &service(body),
        "the `Length` of `items` must be known when the program is compiled, since it says \
         how much room to keep: use a number or a constant",
    );
}

#[test]
fn each_on_a_let_needs_a_list() {
    let body = "let left: Int where each Range(0, 9) = 1\nleft";

    assert_error(&service(body), "`each` can only be used on a list");
}
