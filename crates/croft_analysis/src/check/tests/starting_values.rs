//! Starting values: what a field holds when a record, event, error or
//! adapter leaves it out.

use super::{assert_error, dungeon, errors};

#[test]
fn leaves_out_a_field_with_a_starting_value() {
    let source = dungeon(
        "value Speed { x: Int  y: Int = 0 }\n\
         fn walk(): Speed = Speed { x: 1 }\n\
         fn climb(): Speed = Speed { x: 0, y: 1 }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn still_needs_the_fields_without_one() {
    let source = dungeon(
        "value Speed { x: Int  y: Int = 0 }\n\
         fn stand(): Speed = Speed { y: 1 }",
    );

    assert_error(&source, "`Speed` needs a value for `x`");
}

#[test]
fn checks_a_starting_value_against_its_type() {
    let source = dungeon("value Speed { x: Int = \"fast\" }");
    assert_error(&source, "expected `Int`, found `String`");
}

#[test]
fn reports_a_starting_value_that_breaks_its_constraints() {
    let source = dungeon("value Lives { count: Int where Range(0, 9) = 12 }");

    assert_error(
        &source,
        "`Lives` starts with `12` for `count`, which breaks `Range(0, 9)`",
    );
}

#[test]
fn reads_constants_but_not_other_fields() {
    let source = dungeon(
        "const MAX_LIVES: Int = 3\n\
         value Lives { count: Int = MAX_LIVES }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());

    let source = dungeon("value Lives { most: Int  count: Int = most }");
    assert_error(&source, "cannot find `most` here");
}

#[test]
fn gives_aggregates_and_entities_starting_values() {
    let source = dungeon(
        "aggregate Room {\n\
             id: RoomId\n\
             lives: Int where Range(0, 9) = 3\n\
             visited: Bool = false\n\
             factory Open(id: RoomId) = Room { id }\n\
         }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn gives_events_and_errors_starting_values() {
    let source = dungeon(
        "error Blocked { reason: String = \"a wall\" }\n\
         fn blocked(): Result<Unit, Blocked> = Err(Blocked {})\n\
         aggregate Room {\n\
             id: RoomId\n\
             event Opened { lives: Int = 3 }\n\
             factory Open(id: RoomId) = {\n\
                 emit Opened {}\n\
                 Room { id }\n\
             }\n\
         }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn gives_generic_values_starting_values() {
    let source = dungeon(
        "value Shelf<T> { items: List<T> where Length(0, 4) = []  name: String }\n\
         fn empty(): Shelf<Int> = Shelf { name: \"top\" }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn explains_that_variant_fields_have_no_starting_values() {
    let source = dungeon("enum Item { Coins(count: Int = 1)  Flask }");

    assert_error(
        &source,
        "a variant's fields have no starting values, since they are given like arguments",
    );
}

#[test]
fn still_needs_a_starting_value_on_every_adapter_field() {
    let source = dungeon(
        "port Clock { fn tick(): Unit }\n\
         adapter FixedClock for Clock {\n\
             ticks: Int\n\
             fn tick() = Unit\n\
         }",
    );

    assert_error(
        &source,
        "adapter field `ticks` needs a starting value: `ticks: ... = ...`",
    );
}

#[test]
fn reports_an_adapter_starting_value_that_breaks_its_constraints() {
    let source = dungeon(
        "port Clock { fn tick(): Unit }\n\
         adapter FixedClock for Clock {\n\
             ticks: Int where Range(0, 9) = 10\n\
             fn tick() = Unit\n\
         }",
    );

    assert_error(
        &source,
        "`FixedClock` starts with `10` for `ticks`, which breaks `Range(0, 9)`",
    );
}
