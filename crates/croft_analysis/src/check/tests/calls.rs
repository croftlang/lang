//! `calls { ... } from` and names given with `as`, in `calls` and `import`.

use super::{check_files, errors};

/// A context that exports a port, a service and an event, for a domain to use.
const DUNGEON: &str = "context Dungeon {\n\
     export { Keyboard, Greet, Room.Moved }\n\
     port Keyboard {\n fn pressed(): Int\n}\n\
     identity RoomId for Room: Int\n\
     aggregate Room {\n id: RoomId\n event Moved { to: Int }\n}\n\
     service Greet(): Unit {\n calls { Console }\n} = Console.print(\"hello\")\n\
     }";

/// The program with `domain` as its domain.
fn program(domain: &str) -> Vec<String> {
    match check_files(&[DUNGEON, domain]) {
        Ok(_) => Vec::new(),
        Err(errors) => errors,
    }
}

/// The errors in a `Game` domain over `DUNGEON`, with `contract` and `body`.
fn game(contract: &str, body: &str) -> Vec<String> {
    program(&format!(
        "domain Game {{\n contexts {{ Dungeon }}\n {contract}\n}} = {{\n {body}\n}}"
    ))
}

#[test]
fn calls_a_port_from_another_context_by_the_name_it_is_given() {
    let contract = "calls { Keyboard as keys } from Dungeon\n calls { Console as out }";
    let body = "out.print(keys.pressed().to_string())";

    assert_eq!(game(contract, body), Vec::<String>::new());
}

#[test]
fn calls_a_port_by_its_own_name() {
    let contract = "calls { Keyboard } from Dungeon\n calls { Console }";
    let body = "Console.print(Keyboard.pressed().to_string())";

    assert_eq!(game(contract, body), Vec::<String>::new());
}

#[test]
fn names_the_name_calls_gives_a_port() {
    let renamed = game("calls { Console as out }", "Console.print(\"hi\")");
    assert_eq!(
        renamed,
        ["cannot find `Console` here: `calls` names it `out`"]
    );

    let snake_case = game("calls { Console }", "console.print(\"hi\")");
    assert_eq!(
        snake_case,
        ["cannot find `console` here: `calls` names it `Console`"]
    );
}

#[test]
fn checks_what_calls_brings_in_from_a_context() {
    let service = game("calls { Greet } from Dungeon", "Unit");
    assert_eq!(
        service,
        ["expected a port or repository, but `Greet` is a service"]
    );

    let unexported = game("calls { Screen } from Dungeon", "Unit");
    assert_eq!(unexported, ["context `Dungeon` does not export `Screen`"]);

    let unlisted = program(
        "context Arcade {}\n\
         domain Game {\n contexts { Dungeon, Arcade }\n calls { Keyboard } from Arcade\n} = Unit",
    );
    assert_eq!(unlisted, ["context `Arcade` does not export `Keyboard`"]);
}

#[test]
fn lists_each_called_port_once_under_a_free_name() {
    let twice = game(
        "calls { Console, Keyboard as Console } from Dungeon",
        "Unit",
    );
    assert!(twice.contains(&"`Console` is already in `calls`".to_string()));

    let taken = game(
        "import { Greet } from Dungeon\n calls { Console as Greet }",
        "Unit",
    );
    assert_eq!(
        taken,
        ["`Greet` already names something here, so `Console` can't be called that"]
    );
}

#[test]
fn a_service_calls_a_port_from_a_context_upstream_of_it() {
    let arcade = "context Arcade {\n\
         service Play(): Int {\n calls { Keyboard as keys } from Dungeon\n} = keys.pressed()\n\
         }";

    let upstream = "domain Game {\n contexts { Dungeon -> Arcade }\n} = Unit";
    assert!(check_files(&[DUNGEON, arcade, upstream]).is_ok());

    let unrelated = "domain Game {\n contexts { Dungeon, Arcade }\n} = Unit";
    let found = check_files(&[DUNGEON, arcade, unrelated]).err().unwrap();
    assert_eq!(
        found,
        ["`Arcade` uses `Dungeon`, so `contexts` needs `Dungeon -> Arcade`"]
    );
}

#[test]
fn calls_a_standard_port_from_its_standard_context() {
    let notes = "context Notes {\n\
         import { FileError } from Std.Files\n\
         service Keep(): Result<Unit, FileError> {\n calls { Files as disk } from Std.Files\n} = {\n\
         let data = disk.read(\"notes.txt\", 4096)?\n disk.write(\"copy.txt\", data)\n}\n\
         }";

    assert_eq!(errors(notes), Vec::<String>::new());
}

#[test]
fn uses_a_name_given_with_as_in_place_of_the_declared_one() {
    let scoring = "context Scoring {\n\
         import { Room.Moved as Moved, Greet as Welcome } from Dungeon\n\
         service Start(): Unit = Welcome()\n\
         fn to(moved: Moved): Int = moved.to\n\
         }";
    let domain = "domain Game {\n contexts { Dungeon -> Scoring }\n} = Unit";
    assert!(check_files(&[DUNGEON, scoring, domain]).is_ok());

    let declared = scoring.replace("= Welcome()", "= Greet()");
    let found = check_files(&[DUNGEON, &declared, domain]).err().unwrap();
    assert_eq!(found, ["cannot find `Greet` here"]);
}

#[test]
fn keeps_the_name_of_a_built_in_method() {
    let source = "context Words {\n import { trim as strip } from Std.Text\n}";
    assert_eq!(
        errors(source),
        ["`trim` is built in, so it can't be renamed"]
    );
}
