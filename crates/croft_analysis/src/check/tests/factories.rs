//! Factories: where aggregates are made, and where they are changed.

use super::{assert_error, errors};

/// A room with one factory, and `extra` beside it in the context.
fn game(extra: &str) -> String {
    format!(
        "context Game {{\n\
         identity RoomId for Room: Int\n\
         aggregate Room {{\n\
             id: RoomId\n\
             lives: Int\n\
             error NotFound\n\
             event Opened {{ lives: Int }}\n\
             factory Open(id: RoomId, lives: Int) = {{\n emit Opened {{ lives }}\n Room {{ id, lives }}\n }}\n\
             command Heal(amount: Int) = {{\n set lives = lives + amount\n }}\n\
         }}\n\
         repository Rooms for Room\n\
         enum GameError {{\n include Room.Error\n include StorageError\n }}\n\
         {extra}\n}}"
    )
}

#[test]
fn accepts_a_service_that_makes_and_saves() {
    let service = "service Start(): Result<Unit, GameError> {\n calls { Rooms }\n} = {\n\
                   let room = Room.Open(id: 1, lives: 3)?\n Rooms.save(room)?\n}";

    assert_eq!(errors(&game(service)), Vec::<String>::new());
}

#[test]
fn makes_aggregates_only_in_their_factories() {
    let service = "service Start(): Result<Unit, StorageError> {\n calls { Rooms }\n} = \
                   Rooms.save(Room { id: 1, lives: 3 })";

    assert_error(
        &game(service),
        "a new `Room` is made by its factory: `Room.Open`",
    );

    let bare = "context Game {\n identity RoomId for Room: Int\n aggregate Room { id: RoomId }\n\
                fn first(): Room = Room { id: 1 }\n}";

    assert_error(
        bare,
        "a new `Room` is made by a factory; declare one in the aggregate: \
         `factory Open(...) = Room { ... }`",
    );
}

#[test]
fn changes_aggregates_only_through_commands() {
    let service =
        "service Revive(id: RoomId): Result<Unit, GameError> {\n calls { Rooms }\n} = {\n\
                   let room = Rooms.find(id)?\n Rooms.save(room with { lives: 3 })?\n}";

    assert_error(
        &game(service),
        "a `Room` changes by issuing one of its commands, not with `with`",
    );
}

#[test]
fn calls_factories_only_where_making_an_aggregate_belongs() {
    assert_error(
        &game("constraint Fresh(room: Room) = Room.Open(id: 1, lives: 3) == Ok(room)"),
        "only services, policies and functions can make an aggregate with `Room.Open`",
    );
}

#[test]
fn keeps_a_factory_to_making_its_aggregate() {
    let source = "context Game {\n\
                  identity RoomId for Room: Int\n\
                  aggregate Room {\n\
                      id: RoomId\n\
                      lives: Int\n\
                      factory Open(id: RoomId): Room = Room { id, lives: 3 }\n\
                  }\n}";

    assert_error(
        source,
        "a factory gives back the aggregate it makes, so it has no return type",
    );

    let source = "context Game {\n\
                  identity RoomId for Room: Int\n\
                  aggregate Room {\n\
                      id: RoomId\n\
                      lives: Int\n\
                      factory Open(id: RoomId) = {\n set lives = 3\n Room { id, lives: 3 }\n }\n\
                  }\n}";

    assert_error(
        source,
        "`set` only works in commands, adapters and services",
    );
}

#[test]
fn lets_a_repositorys_own_adapter_build_its_aggregate() {
    let adapter = "repository Saved for Room {\n fn first(): Result<Room, StorageError>\n}\n\
                   adapter FixedRooms for Saved = {\n fn first() = Ok(Room { id: 1, lives: 3 })\n}";

    let found = errors(&game(adapter));

    assert!(
        !found.iter().any(|error| error.contains("is made by")),
        "{found:?}"
    );
}
