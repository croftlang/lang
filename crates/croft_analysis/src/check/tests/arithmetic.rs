//! Arithmetic with no answer in 32 bits: a whole number has exactly 32 bits,
//! so `+`, `-` and `*` past them, and `/` or `%` by zero, are errors when the
//! values are known, and checked while running when they are not.

use super::{assert_error, errors};

/// A context with `extra` in it, and `Decimal` imported.
fn sums(extra: &str) -> String {
    format!(
        "context Sums {{\n\
         use {{ Decimal }} from Std.Numbers\n\
         const MOST: Int = 2147483647\n\
         {extra}\n}}"
    )
}

#[test]
fn adding_past_32_bits_is_an_error() {
    let source = sums("fn next(): Int = 2147483647 + 1");

    assert_error(
        &source,
        "`2147483647 + 1` gives 2147483648, which doesn't fit in 32 bits",
    );
}

#[test]
fn a_constant_past_32_bits_says_what_it_works_out() {
    let source = sums("fn next(): Int = MOST + 1");

    assert_error(
        &source,
        "`MOST + 1` works out `2147483647 + 1`, which gives 2147483648 and doesn't fit in 32 bits",
    );
}

#[test]
fn subtracting_and_multiplying_past_32_bits_are_errors() {
    let source = sums("fn low(): Int = -2147483648 - 1\n fn big(): Int = 65536 * 65536");

    assert_eq!(
        errors(&source),
        vec![
            "`-2147483648 - 1` gives -2147483649, which doesn't fit in 32 bits".to_string(),
            "`65536 * 65536` gives 4294967296, which doesn't fit in 32 bits".to_string(),
        ]
    );
}

#[test]
fn the_lowest_whole_number_can_be_written() {
    let source = sums("fn lowest(): Int = -2147483648\n fn highest(): Int = MOST");

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_literal_past_32_bits_is_an_error() {
    let source = sums("fn big(): Int = 3000000000");

    assert_error(
        &source,
        "`3000000000` doesn't fit in 32 bits, the size of an `Int`",
    );
}

#[test]
fn negating_the_lowest_whole_number_is_an_error() {
    let source = sums("const LOWEST: Int = -2147483648\n fn flipped(): Int = -LOWEST");

    assert_error(
        &source,
        "`-LOWEST` works out `-(-2147483648)`, which gives 2147483648 and doesn't fit in 32 bits",
    );
}

#[test]
fn dividing_the_lowest_whole_number_by_minus_one_is_an_error() {
    let source = sums("fn quotient(): Int = -2147483648 / -1");

    assert_error(
        &source,
        "`-2147483648 / -1` gives 2147483648, which doesn't fit in 32 bits",
    );
}

#[test]
fn dividing_by_a_known_zero_is_an_error_whatever_is_divided() {
    let source = sums(
        "fn half(n: Int): Int = n / 0\n const NONE: Int = 2 - 2\n fn rest(n: Int): Int = n % NONE",
    );

    assert_eq!(
        errors(&source),
        vec![
            "`n / 0` divides by zero".to_string(),
            "`n % NONE` divides by zero".to_string(),
        ]
    );
}

#[test]
fn a_known_let_is_worked_out() {
    let source = sums("fn next(): Int = {\n let most = MOST\n most * 2\n }");

    assert_error(
        &source,
        "`most * 2` works out `2147483647 * 2`, which gives 4294967294 and doesn't fit in 32 bits",
    );
}

#[test]
fn a_call_with_known_arguments_that_divides_by_zero_is_an_error() {
    let source = sums("fn divide(a: Int, b: Int): Int = a / b\n fn ten(): Int = divide(10, 0)");

    assert_error(&source, "`divide(10, 0)` divides by zero, in `10 / 0`");
}

#[test]
fn a_call_with_known_arguments_past_32_bits_is_an_error() {
    let source = sums("fn grow(a: Int): Int = a * 100000\n fn grown(): Int = grow(grow(1))");

    assert_error(
        &source,
        "`grow(100000)` works out `100000 * 100000`, which gives 10000000000 and doesn't fit in 32 bits",
    );
}

#[test]
fn a_fault_in_a_body_is_not_reported_again_at_each_call() {
    let source = sums("fn next(): Int = 2147483647 + 1\n fn again(): Int = next()");

    assert_error(
        &source,
        "`2147483647 + 1` gives 2147483648, which doesn't fit in 32 bits",
    );
}

#[test]
fn arithmetic_that_fits_is_accepted() {
    let source = sums(
        "fn grow(a: Int): Int = a * 1000\n\
         fn grown(): Int = grow(grow(2))\n\
         fn edge(): Int = MOST - 1 + 1\n\
         fn rest(): Int = -2147483648 % -1\n\
         fn shifted(): Int = 1 << 31",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_decimal_past_32_bits_of_hundredths_is_an_error() {
    let source = sums(
        "const PRICE: Decimal<2> = 21474836.47\n\
         fn dearer(): Decimal<2> = PRICE + 0.01\n\
         fn free(): Decimal<2> = PRICE / 0",
    );

    assert_eq!(
        errors(&source),
        vec![
            "`PRICE + 0.01` gives 21474836.48, which doesn't fit in 32 bits as a `Decimal<2>`"
                .to_string(),
            "`PRICE / 0` divides by zero".to_string(),
        ]
    );
}

#[test]
fn a_decimal_literal_past_32_bits_of_hundredths_is_an_error() {
    let source = sums("fn price(): Decimal<2> = 30000000.00");

    assert_error(
        &source,
        "`30000000.00` doesn't fit in 32 bits as a `Decimal<2>`",
    );
}

#[test]
fn a_range_past_32_bits_is_an_error() {
    let source = sums("value Score { points: Int where Range(0, 4000000000) }");

    assert_error(
        &source,
        "`4000000000` doesn't fit in 32 bits, the size of an `Int`",
    );
}

#[test]
fn a_message_keeps_the_brackets_written() {
    let source = sums("fn rest(n: Int): Int = n % (2 - 2)");

    assert_error(&source, "`n % (2 - 2)` divides by zero");
}
