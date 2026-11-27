//! The functions of `Std.Numbers`: `clamp`, `pow`, `sign` and the checked
//! arithmetic that gives an `Option`, beside `max`, `min` and `abs`. On known
//! values they are worked out, so what they give is checked against
//! constraints, and one with no answer in 32 bits is an error.

use super::{assert_error, errors};

/// A context with `extra` in it, and every number function imported.
fn numbers(extra: &str) -> String {
    format!(
        "context Sums {{\n\
         import {{ Decimal, abs, max, min, clamp, pow, sign, checked_add, checked_subtract, checked_multiply }} from Std.Numbers\n\
         const MOST: Int = 2147483647\n\
         fn heal(amount: Int where Range(1, 50)): Int = amount\n\
         {extra}\n}}"
    )
}

#[test]
fn each_function_needs_its_import() {
    let source = "context Sums {\n fn f(a: Int): Int = clamp(a, 0, 9) + pow(a, 2) + sign(a)\n}";

    assert_eq!(
        errors(source),
        vec![
            "`clamp` needs `import { clamp } from Std.Numbers`".to_string(),
            "`pow` needs `import { pow } from Std.Numbers`".to_string(),
            "`sign` needs `import { sign } from Std.Numbers`".to_string(),
        ]
    );
}

#[test]
fn the_functions_take_and_give_these_types() {
    let source = numbers(
        "fn kept(a: Int): Int = clamp(a, 0, 100)\n\
         fn price(a: Decimal<2>): Decimal<2> = clamp(a, 0.50, 9.99)\n\
         fn squared(a: Int): Int = pow(a, 2)\n\
         fn direction(a: Int): Int = sign(a)\n\
         fn owed(a: Decimal<2>): Int = sign(a)\n\
         fn total(a: Int, b: Int): Option<Int> = checked_add(a, b)\n\
         fn left(a: Int, b: Int): Option<Int> = checked_subtract(a, b)\n\
         fn area(a: Int, b: Int): Int = checked_multiply(a, b).unwrap_or(0)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn the_functions_report_wrong_arguments() {
    let source = numbers(
        "fn a(): Int = clamp(1, 2)\n\
         fn b(): Int = pow(2.5, 2)\n\
         fn c(): Int = checked_add(1, 2)\n\
         fn d(): Int = sign(true)",
    );

    assert_eq!(
        errors(&source),
        vec![
            "`clamp` takes 3 arguments, found 2".to_string(),
            "expected `Int`, found `Float`".to_string(),
            "expected `Int`, found `Option<Int>`".to_string(),
            "`sign` needs a number, found `Bool`".to_string(),
        ]
    );
}

#[test]
fn a_known_power_past_32_bits_is_an_error() {
    let source = numbers("fn big(): Int = pow(10, 10)\n fn huge(): Int = pow(2, 200)");

    assert_eq!(
        errors(&source),
        vec![
            "`pow(10, 10)` gives 10000000000, which doesn't fit in 32 bits".to_string(),
            "`pow(2, 200)` doesn't fit in 32 bits".to_string(),
        ]
    );
}

#[test]
fn a_known_negative_exponent_is_an_error() {
    let source = numbers("fn half(): Int = pow(2, -1)\n fn any(n: Int): Int = pow(n, -2)");

    assert_eq!(
        errors(&source),
        vec![
            "`pow(2, -1)` has an exponent below zero".to_string(),
            "`pow(n, -2)` has an exponent below zero".to_string(),
        ]
    );
}

#[test]
fn a_known_low_above_the_high_is_an_error() {
    let source = numbers("fn kept(a: Int): Int = clamp(a, 10, 0)");

    assert_error(&source, "`clamp(a, 10, 0)` has a low above its high");
}

#[test]
fn the_known_absolute_value_of_the_lowest_number_is_an_error() {
    let source = numbers("fn far(): Int = abs(-2147483647 - 1)");

    assert_error(
        &source,
        "`abs(-2147483647 - 1)` works out `abs(-2147483648)`, which gives 2147483648 and doesn't fit in 32 bits",
    );
}

#[test]
fn a_call_that_works_out_a_power_past_32_bits_is_an_error() {
    let source = numbers(
        "fn power(n: Int): Int = pow(n, 20)\n\
         fn small(): Int = power(1)\n\
         fn big(): Int = power(10)",
    );

    assert_error(
        &source,
        "`power(10)` works out `pow(10, 20)`, which gives 100000000000000000000 and doesn't fit in 32 bits",
    );
}

#[test]
fn a_call_that_clamps_with_a_low_above_the_high_is_an_error() {
    let source = numbers("fn kept(low: Int): Int = clamp(5, low, 9)\n fn bad(): Int = kept(10)");

    assert_error(
        &source,
        "`kept(10)` works out `clamp(5, 10, 9)`, which has a low above its high",
    );
}

#[test]
fn known_answers_are_checked_against_constraints() {
    let source = numbers(
        "fn a(): Int = heal(clamp(70, 1, 50))\n\
         fn b(): Int = heal(clamp(70, 1, 60))\n\
         fn c(): Int = heal(sign(-5))\n\
         fn d(): Int = heal(pow(2, 5))\n\
         fn e(): Int = heal(pow(2, 6))\n\
         fn f(): Int = heal(max(3, abs(-80)))\n\
         fn g(): Int = heal(min(3, abs(-80)))",
    );

    assert_eq!(
        errors(&source),
        vec![
            "`heal` is given `60` for `amount`, which breaks `Range(1, 50)`".to_string(),
            "`heal` is given `-1` for `amount`, which breaks `Range(1, 50)`".to_string(),
            "`heal` is given `64` for `amount`, which breaks `Range(1, 50)`".to_string(),
            "`heal` is given `80` for `amount`, which breaks `Range(1, 50)`".to_string(),
        ]
    );
}

#[test]
fn known_checked_arithmetic_is_worked_out() {
    let source = numbers(
        "const SUM: Int = checked_add(MOST, 1).unwrap_or(-1)\n\
         const PRODUCT: Int = checked_multiply(5, 6).unwrap_or(0)\n\
         const LEFT: Int = checked_subtract(-2147483648, 1).unwrap_or(99)\n\
         fn a(): Int = heal(SUM)\n\
         fn b(): Int = heal(PRODUCT)\n\
         fn c(): Int = heal(LEFT)\n\
         fn d(): Bool = checked_add(1, 2) == Some(3)",
    );

    assert_eq!(
        errors(&source),
        vec![
            "`heal` is given `-1` for `amount`, which breaks `Range(1, 50)`".to_string(),
            "`heal` is given `99` for `amount`, which breaks `Range(1, 50)`".to_string(),
        ]
    );
}

#[test]
fn a_constraint_can_ask_whether_checked_arithmetic_fits() {
    let source = numbers(
        "constraint Squares(n: Int) = checked_multiply(n, n).is_some\n\
         fn square(n: Int where Squares): Int = n\n\
         fn small(): Int = square(46340)\n\
         fn big(): Int = square(46341)",
    );

    assert_error(
        &source,
        "`square` is given `46341` for `n`, which breaks `Squares`",
    );
}

#[test]
fn checked_arithmetic_past_32_bits_is_not_an_error() {
    let source = numbers(
        "fn a(): Option<Int> = checked_add(MOST, 1)\n\
         fn b(): Option<Int> = checked_multiply(65536, 65536)\n\
         fn c(): Option<Int> = checked_subtract(-2147483648, 1)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn powers_and_clamps_that_fit_are_accepted() {
    let source = numbers(
        "fn a(): Int = pow(-2, 31)\n\
         fn b(): Int = pow(2, 30) - 1 + pow(2, 30)\n\
         fn c(): Int = pow(0, 0) + pow(1, 2147483647) + pow(-1, 2147483647)\n\
         fn d(): Int = clamp(5, 5, 5)\n\
         fn e(): Int = abs(-2147483647)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}
