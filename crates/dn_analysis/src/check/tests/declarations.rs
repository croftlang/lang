use super::{assert_error, dungeon, errors};

#[test]
fn accepts_the_small_dungeon() {
    assert_eq!(errors(&dungeon("")), Vec::<String>::new());
}

#[test]
fn reports_unknown_and_wrong_kinds_of_type() {
    assert_error(
        &dungeon("value Door { at: Place }"),
        "cannot find a type named `Place`",
    );
    let source = dungeon("const MAX: Int = 3\nvalue Door { at: MAX }");
    assert_error(&source, "expected a type, but `MAX` is a constant");
}

#[test]
fn checks_type_arguments_of_built_in_types() {
    assert_error(
        &dungeon("value Door { keys: List }"),
        "`List` takes 1 type arguments, found 0",
    );
    assert_error(
        &dungeon("value Door { at: Position<Int> }"),
        "`Position` takes no type arguments",
    );
}

#[test]
fn checks_constraint_arguments_and_each() {
    let source = dungeon("value Door { width: Int where Range(1) }");
    assert_error(&source, "`Range` takes 2 arguments here, found 1");
    let source = dungeon(
        "constraint Wide(width: Int, low: Int) = width > low\nvalue Door { width: Int where Wide }",
    );
    assert_error(&source, "`Wide` takes 1 arguments here, found 0");
    assert_error(
        &dungeon("value Door { width: Int where each Range(1, 2) }"),
        "`each` can only be used on a list",
    );
}

#[test]
fn checks_identities_of_values_entities_and_aggregates() {
    let source = dungeon("value Door { identity id: RoomId }");
    assert_error(
        &source,
        "value `Door` cannot have an identity; make it an entity",
    );
    let source = dungeon("aggregate Room {\n identity id: RoomId\n entity Enemy { lives: Int }\n}");
    assert_error(&source, "entity `Enemy` needs exactly one `identity` field");
    let source = dungeon("aggregate Room { lives: Int }");
    assert_error(
        &source,
        "aggregate `Room` needs exactly one `identity` field",
    );
    let source = dungeon("aggregate Room { identity id: Int }");
    assert_error(
        &source,
        "the identity of `Room` must be an identity type, not `Int`",
    );
}

#[test]
fn checks_invariants_name_an_error() {
    let aggregate = |invariant: &str| {
        let constraint = "constraint Ready(room: Room) = true";
        dungeon(&format!("{constraint}\naggregate Room {{\n identity id: RoomId\n {invariant}\n error NotReady\n}}"))
    };
    assert_eq!(
        errors(&aggregate("invariant Ready or NotReady")),
        Vec::<String>::new()
    );
    let message = "an invariant needs the error it fails with: `invariant Name or Error`";
    assert_error(&aggregate("invariant Ready"), message);
    assert_error(
        &aggregate("invariant Ready or Position"),
        "expected an error, but `Position` is a value",
    );
}
