use super::{assert_error, errors};

/// A small context with an aggregate, to write bodies against.
fn game(extra: &str) -> String {
    format!(
        "context Game {{\n\
         identity RoomId: Int where Range(1, 20)\n\
         identity EnemyId: Int where Range(0, 9)\n\
         enum Direction {{ North, South }}\n\
         value Position {{ x: Int  y: Int }}\n\
         aggregate Room {{\n\
             identity id: RoomId\n\
             lives: Int\n\
             entity Enemy {{\n identity id: EnemyId\n hit_points: Int\n }}\n\
             enemies: List<Enemy>\n\
             error NotFound\n\
             event Moved {{ to: Position }}\n\
             command Heal(amount: Int) = {{\n set lives = lives + amount\n }}\n\
         }}\n\
         {extra}\n}}"
    )
}

#[test]
fn accepts_the_small_game() {
    assert_eq!(errors(&game("")), Vec::<String>::new());
}

#[test]
fn checks_a_body_against_its_return_type() {
    assert_error(
        &game("fn f(): Int = \"three\""),
        "expected `Int`, found `String`",
    );
    assert_error(
        &game("fn f(): Int = 1 + true"),
        "expected `Int`, found `Bool`",
    );
    assert_error(&game("fn f(): Int = width"), "cannot find `width` here");
}

#[test]
fn keeps_identities_apart() {
    assert_eq!(
        errors(&game("const FIRST: RoomId = 1")),
        Vec::<String>::new()
    );
    let source = game("fn f(enemy: EnemyId): RoomId = enemy");
    assert_error(&source, "expected `RoomId`, found `EnemyId`");
}

#[test]
fn finds_bare_variants_from_the_expected_type() {
    let source =
        game("fn f(up: Bool): Option<Direction> = {\n if up {\n Some(North)\n }\n Some(South)\n}");
    assert_eq!(errors(&source), Vec::<String>::new());
    let source = game("fn f(up: Bool): Direction = {\n if up {\n 3\n }\n South\n}");
    assert_error(
        &source,
        "this guard gives back `Int`, but the body gives back `Direction`",
    );
}

#[test]
fn checks_records_and_with() {
    assert_error(
        &game("fn f(): Position = Position { x: 1 }"),
        "`Position` needs a value for `y`",
    );
    let source = game("fn f(p: Position): Position = p with { z: 1 }");
    assert_error(&source, "`Position` has no field `z`");
}

#[test]
fn checks_every_variant_is_matched() {
    let source = game("fn f(d: Direction): Int = match d {\n North -> 1\n}");
    assert_error(&source, "this match does not handle `South`");
    let source = game("fn f(d: Option<Int>): Int = match d {\n Some(n) -> n\n None -> 0\n}");
    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn checks_list_methods_and_lambdas() {
    let source = game("fn f(room: Room): List<EnemyId> = room.enemies.filter(e -> e.hit_points > 0).map(e -> e.id)");
    assert_eq!(errors(&source), Vec::<String>::new());
    let source = game("fn f(room: Room): Bool = room.enemies.all(e -> e.hit_points)");
    assert_error(
        &source,
        "the lambda passed to `all` must give back `Bool`, found `Int`",
    );
    let source = game("fn f(room: Room, id: RoomId): Bool = room.enemies.contains(id)");
    assert_error(&source, "expected `EnemyId`, found `RoomId`");
}

#[test]
fn checks_string_methods() {
    assert_eq!(
        errors(&game(
            "fn f(name: String): Bool = name.trim().starts_with(\"Sir\")"
        )),
        Vec::<String>::new()
    );
    assert_error(
        &game("fn f(name: String): Int = name.shout()"),
        "`String` has no method `shout`",
    );
}
