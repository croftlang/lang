use super::{assert_error, dungeon};

#[test]
fn checks_function_shapes() {
    assert_error(&dungeon("fn width() = 3"), "expected `Unit`, found `Int`");
    assert_error(
        &dungeon("fn width(): Int"),
        "`width` needs a body after `=`",
    );
    assert_error(
        &dungeon("fn width: Int = 3"),
        "`width` needs brackets, even with no parameters",
    );
    assert_error(
        &dungeon("fn width(door): Int = 3"),
        "parameter `door` needs a type",
    );
    let source = dungeon("aggregate Room {\n identity id: RoomId\n command Reset: Int = 3\n}");
    assert_error(
        &source,
        "a command gives back its aggregate, so it has no return type",
    );
}

#[test]
fn checks_names_are_written_in_the_right_case() {
    assert_error(
        &dungeon("value door_frame { x: Int }"),
        "value `door_frame` should be written in PascalCase",
    );
    assert_error(
        &dungeon("fn Width(): Int = 3"),
        "function `Width` should be written in snake_case",
    );
    assert_error(
        &dungeon("const max: Int = 3"),
        "constant `max` should be written in UPPER_SNAKE_CASE",
    );
    assert_error(
        &dungeon("value Int { x: Int }"),
        "`Int` is a built-in type and cannot be declared again",
    );
}
