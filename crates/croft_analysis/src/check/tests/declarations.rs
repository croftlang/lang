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
    let source = dungeon("identity DoorId for Door: Int\nvalue Door { id: DoorId }");
    assert_error(
        &source,
        "`Door` is a value, which has no identity; make it an entity",
    );
    let source = dungeon("aggregate Room {\n id: RoomId\n entity Enemy { lives: Int }\n}");
    assert_error(
        &source,
        "entity `Enemy` has no identity; declare one in the context: `identity EnemyId for Enemy: Int`",
    );
    let source = dungeon("aggregate Room { lives: Int }");
    assert_error(
        &source,
        "aggregate `Room` needs a field of its identity: `id: RoomId`",
    );
    let source = dungeon("aggregate Room { id: RoomId  copy: RoomId }");
    assert_error(
        &source,
        "aggregate `Room` already has a field of its identity `RoomId`; it holds only one",
    );
    let source = dungeon("identity Key for Lock: Int");
    assert_error(
        &source,
        "identity `Key` is for `Lock`, but `Dungeon` has no aggregate or entity by that name",
    );
    let source = dungeon("identity RoomKey for Room: Int");
    assert_error(
        &source,
        "`Room` is already identified by `RoomId`; it has only one identity",
    );
}

#[test]
fn finds_identity_fields_by_their_type_whatever_they_are_called() {
    let source = dungeon(
        "identity HallId for Hall: Int\n\
         aggregate Hall { key: HallId  next_room: RoomId }\n\
         identity EnemyId for Room.Enemy: Int\n\
         aggregate Room {\n number: RoomId\n entity Enemy { tag: EnemyId  lives: Int }\n}",
    );
    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn asks_which_entity_an_identity_is_for_when_two_share_a_name() {
    let source = dungeon(
        "identity HallId for Hall: Int\n\
         identity EnemyId for Enemy: Int\n\
         aggregate Hall {\n id: HallId\n entity Enemy { id: EnemyId }\n}\n\
         aggregate Room {\n id: RoomId\n entity Enemy { id: EnemyId }\n}",
    );
    assert!(errors(&source).contains(
        &"`Enemy` could be Hall.Enemy or Room.Enemy; write which one, such as `Hall.Enemy`"
            .to_string()
    ));
}

#[test]
fn checks_invariants_name_an_error() {
    let aggregate = |invariant: &str| {
        let constraint = "constraint Ready(room: Room) = true";
        dungeon(&format!(
            "{constraint}\naggregate Room {{\n id: RoomId\n {invariant}\n error NotReady\n}}"
        ))
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
