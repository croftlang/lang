use dn_syntax::{parse, SourceId};

use crate::manifest::Manifest;

const GAME: &str = "context Game {\n\
    port Screen {\n fn clear(): Unit\n}\n\
    port Clock {\n fn tick(): Unit\n}\n\
    adapter TestScreen for Screen = {\n fn clear() = Unit\n}\n\
    adapter TestClock for Clock = {\n fn tick() = Unit\n}\n\
    service Main(): Unit {\n calls Screen\n} = Unit\n\
    service Turn(count: Int): Unit = Unit\n}";

/// Checks the game with a `dn.toml`, giving each error with its line in the file.
fn check_with(toml: &str) -> Vec<String> {
    let manifest =
        Manifest::parse(toml, SourceId(1)).unwrap_or_else(|error| panic!("{}", error.message));
    let file = parse(GAME, SourceId(0)).expect("valid syntax");
    match crate::check(vec![file], Some(manifest)) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|e| format!("{}: {}", e.span.line, e.message))
            .collect(),
    }
}

/// `[package]`, then `extra` from line 4 on.
fn toml(extra: &str) -> String {
    format!("[package]\nname = \"game\"\n\n{extra}")
}

#[test]
fn accepts_a_target_that_fills_every_port() {
    let file =
        toml("[target.test]\nmain = \"Main\"\n\n[target.test.adapters]\nScreen = \"TestScreen\"\n");
    assert_eq!(check_with(&file), Vec::<String>::new());
}

#[test]
fn reports_a_port_without_an_adapter() {
    let file = toml("[target.test]\nmain = \"Main\"\n");
    assert_eq!(
        check_with(&file),
        vec!["4: target `test` has no adapter for port `Screen`"]
    );
}

#[test]
fn reports_an_adapter_for_the_wrong_port() {
    let file =
        toml("[target.test]\nmain = \"Main\"\n\n[target.test.adapters]\nScreen = \"TestClock\"\n");
    assert_eq!(
        check_with(&file),
        vec!["8: adapter `TestClock` is for port `Clock`, not `Screen`"]
    );
}

#[test]
fn checks_what_each_target_runs() {
    let adapters = "\n[target.test.adapters]\nScreen = \"TestScreen\"\n";
    let file = toml(&format!("[target.test]\nmain = \"Turn\"\n{adapters}"));
    assert_eq!(
        check_with(&file),
        vec!["5: the main service `Turn` must take no parameters"]
    );
    let file = toml(&format!("[target.test]\nmain = \"Start\"\n{adapters}"));
    assert_eq!(
        check_with(&file),
        vec!["5: cannot find a service named `Start`"]
    );
    let file = toml("[target.test.adapters]\nScreen = \"TestScreen\"\n");
    assert_eq!(check_with(&file), vec!["4: target `test` has nothing to run: declare a `domain`, or name a service with `main`"]);
}

#[test]
fn reports_unknown_keys_in_the_file() {
    let error =
        Manifest::parse("[package]\nname = \"game\"\nmain = \"Main\"\n", SourceId(1)).unwrap_err();
    assert_eq!(error.span.line, 3);
    assert!(
        error.message.contains("unknown field `main`"),
        "{}",
        error.message
    );
}
