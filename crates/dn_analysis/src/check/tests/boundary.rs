use super::{assert_error, dungeon, errors};

const SCREEN: &str = "port Screen {\n fn clear(): Unit\n fn draw(at: Position): Unit\n}";

fn with_adapter(operations: &str) -> String {
    dungeon(&format!(
        "{SCREEN}\nadapter TestScreen for Screen on target test {{\n{operations}\n}}"
    ))
}

#[test]
fn checks_adapters_provide_their_port() {
    let complete = "fn clear() = Unit\nfn draw(at) = Unit";
    assert_eq!(errors(&with_adapter(complete)), Vec::<String>::new());
    assert_error(
        &with_adapter("fn clear() = Unit"),
        "adapter `TestScreen` does not provide `draw` from port `Screen`",
    );
    let extra = format!("{complete}\nfn beep() = Unit");
    assert_error(
        &with_adapter(&extra),
        "port `Screen` has no operation `beep`",
    );
    let wrong = "fn clear() = Unit\nfn draw() = Unit";
    assert_error(
        &with_adapter(wrong),
        "`draw` takes 1 parameters in port `Screen`, found 0",
    );
}

#[test]
fn checks_ports_only_declare_operations() {
    let source = dungeon("port Screen {\n fn clear(): Unit = Unit\n}");
    assert_error(
        &source,
        "a port only declares `clear`; an adapter gives it a body",
    );
}

#[test]
fn checks_services_use_ports() {
    let source = dungeon(&format!(
        "{SCREEN}\nservice Play(): Unit {{\n uses Screen, Position\n}} = Unit"
    ));
    assert_error(
        &source,
        "expected a port or repository, but `Position` is a value",
    );
    assert_error(
        &dungeon("fn f(): Int {\n uses Screen\n} = 3"),
        "only a service can use ports",
    );
}

#[test]
fn checks_publish_and_use() {
    let scoring = |names: &str| format!("context Scoring {{\n use {{ {names} }} from Dungeon\n}}");
    let dungeon = dungeon("publish { RoomId }");
    let checked = |names: &str| super::check_files(&[&dungeon, &scoring(names)]);
    assert!(checked("RoomId").is_ok());
    assert_eq!(
        checked("Position").err().unwrap(),
        vec!["context `Dungeon` does not publish `Position`"]
    );
    assert_error(
        &super::dungeon("publish { Door }"),
        "cannot publish `Door`: context `Dungeon` does not declare it",
    );
}

#[test]
fn checks_the_application_binds_every_port() {
    let adapter = "adapter TestScreen for Screen on target test {\n fn clear() = Unit\n fn draw(at) = Unit\n}";
    let service = "service Play(): Unit {\n uses Screen\n} = Unit";
    let context = dungeon(&format!("{SCREEN}\n{adapter}\n{service}"));
    let application = |bindings: &str| {
        format!("application Game {{\n start Play\n target test {{\n{bindings}\n }}\n target zx {{\n{bindings}\n }}\n}}")
    };
    let errors = super::check_files(&[&context, &application("Screen -> TestScreen")])
        .err()
        .unwrap();
    assert_eq!(
        errors,
        vec!["adapter `TestScreen` is for target `test`, not `zx`"]
    );
    let errors = super::check_files(&[&context, &application("")])
        .err()
        .unwrap();
    assert!(errors.contains(&"target `test` has no adapter for port `Screen`".to_string()));
}
