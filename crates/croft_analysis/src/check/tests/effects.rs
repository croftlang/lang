//! Where loading, saving, issuing, port calls and loops are allowed, and how
//! failures are passed up.

use super::{assert_error, errors};

/// A game with a `Screen` port, a `Room` aggregate and its repository, and
/// `extra` beside them.
fn game(extra: &str) -> String {
    format!(
        "context Game {{\n\
         identity RoomId for Room: Int\n\
         port Screen {{\n fn clear(): Unit\n}}\n\
         aggregate Room {{\n\
             id: RoomId\n\
             lives: Int\n\
             error NotFound\n\
             event Healed {{ lives: Int }}\n\
             command Heal(amount: Int) = {{\n set lives = lives + amount\n emit Healed {{ lives }}\n }}\n\
         }}\n\
         repository Rooms for Room\n\
         enum GameError {{\n include Room.Error\n include StorageError\n }}\n\
         {extra}\n}}"
    )
}

#[test]
fn accepts_a_service_that_loads_changes_and_saves() {
    let service =
        "service Play(id: RoomId): Result<Unit, GameError> {\n calls { Rooms, Screen }\n} = {\n\
                   let room = Rooms.find(id)?\n let healed = issue Heal(amount: 1) on room?\n\
                   Rooms.save(healed)?\n issue Heal(amount: 2) on Room(id)?\n Screen.clear()\n}";

    assert_eq!(errors(&game(service)), Vec::<String>::new());
}

#[test]
fn checks_failures_are_handled_and_can_be_passed_up() {
    let service = "service Play(id: RoomId): Result<Unit, Room.Error> {\n calls { Rooms }\n} = {\n Rooms.find(id)\n Unit\n}";

    assert_error(
        &game(service),
        "this `Result` is not handled: add `?`, or `or` and an error",
    );

    let service =
        "service Play(id: RoomId): Unit {\n calls { Rooms }\n} = {\n Rooms.find(id)?\n Unit\n}";

    assert_error(
        &game(service),
        "this body cannot fail: give it a `Result` return type",
    );

    let service = "service Play(id: RoomId): Result<Unit, Room.Error> {\n calls { Rooms }\n} = {\n issue Heal(amount: 1) on Room(id)?\n}";

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
        "only a command or factory can raise an event",
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
    let service = "service Draw(): Unit {\n calls { Screen }\n} = {\n loop (let row where Range(0, 23)) {\n Screen.clear()\n }\n}";
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
        "service Start(): Bool {\n calls { Arguments }\n} = Arguments.all().contains(\"--demo\")";
    assert_eq!(errors(&game(service)), Vec::<String>::new());

    let service = "service Start(): Bool {\n calls { Arguments }\n} = Arguments.all().contains(3)";
    assert_error(&game(service), "expected `String`, found `Int`");
}

#[test]
fn gives_services_the_built_in_console() {
    let service = "service Greet(): Unit {\n calls { Console }\n} = Console.print(\"hello\")";
    assert_eq!(errors(&game(service)), Vec::<String>::new());

    let service = "service Greet(): Unit {\n calls { Console }\n} = Console.print(3)";
    assert_error(&game(service), "expected `String`, found `Int`");

    let service = "service Greet(): Unit {\n calls { Console }\n} = Console.shout(\"hello\")";
    assert_error(&game(service), "port `Console` has no operation `shout`");
}

#[test]
fn reads_lines_and_prints_errors_through_the_built_in_console() {
    let service = "service Echo(): Unit {\n calls { Console }\n} = match Console.read_line() {\n Some(line) -> Console.print_error(line)\n None -> Unit\n}";
    assert_eq!(errors(&game(service)), Vec::<String>::new());

    let service = "service Echo(): String {\n calls { Console }\n} = Console.read_line()";
    assert_error(&game(service), "expected `String`, found `Option<String>`");
}

#[test]
fn gives_services_the_built_in_process() {
    let service = "service Quit(): Unit {\n calls { Process }\n} = Process.exit(3)";
    assert_eq!(errors(&game(service)), Vec::<String>::new());

    let service = "service Quit(): Unit = Process.exit(3)";
    assert_error(&game(service), "cannot find `Process.exit` here");

    let service = "service Quit(): Unit {\n calls { Process }\n} = Process.exit(256)";
    assert_error(
        &game(service),
        "`Process.exit` is given `256` for `code`, which breaks `Range(0, 255)`",
    );
}

#[test]
fn checks_known_arguments_to_a_port_operation() {
    let source = game(
        "port Log {\n fn write(level: Int where Range(1, 3))\n}\n\
         service Note(): Unit {\n calls { Log }\n} = Log.write(level: 4)",
    );

    assert_error(
        &source,
        "`Log.write` is given `4` for `level`, which breaks `Range(1, 3)`",
    );
}

#[test]
fn gives_back_unit_when_there_is_no_return_type() {
    let source = game(
        "port Log {\n fn write(text: String)\n}\n\
         service Note(): Unit {\n calls { Log }\n} = Log.write(\"hi\")\n\
         fn nothing() = Unit",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

/// An aggregate with no errors never fails, so issuing its commands can be
/// passed up through any error type, and its empty error set included.
#[test]
fn an_aggregate_with_no_errors_passes_up_anywhere() {
    let source = "context Shipping {\n\
         identity EventId for Handling: Int\n\
         error NoRoute\n\
         aggregate Handling {\n id: EventId\n event Registered {}\n command Register = {\n emit Registered {}\n }\n }\n\
         enum ShippingError {\n include NoRoute\n }\n\
         enum Everything {\n include Handling.Error\n include NoRoute\n }\n\
         service Register(handling: Handling): Result<Handling, ShippingError> = {\n let registered = issue Register on handling?\n registered\n }\n\
         }";

    assert_eq!(errors(source), Vec::<String>::new());
}
