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

#[test]
fn checks_bit_operators() {
    let source = game("fn f(bits: Int): Bool = bits & 0b100 != 0 && ~bits >> 1 > 0");
    assert_eq!(errors(&source), Vec::<String>::new());
    assert_error(
        &game("fn f(on: Bool): Int = on & 1"),
        "expected `Int`, found `Bool`",
    );
    assert_error(
        &game("fn f(bits: Int): Int = bits << 32"),
        "cannot shift by 32, only by 0 to 31",
    );
}

#[test]
fn checks_decimals() {
    let source = game(
        "fn f(price: Decimal<2>, count: Int): Decimal<2> = {\n let tax: Decimal<2> = 0.2\n price * count + price * tax - 1\n}",
    );
    assert_eq!(errors(&source), Vec::<String>::new());
    assert_error(
        &game("fn f(): Decimal<2> = 2.505"),
        "`2.505` has more than 2 decimal places",
    );
    assert_error(
        &game("fn f(price: Decimal): Int = 0"),
        "`Decimal` needs its decimal places, as in `Decimal<2>`",
    );
    assert_error(
        &game("fn f(price: Decimal<9>): Int = 0"),
        "`Decimal` has 1 to 6 decimal places, found 9",
    );
    assert_error(
        &game("fn f(a: Decimal<2>, b: Decimal<3>): Decimal<2> = a + b"),
        "expected `Decimal<2>`, found `Decimal<3>`",
    );
    assert_error(
        &game("fn f(a: Decimal<2>): Decimal<2> = a % 2"),
        "a `Decimal` has no remainder",
    );
    assert_error(
        &game("fn f(a: Decimal<2>): Decimal<2> = 1 / a"),
        "only a `Decimal` can be divided by a `Decimal`",
    );
    assert_error(
        &game("fn f(counts: List<Int>): List<Decimal<2>> = counts"),
        "expected `List<Decimal<2>>`, found `List<Int>`",
    );
}
