//! Generates Rust for the example project, then compiles and runs tests
//! against it with rustc, so we know the output is valid Rust that behaves.

use std::path::PathBuf;
use std::process::Command;

use dn_codegen::generate_rust;
use dn_syntax::{parse, SourceId};

const EXAMPLE: &str = include_str!("../../../examples/invoicing/invoicing.dn");

const BEHAVIOUR_TESTS: &str = r#"
#[cfg(test)]
mod behaviour {
    use super::*;

    fn line(description: &str, amount: i64) -> Line {
        Line {
            description: description.to_string(),
            price: Money { amount, currency: Currency::Gbp },
        }
    }

    fn invoice(lines: Vec<Line>, issue_day: i64, due_day: i64) -> Invoice {
        Invoice {
            id: InvoiceId { value: "INV-1".to_string() },
            lines,
            status: InvoiceStatus::Draft,
            issue_day,
            due_day,
        }
    }

    #[test]
    fn valid_invoice_passes() {
        assert_eq!(invoice(vec![line("Lesson", 2500)], 1, 30).validate(), Ok(()));
    }

    #[test]
    fn field_constraint_is_enforced() {
        let error = invoice(vec![], 1, 30).validate().unwrap_err();
        assert_eq!(error.constraint, "NonEmpty");
        assert_eq!(error.path, "Invoice.lines");
    }

    #[test]
    fn nested_structs_are_validated() {
        let error = invoice(vec![line("Lesson", 0)], 1, 30).validate().unwrap_err();
        assert_eq!(error.path, "Money.amount");
    }

    #[test]
    fn struct_constraint_is_enforced() {
        let error = invoice(vec![line("Lesson", 100)], 30, 1).validate().unwrap_err();
        assert_eq!(error.constraint, "DueAfterIssue");
    }

    #[test]
    fn constraint_with_argument_is_enforced() {
        let long = "x".repeat(201);
        let error = line(&long, 100).validate().unwrap_err();
        assert_eq!(error.constraint, "MaxLength");
    }
}
"#;

#[test]
fn example_compiles_and_behaves() {
    let module = parse(EXAMPLE, SourceId(0)).expect("example parses");
    let program = dn_analysis::check(vec![module]).expect("example checks");
    let source = generate_rust(&program) + BEHAVIOUR_TESTS;

    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("generated_code");
    std::fs::create_dir_all(&dir).unwrap();
    let source_path = dir.join("invoicing.rs");
    let binary_path = dir.join("invoicing_tests");
    std::fs::write(&source_path, &source).unwrap();

    let compiled = Command::new("rustc")
        .args(["--edition", "2021", "--test", "-D", "warnings"])
        .arg(&source_path)
        .arg("-o")
        .arg(&binary_path)
        .output()
        .expect("rustc runs");
    assert!(
        compiled.status.success(),
        "generated code failed to compile:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let ran = Command::new(&binary_path).output().expect("tests run");
    assert!(
        ran.status.success(),
        "generated code tests failed:\n{}",
        String::from_utf8_lossy(&ran.stdout)
    );
}
