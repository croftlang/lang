//! Ports, adapters, repositories, and what contexts publish and use.

use super::{assert_error, dungeon, errors};

/// A `Screen` port with two operations.
const SCREEN: &str = "port Screen {\n fn clear(): Unit\n fn draw(at: Position): Unit\n}";

/// The dungeon with `SCREEN` and a `TestScreen` adapter holding `operations`.
fn with_adapter(operations: &str) -> String {
    dungeon(&format!(
        "{SCREEN}\nadapter TestScreen for Screen = {{\n{operations}\n}}"
    ))
}

#[test]
fn checks_adapters_provide_their_port() {
    let complete = "fn clear() = Unit\nfn draw(at) = Unit";
    assert_eq!(errors(&with_adapter(complete)), Vec::<String>::new());

    assert_error(
        &with_adapter("fn clear() = Unit"),
        "adapter `TestScreen` does not provide `draw` from port `Screen`",
    );

    let extra = format!("{complete}\nfn beep() = Unit");

    assert_error(
        &with_adapter(&extra),
        "port `Screen` has no operation `beep`",
    );

    let wrong = "fn clear() = Unit\nfn draw() = Unit";

    assert_error(
        &with_adapter(wrong),
        "`draw` takes 1 parameters in port `Screen`, found 0",
    );
}

#[test]
fn checks_ports_only_declare_operations() {
    let source = dungeon("port Screen {\n fn clear(): Unit = Unit\n}");

    assert_error(
        &source,
        "a port only declares `clear`; an adapter gives it a body",
    );
}

#[test]
fn checks_services_use_ports() {
    let source = dungeon(&format!(
        "{SCREEN}\nservice Play(): Unit {{\n calls {{ Screen, Position }}\n}} = Unit"
    ));

    assert_error(
        &source,
        "expected a port or repository, but `Position` is a value",
    );

    assert_error(
        &dungeon("fn f(): Int {\n calls { Screen }\n} = 3"),
        "only a service can call ports",
    );
}

#[test]
fn checks_publish_and_use() {
    let scoring = |names: &str| format!("context Scoring {{\n use {{ {names} }} from Dungeon\n}}");
    let dungeon = dungeon("publish { RoomId }");
    let checked = |names: &str| super::check_files(&[&dungeon, &scoring(names)]);
    assert!(checked("RoomId").is_ok());

    assert_eq!(
        checked("Position").err().unwrap(),
        vec!["context `Dungeon` does not publish `Position`"]
    );

    assert_error(
        &super::dungeon("publish { Door }"),
        "cannot publish `Door`: context `Dungeon` does not declare it",
    );
}

/// The dungeon with a `Rooms` repository declaring `operations`, and `extra`
/// beside it.
fn with_repository(operations: &str, extra: &str) -> String {
    dungeon(&format!(
        "aggregate Room {{\n id: RoomId\n error NotFound\n }}\n\
         identity DoorId for Door: Int where Range(1, 9)\n\
         aggregate Door {{\n id: DoorId\n }}\n\
         repository Rooms for Room {{\n{operations}\n}}\n{extra}"
    ))
}

#[test]
fn checks_a_repositorys_own_operations() {
    let find = "fn find(id: RoomId): Result<Option<Room>, StorageError>";
    let empty = "fn empty(): Result<List<Room>, StorageError>";

    assert_eq!(
        errors(&with_repository(&format!("{find}\n{empty}"), "")),
        Vec::<String>::new()
    );

    assert_error(
        &with_repository("fn find(id: RoomId): Option<Room>", ""),
        "`find` can fail with `StorageError`, whichever adapter keeps the aggregates: `Result<..., StorageError>`",
    );

    assert_error(
        &with_repository("fn door(id: RoomId): Result<Door, StorageError>", ""),
        "repository `Rooms` keeps `Room`, so `door` can't take or give back `Door`",
    );

    assert_error(
        &dungeon("aggregate Room {\n id: RoomId\n }\nrepository Rooms for Room {\n}"),
        "repository `Rooms` declares no operations; leave out the braces for `find` and `save`",
    );
}

#[test]
fn calls_only_what_a_repository_declares() {
    let find = "fn find(id: RoomId): Result<Option<Room>, StorageError>";

    let service = |body: &str| {
        format!("service Look(): Result<Unit, StorageError> {{\n calls {{ Rooms }}\n}} = {{\n{body}\n}}")
    };

    let found = service("let room = Rooms.find(1)?\nOk(Unit)");
    assert_eq!(errors(&with_repository(find, &found)), Vec::<String>::new());

    assert_error(
        &with_repository(find, &service("Rooms.save(Room { id: 1 })")),
        "repository `Rooms` has no operation `save`; it has `find`",
    );
}

#[test]
fn issues_on_an_identity_only_with_a_find_and_a_save() {
    let source = |find: &str| {
        dungeon(&format!(
            "aggregate Room {{\n id: RoomId\n lit: Bool\n error NotFound\n command Light = {{\n set lit = true\n }}\n }}\n\
             enum Lookup {{\n include Room.NotFound\n include StorageError\n }}\n\
             repository Rooms for Room {{\n{find}\n fn save(room: Room): Result<Unit, StorageError>\n}}\n\
             service Light(): Result<Unit, Room.Error> {{\n calls {{ Rooms }}\n}} = issue Light on Room(1)"
        ))
    };

    let found = errors(&source("fn load(id: RoomId): Result<Room, Lookup>"));

    assert!(
        found.iter().all(|e| !e.contains("loads and saves")),
        "{found:?}"
    );

    let missing = errors(&source(
        "fn load(id: RoomId): Result<Option<Room>, StorageError>",
    ));

    assert!(
        missing.contains(&"issuing a command on `Room(...)` loads and saves it through `Rooms`, which needs a find by identity that fails with `Room.NotFound`, and a save".to_string()),
        "{missing:?}"
    );
}

/// A `Dice` port whose `roll` has `contract` as its contract, and a service
/// that calls it with `call`.
fn with_contract(contract: &str, call: &str) -> String {
    dungeon(&format!(
        "port Dice {{\n fn roll(low: Int, high: Int): Int {{\n {contract}\n }}\n}}\n\
         service Play(): Int {{\n calls {{ Dice }}\n}} = {call}"
    ))
}

#[test]
fn checks_a_port_contract_at_each_call() {
    assert_eq!(
        errors(&with_contract("low <= high", "Dice.roll(1, 6)")),
        Vec::<String>::new()
    );

    assert_error(
        &with_contract("low <= high", "Dice.roll(6, 1)"),
        "`Dice.roll(6, 1)` breaks `low <= high`",
    );

    assert_error(
        &with_contract("low + 1", "Dice.roll(1, 6)"),
        "expected `Bool`, found `Int`",
    );

    assert_error(
        &with_contract("missing > 0", "Dice.roll(1, 6)"),
        "cannot find `missing` here",
    );
}

#[test]
fn checks_a_port_contract_only_holds() {
    let source = with_contract("low <= high or Reversed", "Dice.roll(1, 6)")
        .replace("port Dice", "error Reversed\nport Dice");

    assert_error(
        &source,
        "a port's contract line must hold, so `roll` can't fail with `Reversed`; give it a `Result` its adapter fails with",
    );
}

/// A port's parameter constraints are the caller's to keep too.
#[test]
fn checks_a_known_argument_against_a_port_parameter() {
    let source = dungeon(
        "port Dice {\n fn roll(sides: Int where Range(2, 20)): Int\n}\n\
         service Play(): Int {\n calls { Dice }\n} = Dice.roll(1)",
    );

    assert_error(
        &source,
        "`Dice.roll` is given `1` for `sides`, which breaks `Range(2, 20)`",
    );
}

/// A port's parameter constraint can read another parameter.
#[test]
fn checks_a_port_parameter_constraint_that_reads_another_parameter() {
    let source = dungeon(
        "port Dice {\n fn between(low: Int, high: Int where Range(low, 99)): Int\n}\n\
         service Play(): Int {\n calls { Dice }\n} = Dice.between(6, 1)",
    );

    assert_error(
        &source,
        "`Dice.between` is given `1` for `high`, which breaks `Range(low, 99)`, with `low` given `6`",
    );
}
