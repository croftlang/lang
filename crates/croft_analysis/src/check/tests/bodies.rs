use super::{assert_error, errors};

/// A small context with an aggregate, to write bodies against.
fn game(extra: &str) -> String {
    format!(
        "context Game {{\n\
         use {{ trim, starts_with }} from Std.Text\n\
         use {{ Decimal, max, min, abs }} from Std.Numbers\n\
         identity RoomId for Room: Int where Range(1, 20)\n\
         identity EnemyId for Enemy: Int where Range(0, 9)\n\
         enum Direction {{ North, South }}\n\
         value Position {{ x: Int  y: Int }}\n\
         aggregate Room {{\n\
             id: RoomId\n\
             lives: Int\n\
             entity Enemy {{\n id: EnemyId\n hit_points: Int\n }}\n\
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

#[test]
fn checks_imports_from_standard_contexts() {
    assert_error(
        "context Shop {\n fn f(name: String): String = name.to_upper()\n}",
        "`to_upper` needs `use { to_upper } from Std.Text`",
    );

    assert_error(
        "context Shop {\n fn f(a: Int): Int = abs(a)\n}",
        "`abs` needs `use { abs } from Std.Numbers`",
    );

    assert_error(
        "context Shop {\n use { shout } from Std.Text\n}",
        "`Std.Text` does not provide `shout`",
    );

    let source = "context Shop {\n use { to_upper } from Std.Text\n fn f(names: List<String>): Bool = names.contains(\"A\") && names.length > 0\n fn g(name: String): String = name.to_upper()\n}";
    assert_eq!(errors(source), Vec::<String>::new());
}

#[test]
fn checks_bytes() {
    let source = "context Shop {\n use { to_text } from Std.Text\n fn f(): Option<Int> = {\n let data: Bytes = [1, 2, 0xFF]\n data.add(3).get(0)\n }\n fn g(data: Bytes where Length(0, 512)): String = data.to_text()\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n fn f(): Bytes = [1, 256]\n}",
        "a byte is from 0 to 255, found 256",
    );

    assert_error(
        "context Shop {\n fn f(data: Bytes): String = data.to_text()\n}",
        "`to_text` needs `use { to_text } from Std.Text`",
    );

    // `+` joins `Bytes`, with a literal on either side.
    let source = "context Shop {\n fn f(data: Bytes): Bytes = [0] + data + [1, 2]\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n fn f(data: Bytes): Bytes = data + 1\n}",
        "expected `Bytes`, found `Int`",
    );

    assert_error(
        "context Shop {\n fn f(data: Bytes): Bytes = data - data\n}",
        "cannot do arithmetic on `Bytes`",
    );
}

#[test]
fn handles_the_rest_of_a_match_with_a_wildcard() {
    let source = game("fn f(d: Direction): Int = match d {\n North -> 1\n _ -> 0\n}");
    assert_eq!(errors(&source), Vec::<String>::new());

    assert_error(
        &game("fn f(d: Direction): Int = match d {\n _ -> 0\n North -> 1\n}"),
        "`_` handles everything left, so it must be the last arm",
    );

    assert_error(
        &game("fn f(d: Direction): Int = match d {\n North -> 1\n South -> 2\n _ -> 0\n}"),
        "every variant is already handled, so `_` is never reached",
    );

    assert_error(
        &game("fn f(d: Option<Int>): Int = match d {\n Some(n) -> n\n _(x) -> 0\n}"),
        "`_` stands for several variants, so it can't name their fields",
    );
}

#[test]
fn checks_single_value_types() {
    let source = "context Shop {\n value Score: Int where Range(0, 999)\n fn f(score: Score): Score = Score(score.value + 1)\n fn g(): Score = 7\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n value Score: Int\n fn f(n: Int): Score = n\n}",
        "expected `Score`, found `Int`",
    );

    assert_error(
        "context Shop {\n value Score: Int\n fn f(score: Score): Score = score + 1\n}",
        "cannot do arithmetic on `Score`",
    );

    assert_error(
        "context Shop {\n value Point { x: Int }\n fn f(): Point = Point(3)\n}",
        "`Point` is made with its fields: `Point { ... }`",
    );
}

#[test]
fn checks_maps() {
    let source = "context Shop {\n use { Map } from Std.Collections\n fn f(stock: Map<String, Int> where Length(0, 8)): Option<Int> = stock.set(\"key\", 1).get(\"key\")\n fn g(): Map<Int, Bool> = [:]\n fn h(): Int = [\"a\": 1].keys.length\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n fn f(stock: Map<String, Int>): Int = 0\n}",
        "`Map` needs `use { Map } from Std.Collections`",
    );

    assert_error(
        "context Shop {\n use { Map } from Std.Collections\n fn f(): Map<String, Int> = [\"a\": true]\n}",
        "expected `Int`, found `Bool`",
    );

    assert_error(
        "context Shop {\n use { Map } from Std.Collections\n fn f(): Int = [:].length\n}",
        "an empty map needs its type from where it goes, such as `let stock: Map<String, Int> = [:]`",
    );
}

#[test]
fn checks_generics_through_their_instances() {
    let source = "context Shop {\n value Pair<A, B> { first: A  second: B }\n fn swap<A, B>(pair: Pair<A, B>): Pair<B, A> = Pair { first: pair.second, second: pair.first }\n fn f(): Pair<String, Int> = swap(Pair { first: 1, second: \"a\" })\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n value Pair<A, B> { first: A  second: B }\n fn f(pair: Pair<Int>): Int = 0\n}",
        "`Pair` takes 2 type arguments, for `Pair<A, B>`, found 1",
    );

    assert_error(
        "context Shop {\n fn first<Item>(items: List<Item>): Option<Item> = items.first\n}",
        "a type parameter is a single capital letter, such as `T`, not `Item`",
    );

    assert_error(
        "context Shop {\n fn none<T>(): Option<T> = None\n fn f(): Int = {\n let x = none()\n 0\n }\n}",
        "cannot work out `T` for `none` from what it is given",
    );

    // A template's body is checked as each instance, so a mistake shows when it is used.
    assert_error(
        "context Shop {\n fn twice<T>(x: T): T = x + x\n fn f(): Bool = twice(true)\n}",
        "cannot do arithmetic on `Bool`",
    );
}
