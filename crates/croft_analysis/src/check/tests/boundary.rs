use super::{assert_error, dungeon, errors};

const SCREEN: &str = "port Screen {\n fn clear(): Unit\n fn draw(at: Position): Unit\n}";

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
        "{SCREEN}\nservice Play(): Unit {{\n calls Screen, Position\n}} = Unit"
    ));
    assert_error(
        &source,
        "expected a port or repository, but `Position` is a value",
    );
    assert_error(
        &dungeon("fn f(): Int {\n calls Screen\n} = 3"),
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

fn with_repository(operations: &str, extra: &str) -> String {
    dungeon(&format!(
        "aggregate Room {{\n identity id: RoomId\n error NotFound\n }}\n\
         identity DoorId: Int where Range(1, 9)\n\
         aggregate Door {{\n identity id: DoorId\n }}\n\
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
        &dungeon("aggregate Room {\n identity id: RoomId\n }\nrepository Rooms for Room {\n}"),
        "repository `Rooms` declares no operations; leave out the braces for `find` and `save`",
    );
}

#[test]
fn calls_only_what_a_repository_declares() {
    let find = "fn find(id: RoomId): Result<Option<Room>, StorageError>";
    let service = |body: &str| {
        format!("service Look(): Result<Unit, StorageError> {{\n calls Rooms\n}} = {{\n{body}\n}}")
    };
    let found = service("let room = rooms.find(1)?\nOk(Unit)");
    assert_eq!(errors(&with_repository(find, &found)), Vec::<String>::new());
    assert_error(
        &with_repository(find, &service("rooms.save(Room { id: 1 })")),
        "repository `Rooms` has no operation `save`; it has `find`",
    );
}

#[test]
fn issues_on_an_identity_only_with_a_find_and_a_save() {
    let source = |find: &str| {
        dungeon(&format!(
            "aggregate Room {{\n identity id: RoomId\n lit: Bool\n error NotFound\n command Light = {{\n set lit = true\n }}\n }}\n\
             enum Lookup {{\n include Room.NotFound\n include StorageError\n }}\n\
             repository Rooms for Room {{\n{find}\n fn save(room: Room): Result<Unit, StorageError>\n}}\n\
             service Light(): Result<Unit, Room.Error> {{\n calls Rooms\n}} = issue Light on Room(1)"
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
