use super::{assert_error, errors};

fn game(extra: &str) -> String {
    format!(
        "context Game {{\n\
         identity RoomId: Int\n\
         error StorageError\n\
         port Storage {{\n fn load(name: String): Result<Int, StorageError>\n}}\n\
         port Screen {{\n fn clear(): Unit\n}}\n\
         aggregate Room {{\n\
             identity id: RoomId\n\
             lives: Int\n\
             error NotFound\n\
             event Healed {{ lives: Int }}\n\
             command Heal(amount: Int) = {{\n set lives = lives + amount\n emit Healed {{ lives }}\n }}\n\
         }}\n\
         repository Rooms for Room in Storage\n\
         enum GameError {{\n include Room.Error\n include StorageError\n }}\n\
         {extra}\n}}"
    )
}

#[test]
fn accepts_a_service_that_loads_changes_and_saves() {
    let service =
        "service Play(id: RoomId): Result<Unit, GameError> {\n calls Rooms, Screen\n} = {\n\
                   let room = rooms.find(id)?\n let healed = issue Heal(amount: 1) on room?\n\
                   rooms.save(healed)?\n issue Heal(amount: 2) on Room(id)?\n screen.clear()\n}";
    assert_eq!(errors(&game(service)), Vec::<String>::new());
}

#[test]
fn checks_failures_are_handled_and_can_be_passed_up() {
    let service = "service Play(id: RoomId): Result<Unit, Room.Error> {\n calls Rooms\n} = {\n rooms.find(id)\n Unit\n}";
    assert_error(
        &game(service),
        "this `Result` is not handled: add `?`, or `or` and an error",
    );
    let service =
        "service Play(id: RoomId): Unit {\n calls Rooms\n} = {\n rooms.find(id)?\n Unit\n}";
    assert_error(
        &game(service),
        "this body cannot fail: give it a `Result` return type",
    );
    let service = "service Play(id: RoomId): Result<Unit, Room.Error> {\n calls Rooms\n} = {\n issue Heal(amount: 1) on Room(id)?\n}";
    assert_error(
        &game(service),
        "`Room.Error` does not include `StorageError`, so it cannot be passed up here",
    );
}

#[test]
fn checks_where_each_effect_is_allowed() {
    assert_error(
        &game("fn f(): Int = {\n set lives = 3\n 3\n}"),
        "`set` only works in commands, adapters and services",
    );
    assert_error(
        &game("fn f(): Unit = {\n loop {\n break\n }\n}"),
        "only services and adapters can loop; use a list method such as `map` here",
    );
    assert_error(
        &game("service Play(): Unit = {\n break\n}"),
        "`break` can only be used inside a loop",
    );
    assert_error(
        &game("fn f(): Unit = emit Healed { lives: 1 }"),
        "only a command can raise an event",
    );
    assert_error(
        &game("fn f(): Unit = native beep()"),
        "only an adapter can call native code such as `beep`",
    );
    let source = game("fn f(id: RoomId): Unit = {\n issue Heal(amount: 1) on Room(id)\n Unit\n}");
    let found = errors(&source);
    assert!(found
        .contains(&"only services and policies can load an aggregate by its identity".to_string()));
}

#[test]
fn checks_issued_commands_and_their_arguments() {
    let issue = |text: &str| {
        game(&format!(
            "fn f(room: Room): Result<Room, Room.Error> = {text}"
        ))
    };
    assert_eq!(
        errors(&issue("issue Heal(amount: 1) on room")),
        Vec::<String>::new()
    );
    assert_error(
        &issue("issue Heal(1) on room"),
        "arguments to `Heal` are always named: `name: value`",
    );
    assert_error(&issue("issue Jump on room"), "`Room` has no command `Jump`");
}

#[test]
fn counts_a_loop_through_a_range() {
    let service = "service Draw(): Unit {\n calls Screen\n} = {\n loop (let row where Range(0, 23)) {\n screen.clear()\n }\n}";
    assert_eq!(errors(&game(service)), Vec::<String>::new());
    let service = "service Draw(): Unit = {\n loop (let row where Length(0, 23)) {\n Unit\n }\n}";
    assert_error(
        &game(service),
        "a counted loop needs a `Range` constraint, such as `where Range(0, 9)`",
    );
}

#[test]
fn gives_services_their_command_line_arguments() {
    let service =
        "service Start(): Bool {\n calls Arguments\n} = arguments.all().contains(\"--demo\")";
    assert_eq!(errors(&game(service)), Vec::<String>::new());
    let service = "service Start(): Bool {\n calls Arguments\n} = arguments.all().contains(3)";
    assert_error(&game(service), "expected `String`, found `Int`");
}
