//! Tests for `Set` from `Std.Collections`, and the list methods `unique`,
//! `insert` and `fold`, whose lambda takes two parameters.

use super::{assert_error, errors};
use crate::built_in_members;
use crate::types::Type;

/// A context that uses `Set`, with `body` added to it.
fn with_sets(body: &str) -> String {
    format!("context Shop {{\n use {{ Set }} from Std.Collections\n{body}\n}}")
}

#[test]
fn checks_sets() {
    let source = with_sets(
        " fn f(seen: Set<Int> where Length(0, 8)): Bool = seen.add(3).remove(4).contains(3)\n \
         fn g(): Set<String> = [\"a\", \"b\"]\n \
         fn h(): Set<Int> = []\n \
         fn i(seen: Set<Int>): Int = seen.length + seen.items.length\n \
         fn j(seen: Set<Int>): Bool = seen.is_empty || seen == [1, 2]",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn needs_set_imported() {
    assert_error(
        "context Shop {\n fn f(seen: Set<Int>): Int = 0\n}",
        "`Set` needs `use { Set } from Std.Collections`",
    );
}

#[test]
fn checks_the_items_of_a_set() {
    assert_error(
        &with_sets(" fn f(): Set<Int> = [1, \"two\"]"),
        "expected `Int`, found `String`",
    );

    assert_error(
        &with_sets(" fn f(seen: Set<Int>): Set<Int> = seen.add(\"two\")"),
        "expected `Int`, found `String`",
    );

    assert_error(
        &with_sets(" fn f(seen: Set<Int>): Option<Int> = seen.get(0)"),
        "`Set<Int>` has no method `get`",
    );
}

#[test]
fn reports_an_item_listed_twice_in_a_set() {
    assert_error(
        &with_sets(" const TWO: Int = 2\n fn f(): Set<Int> = [1, 2, TWO]"),
        "`2` is in this set more than once, but a set holds each item once",
    );
}

#[test]
fn needs_a_length_on_a_set_that_set_changes() {
    let source = with_sets(
        " service Collect(extra: Int) = {\n  let seen: Set<Int> = []\n  set seen = seen.add(extra)\n }",
    );

    assert_error(
        &source,
        "`seen` is changed with `set`, so say how many items it can hold with a `Length`, \
         such as `let seen: Set<Int> where Length(0, 100) = ...`",
    );

    let source = with_sets(
        " service Collect(extra: Int) = {\n  let seen: Set<Int> where Length(0, 4) = []\n  \
         loop (let i where Range(1, 10)) {\n   set seen = seen.add(extra)\n  }\n }",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn checks_unique_and_insert() {
    let source = "context Shop {\n fn f(): List<Int> = [3, 1, 3].unique().insert(0, 9)\n}";
    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n fn f(): List<Int> = [1].insert(0, \"a\")\n}",
        "expected `Int`, found `String`",
    );

    assert_error(
        "context Shop {\n fn f(): List<Int> = [1].insert(-1, 2)\n}",
        "`insert` needs a place from 0 to the list's length, found -1",
    );
}

#[test]
fn checks_fold() {
    let source = "context Shop {\n \
                  fn total(prices: List<Int>): Int = prices.fold(0, (total, price) -> total + price)\n \
                  fn names(ids: List<Int>): String = ids.fold(\"\", (text, id) -> text + id.to_string())\n \
                  fn evens(ids: List<Int>): List<Int> = ids.fold([], (kept, id) -> kept + [id * 2])\n}";

    assert_eq!(errors(source), Vec::<String>::new());

    assert_error(
        "context Shop {\n fn f(ids: List<Int>): Int = ids.fold(0, id -> id)\n}",
        "the lambda passed to `fold` takes 2 parameters, such as \
         `(total, item) -> total + item`, found 1",
    );

    assert_error(
        "context Shop {\n fn f(ids: List<Int>): Int = ids.fold(0, (total, id) -> \"a\")\n}",
        "the lambda passed to `fold` must give back `Int`, found `String`",
    );

    assert_error(
        "context Shop {\n fn f(ids: List<Int>): Int = ids.fold(0)\n}",
        "`fold` takes 2 arguments, a start and a lambda such as `(total, item) -> total + item`",
    );

    assert_error(
        "context Shop {\n fn f(ids: List<Int>): Int = ids.fold([], (kept, id) -> kept).length\n}",
        "an empty list to start `fold` with needs its type from where the result goes, \
         such as `let names: List<String> = items.fold([], ...)`",
    );
}

#[test]
fn reports_a_lambda_with_the_wrong_number_of_parameters() {
    assert_error(
        "context Shop {\n fn f(ids: List<Int>): List<Int> = ids.filter((a, b) -> a > b)\n}",
        "the lambda passed to `filter` takes 1 parameter, such as `e -> e.id`, found 2",
    );
}

#[test]
fn offers_the_members_of_sets_and_the_new_list_methods() {
    let names = |ty: &Type| -> Vec<&str> {
        built_in_members(ty)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    };

    assert_eq!(
        names(&Type::set(Type::Int)),
        ["length", "is_empty", "items", "add", "remove", "contains"]
    );

    let list = names(&Type::list(Type::Int));

    for name in ["unique", "insert", "fold"] {
        assert!(list.contains(&name), "{name} in {list:?}");
    }
}
