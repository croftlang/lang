//! Constraints the compiler can work out: a value known when the program is
//! compiled, such as a literal or a constant, is checked against each
//! constraint it must keep, and a broken one is an error, not a runtime stop.

use super::{assert_error, errors};

/// A clinic whose doses must not be an overdose, with `extra` added inside it.
fn clinic(extra: &str) -> String {
    format!(
        "context Clinic {{\n\
         constraint Overdose(mg: Int, max: Int) = mg <= max\n\
         fn give(mg: Int where Overdose(4000)): Int = mg\n\
         {extra}\n}}"
    )
}

#[test]
fn a_literal_argument_that_breaks_a_constraint_is_an_error() {
    let source = clinic("fn treat(): Int = give(5000)");

    assert_error(
        &source,
        "`give` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_literal_argument_that_keeps_a_constraint_is_accepted() {
    let source = clinic("fn treat(): Int = give(4000)");

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_named_argument_is_checked_against_its_parameter() {
    let source = clinic("fn treat(): Int = give(mg: 5000)");

    assert_error(
        &source,
        "`give` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn arithmetic_on_constants_is_worked_out() {
    let source = clinic("const SAFE: Int = 4000\n fn treat(): Int = give(SAFE + 1)");

    assert_error(
        &source,
        "`give` is given `4001` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_built_in_range_is_checked() {
    let source = clinic(
        "fn heal(amount: Int where Range(1, 50)): Int = amount\n fn treat(): Int = heal(70)",
    );

    assert_error(
        &source,
        "`heal` is given `70` for `amount`, which breaks `Range(1, 50)`",
    );
}

#[test]
fn a_decimal_range_is_checked() {
    let source = "context Till {\n\
                  import { Decimal } from Std.Numbers\n\
                  fn pay(amount: Decimal<2> where Range(0.01, 99.99)): Decimal<2> = amount\n\
                  fn sell(): Decimal<2> = pay(120.50)\n}";

    assert_error(
        source,
        "`pay` is given `120.50` for `amount`, which breaks `Range(0.01, 99.99)`",
    );
}

#[test]
fn a_string_length_is_checked() {
    let source = clinic("fn admit(name: String where Length(1, 8)): String = name\n fn treat(): String = admit(\"Bartholomew\")");

    assert_error(
        &source,
        "`admit` is given `\"Bartholomew\"` for `name`, which breaks `Length(1, 8)`",
    );
}

#[test]
fn each_item_of_a_known_list_is_checked() {
    let source = clinic(
        "constraint Even(number: Int) = number % 2 == 0\n\
         fn count(numbers: List<Int> where each Even): Int = numbers.length\n\
         fn treat(): Int = count([2, 3])",
    );

    assert_error(
        &source,
        "`count` is given `[2, 3]` for `numbers`, which breaks `each Even`",
    );
}

#[test]
fn a_constraint_that_calls_another_is_worked_out() {
    let source = clinic(
        "constraint Safe(mg: Int) = Overdose(mg, 4000) && mg > 0\n\
         fn dose(mg: Int where Safe): Int = mg\n\
         fn treat(): Int = dose(0)",
    );

    assert_error(&source, "`dose` is given `0` for `mg`, which breaks `Safe`");
}

#[test]
fn a_value_only_known_while_running_is_left_to_runtime() {
    let source = clinic("fn treat(mg: Int): Int = give(mg * 2)");

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_call_from_the_domain_is_checked_against_the_context_constraint() {
    let source = "context Clinic {\n\
                  export { administer }\n\
                  constraint Overdose(milligrams: Int, limit: Int) = milligrams <= limit\n\
                  fn administer(milligrams: Int where Overdose(4000)): Int = milligrams\n}\n\
                  domain App {\n\
                  contexts { Clinic }\n\
                  import { administer } from Clinic\n\
                  calls { Console }\n} = {\n\
                  administer(5000)\n\
                  Console.print(\"OK\")\n}";

    assert_error(
        source,
        "`administer` is given `5000` for `milligrams`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_record_field_is_checked() {
    let source = clinic(
        "value Dose {\n mg: Int where Overdose(4000)\n}\n\
         fn treat(): Dose = Dose { mg: 5000 }",
    );

    assert_error(
        &source,
        "`Dose` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_field_changed_with_with_is_checked() {
    let source = clinic(
        "value Dose {\n mg: Int where Overdose(4000)\n}\n\
         fn double(dose: Dose): Dose = dose with { mg: 8000 }",
    );

    assert_error(
        &source,
        "`Dose` is given `8000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_single_value_is_checked() {
    let source = clinic(
        "value Score: Int where Range(0, 999)\n\
         fn top(): Score = Score(5000)",
    );

    assert_error(
        &source,
        "`Score` is given `5000` for `value`, which breaks `Range(0, 999)`",
    );
}

#[test]
fn a_let_name_holding_a_known_value_is_checked_where_it_is_used() {
    let source = clinic("fn treat(): Int = {\n let dose = 2500 * 2\n give(dose)\n}");

    assert_error(
        &source,
        "`give` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_command_argument_is_checked() {
    let source = "context Ward {\n\
                  identity BedId for Bed: Int where Range(1, 9)\n\
                  aggregate Bed {\n\
                  id: BedId\n\
                  given: Int\n\
                  error NotFound\n\
                  command Give(mg: Int where Range(1, 4000)) = {\n set given = given + mg\n }\n\
                  }\n\
                  repository Beds for Bed\n\
                  enum WardError {\n include Bed.Error\n include StorageError\n }\n\
                  service Treat(): Result<Unit, WardError> {\n calls { Beds }\n} = {\n\
                  issue Give(mg: 5000) on Bed(1)?\n}\n}";

    assert_error(
        source,
        "`Give` is given `5000` for `mg`, which breaks `Range(1, 4000)`",
    );
}

#[test]
fn a_factory_argument_is_checked() {
    let source = "context Ward {\n\
                  identity BedId for Bed: Int where Range(1, 9)\n\
                  aggregate Bed {\n\
                  id: BedId\n\
                  given: Int\n\
                  error NotFound\n\
                  factory Admit(id: BedId, given: Int where Range(0, 4000)) = Bed { id, given }\n\
                  }\n\
                  repository Beds for Bed\n\
                  enum WardError {\n include Bed.Error\n include StorageError\n }\n\
                  service Start(): Result<Unit, WardError> {\n calls { Beds }\n} = {\n\
                  let bed = Bed.Admit(id: 1, given: 5000)?\n Beds.save(bed)?\n}\n}";

    assert_error(
        source,
        "`Admit` is given `5000` for `given`, which breaks `Range(0, 4000)`",
    );
}

#[test]
fn an_enum_variant_field_is_checked() {
    let source = clinic(
        "enum Item {\n Coins(count: Int where Range(1, 99))\n Key\n}\n\
         fn reward(): Item = Coins(500)",
    );

    assert_error(
        &source,
        "`Coins` is given `500` for `count`, which breaks `Range(1, 99)`",
    );
}

#[test]
fn a_service_let_that_is_never_set_is_known() {
    let source =
        clinic("service Treat(): Unit = {\n let dose = 5000\n let given = give(dose)\n Unit\n}");

    assert_error(
        &source,
        "`give` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_service_let_that_a_loop_sets_is_left_to_runtime() {
    let source = clinic(
        "service Treat(): Unit = {\n\
         let dose = 1\n\
         loop {\n let given = give(dose)\n set dose = dose * 10\n if dose > 5000 {\n break\n }\n }\n\
         Unit\n}",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_result_worked_out_from_known_arguments_is_checked() {
    let source = clinic(
        "constraint Even(number: Int) = number % 2 == 0\n\
         fn half(number: Int): Int where Even = number / 2\n\
         fn treat(): Int = half(6)",
    );

    assert_error(&source, "`half(6)` gives back `3`, which breaks `Even`");
}

#[test]
fn a_function_body_is_worked_out_when_its_arguments_are_known() {
    let source = clinic(
        "fn double(mg: Int): Int = {\n let twice = mg * 2\n twice\n}\n\
         fn treat(): Int = give(double(2500))",
    );

    assert_error(
        &source,
        "`give` is given `5000` for `mg`, which breaks `Overdose(4000)`",
    );
}

#[test]
fn a_contract_line_broken_by_known_arguments_is_an_error() {
    let source = clinic(
        "fn average(numbers: List<Int>): Int {\n !numbers.is_empty\n} = numbers.sum(n -> n) / numbers.length\n\
         fn treat(): Int = average([])",
    );

    assert_error(&source, "`average([])` breaks `!numbers.is_empty`");
}

#[test]
fn a_contract_line_relating_parameters_is_worked_out() {
    let source = clinic(
        "fn move_bed(from: Int, to: Int): Int {\n from != to\n} = to\n\
         fn treat(): Int = move_bed(3, 3)",
    );

    assert_error(&source, "`move_bed(3, 3)` breaks `from != to`");
}

#[test]
fn a_contract_line_with_an_unknown_argument_is_left_to_runtime() {
    let source = clinic(
        "fn move_bed(from: Int, to: Int): Int {\n from != to\n} = to\n\
         fn treat(bed: Int): Int = move_bed(3, bed)",
    );

    assert_eq!(errors(&source), Vec::<String>::new());
}

/// A clinic with a `Dosage` whose least must not be above its most.
fn dosage(extra: &str) -> String {
    clinic(&format!(
        "constraint Ordered(dosage: Dosage) = dosage.least <= dosage.most\n\
         value Dosage {{\n least: Int\n most: Int\n invariant Ordered\n}}\n\
         {extra}"
    ))
}

#[test]
fn a_value_known_to_break_its_invariant_is_an_error() {
    let source = dosage("fn usual(): Dosage = Dosage { least: 20, most: 10 }");

    assert_error(
        &source,
        "`Dosage { least: 20, most: 10 }` breaks `Dosage`'s invariant `Ordered`",
    );
}

#[test]
fn a_value_that_keeps_its_invariant_is_accepted() {
    let source = dosage("fn usual(): Dosage = Dosage { least: 10, most: 20 }");

    assert_eq!(errors(&source), Vec::<String>::new());
}

#[test]
fn a_value_copied_with_with_is_held_to_its_invariant() {
    let source = dosage(
        "fn raised(): Dosage = {\n let usual = Dosage { least: 10, most: 20 }\n usual with { least: 30 }\n}",
    );

    assert_error(
        &source,
        "`Dosage { least: 30, most: 20 }` breaks `Dosage`'s invariant `Ordered`",
    );
}

#[test]
fn a_value_with_a_field_not_known_is_left_to_runtime() {
    let source = dosage("fn usual(most: Int): Dosage = Dosage { least: 20, most }");

    assert_eq!(errors(&source), Vec::<String>::new());
}

/// A ward whose `Give` command has `contract` between its signature and body.
fn ward(contract: &str, issued: &str) -> String {
    format!(
        "context Ward {{\n\
         identity BedId for Bed: Int where Range(1, 9)\n\
         aggregate Bed {{\n\
         id: BedId\n\
         given: Int\n\
         error NotFound\n\
         error Refused\n\
         command Give(mg: Int) {{\n {contract}\n}} = {{\n set given = given + mg\n }}\n\
         }}\n\
         repository Beds for Bed\n\
         enum WardError {{\n include Bed.Error\n include StorageError\n }}\n\
         service Treat(): Result<Unit, WardError> {{\n calls {{ Beds }}\n}} = {{\n\
         issue Give(mg: {issued}) on Bed(1)?\n}}\n}}"
    )
}

#[test]
fn a_command_contract_line_is_checked() {
    let source = ward("mg > 0", "0");

    assert_error(&source, "`Give(mg: 0)` breaks `mg > 0`");
}

#[test]
fn a_contract_line_with_an_error_is_a_failure_the_caller_handles() {
    let source = ward("mg > 0 or Refused", "0");

    assert_eq!(errors(&source), Vec::<String>::new());
}

/// A span whose `high` must be at least its `low`.
fn spans(extra: &str) -> String {
    clinic(&format!(
        "constraint AtLeast(number: Int, low: Int) = number >= low\n\
         fn span(low: Int, high: Int where AtLeast(low)): Int = high - low\n\
         {extra}"
    ))
}

#[test]
fn a_constraint_that_reads_another_parameter_is_checked_with_its_value() {
    assert_error(
        &spans("fn f(): Int = span(5, 2)"),
        "`span` is given `2` for `high`, which breaks `AtLeast(low)`, with `low` given `5`",
    );

    assert_error(
        &spans("fn f(): Int = span(high: 2, low: 5)"),
        "`span` is given `2` for `high`, which breaks `AtLeast(low)`, with `low` given `5`",
    );

    assert_eq!(
        errors(&spans("fn f(): Int = span(2, 5)")),
        Vec::<String>::new()
    );

    // With `low` only known while running, the check is left to then.
    assert_eq!(
        errors(&spans("fn f(n: Int): Int = span(n, 2)")),
        Vec::<String>::new()
    );
}

#[test]
fn a_built_in_constraint_can_read_other_parameters() {
    let source = clinic(
        "fn pick(low: Int, high: Int, middle: Int where Range(low + 1, high - 1)): Int = middle\n\
         fn f(): Int = pick(1, 5, 5)",
    );

    assert_error(
        &source,
        "`pick` is given `5` for `middle`, which breaks `Range(low + 1, high - 1)`, with `low` given `1` and `high` given `5`",
    );
}

#[test]
fn a_constraint_argument_must_name_something_in_scope() {
    assert_error(
        &clinic("fn span(low: Int, high: Int where Range(lowe, 10)): Int = high"),
        "cannot find `lowe` here",
    );

    assert_error(
        &clinic("fn sized(name: String, count: Int where Range(0, name)): Int = count"),
        "expected `Int`, found `String`",
    );

    assert_error(
        &clinic("fn top(low: Int): Int where Range(low, 10) = low"),
        "`Range(low, 10)` on the result reads the parameter `low`, which only a parameter's constraint can",
    );
}

#[test]
fn a_field_constraint_reads_only_constants() {
    assert_error(
        &clinic("value Box {\n size: Int\n count: Int where Range(0, size)\n}"),
        "cannot find `size` here",
    );

    assert_eq!(
        errors(&clinic(
            "const MOST: Int = 9\nvalue Box {\n count: Int where Range(0, MOST - 1)\n}"
        )),
        Vec::<String>::new()
    );
}

/// `outer` passes one more than it is given to `inner`, whose parameter
/// must be at most 10.
fn nested(extra: &str) -> String {
    clinic(&format!(
        "fn inner(n: Int where Range(0, 10)): Int = n\n\
         fn outer(n: Int): Int = inner(n + 1)\n\
         {extra}"
    ))
}

#[test]
fn a_constraint_broken_by_a_call_in_a_body_is_reported_at_the_call_that_leads_there() {
    assert_error(
        &nested("fn f(): Int = outer(10)"),
        "`outer(10)` calls `inner(11)`, whose `n` breaks `Range(0, 10)`",
    );

    assert_eq!(
        errors(&nested("fn f(): Int = outer(9)")),
        Vec::<String>::new()
    );

    // Only once, at the first call on the way.
    assert_error(
        &nested("fn middle(): Int = outer(10)\nfn f(): Int = middle()"),
        "`outer(10)` calls `inner(11)`, whose `n` breaks `Range(0, 10)`",
    );

    // A call that breaks it itself is reported as usual, once.
    assert_error(
        &nested("fn f(): Int = inner(11)"),
        "`inner` is given `11` for `n`, which breaks `Range(0, 10)`",
    );
}

#[test]
fn values_made_with_their_fields_and_guards_are_worked_out_through_calls() {
    let source = clinic(
        "value Day { number: Int }\n\
         fn inner(n: Int where Range(0, 10)): Int = n\n\
         fn next(day: Day): Int = {\n\
             if day.number > 5 {\n\
                 inner(day.number * 2)\n\
             }\n\
             inner(day.number)\n\
         }\n\
         fn early(): Int = next(Day { number: 3 })\n\
         fn late(): Int = next(Day { number: 7 })",
    );

    assert_error(
        &source,
        "`next(Day { number: 7 })` calls `inner(14)`, whose `n` breaks `Range(0, 10)`",
    );
}
