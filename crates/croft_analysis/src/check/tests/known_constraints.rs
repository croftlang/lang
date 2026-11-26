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
                  use { Decimal } from Std.Numbers\n\
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
                  publish { administer }\n\
                  constraint Overdose(milligrams: Int, limit: Int) = milligrams <= limit\n\
                  fn administer(milligrams: Int where Overdose(4000)): Int = milligrams\n}\n\
                  domain App {\n\
                  contexts { Clinic }\n\
                  use { administer } from Clinic\n\
                  calls Console\n} = {\n\
                  administer(5000)\n\
                  console.print(\"OK\")\n}";

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
                  service Treat(): Result<Unit, WardError> {\n calls Beds\n} = {\n\
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
                  service Start(): Result<Unit, WardError> {\n calls Beds\n} = {\n\
                  let bed = Bed.Admit(id: 1, given: 5000)?\n beds.save(bed)?\n}\n}";

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
         service Treat(): Result<Unit, WardError> {{\n calls Beds\n}} = {{\n\
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
