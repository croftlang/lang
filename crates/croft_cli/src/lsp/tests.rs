//! The server's answers for the logistics fixture, as an editor would ask.

use std::path::{Path, PathBuf};

use super::lookup::{self, Found};
use super::session::{ProjectState, Session};

/// The folder of the logistics fixture.
fn logistics() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/logistics")
}

/// Checks the logistics fixture with `file` changed to `text` when given.
fn checked(file: &str, text: Option<&str>) -> (Session, PathBuf) {
    let root = logistics().canonicalize().expect("the example exists");
    let path = root.join(file);
    let mut session = Session::new(Some(root));

    if let Some(text) = text {
        session.open(path.clone(), text.to_string());
    }

    session.refresh(&path).expect("the project loads");

    (session, path)
}

/// What is under the first `word` on the first line containing `line`.
fn found_at<'a>(state: &'a ProjectState, path: &Path, line: &str, word: &str) -> Option<Found<'a>> {
    let file = state.file_at(path)?;

    let (index, text) = file
        .text
        .lines()
        .enumerate()
        .find(|(_, l)| l.contains(line))?;

    let column = text.find(word)? as u32 + 2;

    lookup::find_at(state, file.id, index as u32 + 1, column)
}

#[test]
fn hover_shows_a_commands_signature_and_doc() {
    let (session, path) = checked("shipping/booking.croft", None);
    let state = session.project_for(&path).expect("checked");
    let found = found_at(state, &path, "issue AssignToRoute", "AssignToRoute")
        .expect("AssignToRoute is found");
    let hover = lookup::hover(state, &found).expect("a hover");

    assert!(
        hover.contains("command AssignToRoute(chosen: Itinerary)"),
        "{hover}"
    );

    assert!(hover.contains("Plans the cargo's legs"), "{hover}");

    let (span, _) = lookup::definition(state, &found).expect("a definition");
    let file = state.file(span.source).expect("a project file");
    assert!(file.path.ends_with("shipping/cargo/commands.croft"));
}

#[test]
fn a_called_port_goes_to_its_port() {
    let (session, path) = checked("shipping/booking.croft", None);
    let state = session.project_for(&path).expect("checked");
    let found = found_at(state, &path, "Clock.today()", "Clock").expect("Clock is found");
    let (span, _) = lookup::definition(state, &found).expect("a definition");
    let file = state.file(span.source).expect("a project file");
    assert!(file.path.ends_with("shipping/ports.croft"));
}

#[test]
fn a_file_that_stops_parsing_keeps_its_last_tree() {
    let (mut session, path) = checked("shipping/booking.croft", None);
    let broken = std::fs::read_to_string(&path).unwrap() + "\nfn broken(";
    session.open(path.clone(), broken);
    session.refresh(&path).expect("the project loads");

    let state = session.project_for(&path).expect("checked");
    assert_eq!(state.diagnostics.len(), 1, "only the syntax error");

    let found = found_at(state, &path, "Some(report) ->", "RegisterHandlingEvent");
    assert!(found.is_some(), "RegisterHandlingEvent is still found");
}

#[test]
fn errors_point_at_the_file_they_are_in() {
    let original = std::fs::read_to_string(logistics().join("shipping/booking.croft")).unwrap();
    let text = original.replace("Cargos.save(routed)?", "Cargos.save(rotued)?");
    let (session, path) = checked("shipping/booking.croft", Some(&text));
    let state = session.project_for(&path).expect("checked");
    let file = state.file_at(&path).expect("booking.croft is checked");

    let messages: Vec<_> = state
        .diagnostics
        .iter()
        .filter(|d| d.span.source == file.id)
        .map(|d| d.message.as_str())
        .collect();

    assert_eq!(messages, ["cannot find `rotued` here"]);
}

/// The hover text for the first `word` on the first line containing `line`.
fn hover_at(state: &ProjectState, path: &Path, line: &str, word: &str) -> String {
    let found = found_at(state, path, line, word).unwrap_or_else(|| panic!("{word} is found"));
    lookup::hover(state, &found).expect("a hover")
}

/// The file where the first `word` on the first line containing `line` is declared.
fn definition_file(state: &ProjectState, path: &Path, line: &str, word: &str) -> PathBuf {
    let found = found_at(state, path, line, word).unwrap_or_else(|| panic!("{word} is found"));
    let (span, _) = lookup::definition(state, &found).expect("a definition");

    match state.file(span.source) {
        Some(file) => file.path.clone(),
        None => super::standard::path_of(span.source).expect("a standard file"),
    }
}

#[test]
fn fields_and_operations_after_a_dot() {
    let (session, path) = checked("shipping/booking.croft", None);
    let state = session.project_for(&path).expect("checked");
    let line = "Console.print(transport_text(cargo.delivery))";
    let hover = hover_at(state, &path, line, "delivery");
    assert!(hover.contains("delivery: Delivery"), "{hover}");

    assert!(
        hover.contains("*A field of `Cargo` in `Shipping`*"),
        "{hover}"
    );

    let file = definition_file(state, &path, line, "delivery");
    assert!(file.ends_with("shipping/cargo/cargo.croft"), "{file:?}");

    let line = "completed_on: Clock.today()";
    let hover = hover_at(state, &path, line, "today");
    assert!(hover.contains("fn today("), "{hover}");
    assert!(hover.contains("*An operation of `Clock`"), "{hover}");

    let file = definition_file(state, &path, line, "today");
    assert!(file.ends_with("shipping/ports.croft"), "{file:?}");

    // `find` is the repository's own, with nowhere to go.
    let hover = hover_at(
        state,
        &path,
        "let cargo = Cargos.find(tracking_id)?",
        "find",
    );

    assert!(hover.contains("fn find(…): Result<Cargo,"), "{hover}");
}

#[test]
fn an_aggregate_is_shown_with_what_it_holds() {
    let (session, path) = checked("shipping/booking.croft", None);
    let state = session.project_for(&path).expect("checked");
    let line = "let cargo = Cargo.Book(tracking_id: SAMPLE_CARGO, route: route)?";
    let hover = hover_at(state, &path, line, "Cargo");

    assert!(
        hover.starts_with("```croft\naggregate Cargo {\n"),
        "{hover}"
    );

    assert!(hover.contains("\n    delivery: Delivery\n"), "{hover}");

    // Commands are written in another file, and shown without their bodies.
    assert!(
        hover.contains("\n    command AssignToRoute(chosen: Itinerary)\n"),
        "{hover}"
    );

    assert!(!hover.contains("set itinerary"), "{hover}");

    // Factories come before commands, also without their bodies.
    assert!(
        hover.contains("\n    factory Book(tracking_id: TrackingId, route: RouteSpecification)\n"),
        "{hover}"
    );

    assert!(!hover.contains("misdirected: false"), "{hover}");
}

#[test]
fn a_field_where_it_is_declared() {
    let path = logistics()
        .canonicalize()
        .unwrap()
        .join("shipping/cargo/cargo.croft");

    let (session, _) = checked("shipping/cargo/cargo.croft", None);
    let state = session.project_for(&path).expect("checked");
    let hover = hover_at(state, &path, "delivery: Delivery", "delivery");

    assert!(
        hover.contains("*A field of `Cargo` in `Shipping`*"),
        "{hover}"
    );
}

#[test]
fn a_standard_declaration_opens_a_file_of_its_own() {
    let (session, path) = checked("shipping/booking.croft", None);
    let state = session.project_for(&path).expect("checked");
    let file = definition_file(state, &path, "include StorageError", "StorageError");
    assert!(file.ends_with("Std.Storage.croft"), "{file:?}");

    let text = std::fs::read_to_string(&file).expect("the file is written");
    assert!(text.contains("port Storage"));
    assert!(super::standard::source_at(&file).is_some());
}

/// What is offered at the end of `typed`, written on a new line after `after`.
fn completions(after: &str, typed: &str) -> Vec<String> {
    let original = std::fs::read_to_string(logistics().join("shipping/booking.croft")).unwrap();
    let text = original.replacen(after, &format!("{after}\n    {typed}"), 1);
    // The file parsed when it was opened, before the typing began.
    let (mut session, path) = checked("shipping/booking.croft", None);
    session.open(path.clone(), text.clone());
    session.refresh(&path).expect("the project loads");

    let state = session.project_for(&path).expect("checked");
    let file = state.file_at(&path).expect("booking.croft is checked");

    let index = text
        .lines()
        .position(|l| l == format!("    {typed}"))
        .unwrap();

    let column = typed.chars().count() as u32 + 5;

    super::completion::complete(state, file.id, &text, index as u32 + 1, column)
        .into_iter()
        .map(|item| item.label)
        .collect()
}

#[test]
fn completes_what_comes_after_a_dot() {
    let after = "Console.print(transport_text(cargo.delivery))";
    let offered = completions(after, "cargo.");

    for name in ["route", "itinerary", "delivery"] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }

    let offered = completions(after, "cargo.route.dest");
    assert!(offered.iter().any(|o| o == "destination"), "{offered:?}");

    let offered = completions(after, "Cargos.");
    assert_eq!(offered, ["find", "save"]);

    let offered = completions(after, "Console.");
    assert!(offered.iter().any(|o| o == "print"), "{offered:?}");

    let offered = completions(after, "HandlingType.");
    assert_eq!(offered, ["Receive", "Load", "Unload", "Claim"]);

    let offered = completions(after, "Cargo.");

    for name in ["AssignToRoute", "RecordHandling", "NotFound"] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }
}

#[test]
fn completes_names_in_scope() {
    let offered = completions("Console.print(transport_text(cargo.delivery))", "ca");

    for name in [
        "cargo",
        "Cargos",
        "tracking_id",
        "Console",
        "transport_text",
        "Cargo",
        "let",
        "match",
        "Some",
    ] {
        assert!(offered.iter().any(|o| o == name), "{name} in {offered:?}");
    }
}

#[test]
fn offers_only_the_words_the_parser_reads() {
    let offered = completions("Console.print(transport_text(cargo.delivery))", "");

    for word in [
        "from", "as", "on", "for", "in", "when", "include", "contexts",
    ] {
        assert!(offered.iter().any(|o| o == word), "{word} in {offered:?}");
    }

    for word in ["start", "target"] {
        assert!(!offered.iter().any(|o| o == word), "{word} in {offered:?}");
    }
}

#[test]
fn a_context_shows_what_it_exports_and_goes_to_its_declaration() {
    let (session, path) = checked("logistics.croft", None);
    let state = session.project_for(&path).expect("checked");

    let hover = hover_at(state, &path, "from Shipping", "Shipping");
    assert!(
        hover.starts_with("```croft\ncontext Shipping {\n"),
        "{hover}"
    );
    assert!(hover.contains("SetUpBooking"), "{hover}");
    assert!(hover.contains("*A context*"), "{hover}");
    assert!(
        hover.contains("Shipping: booking cargo, routing it"),
        "{hover}"
    );

    let hover = hover_at(state, &path, "Routing -> Shipping", "Routing");
    assert!(hover.contains("context Routing"), "{hover}");
    assert!(hover.contains("Used by `Shipping`"), "{hover}");

    let file = definition_file(state, &path, "from Shipping", "Shipping");
    assert!(file.starts_with(logistics().canonicalize().unwrap().join("shipping")));
}

#[test]
fn a_built_in_port_is_shown_as_if_declared() {
    let text = std::fs::read_to_string(logistics().join("logistics.croft")).unwrap();

    let text = text
        .replace(
            "} from Shipping\n",
            "} from Shipping\n\n    calls { Console }\n",
        )
        .replace(
            "    Track(SAMPLE_CARGO)?\n",
            "    Track(SAMPLE_CARGO)?\n    Console.print(\"done\")\n",
        );

    let (session, path) = checked("logistics.croft", Some(&text));
    let state = session.project_for(&path).expect("checked");

    let hover = hover_at(state, &path, "calls { Console }", "Console");

    assert!(
        hover.contains(
            "port Console {\n    fn print(text: String): Unit\n    \
             fn print_error(text: String): Unit\n    \
             fn read_line(): Option<String>\n}"
        ),
        "{hover}"
    );

    assert!(
        hover.contains("Text to and from whoever runs the program."),
        "{hover}"
    );
    assert!(hover.contains("in `Std.Console`*"), "{hover}");

    let file = definition_file(state, &path, "calls { Console }", "Console");
    assert!(file.ends_with("Std.Console.croft"), "{}", file.display());

    let hover = hover_at(state, &path, "Console.print", "print");
    assert!(hover.contains("fn print(text: String): Unit"), "{hover}");
    assert!(
        hover.contains("Prints the text and starts a new line."),
        "{hover}"
    );

    let hover = hover_at(state, &path, "Console.print", "Console");
    assert!(
        hover.contains("Text to and from whoever runs the program."),
        "{hover}"
    );
}

#[test]
fn a_domain_is_shown_as_the_domain() {
    let (session, path) = checked("logistics.croft", None);
    let state = session.project_for(&path).expect("checked");
    let hover = hover_at(state, &path, "domain Logistics", "Logistics");

    assert!(
        hover.starts_with(
            "```croft\ndomain Logistics {\n    contexts {\n        Routing -> Shipping\n"
        ),
        "{hover}"
    );

    assert!(hover.contains("    import { SetUpBooking,"), "{hover}");
    assert!(
        hover.contains("A shipping company's cargo system"),
        "{hover}"
    );
    assert!(hover.contains("*The domain*"), "{hover}");
    assert!(!hover.contains("service"), "{hover}");
}

#[test]
fn a_built_in_constraint_or_function_says_what_it_does() {
    let (session, path) = checked("routing/routing.croft", None);
    let state = session.project_for(&path).expect("checked");

    let hover = hover_at(state, &path, "voyage: String where Length(4)", "Length");
    assert!(
        hover.contains("constraint Length(items: String or List<T>, low: Int, high: Int)"),
        "{hover}"
    );
    assert!(
        hover.contains("with one argument, means exactly 4"),
        "{hover}"
    );
    assert!(hover.contains("*Built in: it needs no import*"), "{hover}");

    let hover = hover_at(
        state,
        &path,
        "departs: Int where Range(0, MAX_DAY)",
        "Range",
    );
    assert!(
        hover.contains("constraint Range(number: Int, low: Int, high: Int)"),
        "{hover}"
    );

    let hover = hover_at(state, &path, "-> Some(Path", "Some(Path");
    assert!(hover.contains("fn Some(value: T): Option<T>"), "{hover}");

    let found = found_at(state, &path, "departs: Int where Range", "Range").expect("found");
    assert!(lookup::definition(state, &found).is_none());
}

#[test]
fn a_core_type_says_what_it_is() {
    let (session, path) = checked("routing/routing.croft", None);
    let state = session.project_for(&path).expect("checked");

    let hover = hover_at(state, &path, "voyage: String where Length(4)", "String");
    assert!(hover.starts_with("```croft\nString\n```"), "{hover}");
    assert!(
        hover.contains("A string with a `Length` holds that many"),
        "{hover}"
    );
    assert!(hover.contains("*Built in: it needs no import*"), "{hover}");

    let hover = hover_at(state, &path, "departs: Int where Range", "Int");
    assert!(
        hover.contains("A whole number of at least 32 bits."),
        "{hover}"
    );

    let hover = hover_at(state, &path, "edges: List<Edge>", "List");
    assert!(hover.starts_with("```croft\nList<T>\n```"), "{hover}");

    let line = "fn find_path(start: String, end: String, deadline: Int): Option<Path>";
    let hover = hover_at(state, &path, line, "Option");
    assert!(hover.contains("`Some(value)` or `None`"), "{hover}");

    // The type an argument names is still its own declaration.
    let hover = hover_at(state, &path, "edges: List<Edge>", "Edge>");
    assert!(hover.contains("value Edge {"), "{hover}");
}

#[test]
fn some_and_none_in_a_match_say_what_they_are() {
    let (session, path) = checked("routing/routing.croft", None);
    let state = session.project_for(&path).expect("checked");

    // The pattern, then the call after the arrow.
    let hover = hover_at(state, &path, "Some(edge) -> Some(Path", "Some(edge)");
    assert!(hover.contains("fn Some(value: T): Option<T>"), "{hover}");

    let hover = hover_at(state, &path, "Some(edge) -> Some(Path", "Some(Path");
    assert!(hover.contains("fn Some(value: T): Option<T>"), "{hover}");

    // `None -> None`: the pattern, then the value.
    let file = state.file_at(&path).expect("open");

    let (index, line) = (file.text.lines().enumerate())
        .find(|(_, l)| l.trim() == "None -> None")
        .expect("the line");

    for column in [line.find("None"), line.rfind("None")] {
        let column = column.expect("None") as u32 + 2;
        let found = lookup::find_at(state, file.id, index as u32 + 1, column).expect("found");
        let hover = lookup::hover(state, &found).expect("a hover");
        assert!(hover.contains("An `Option` that holds nothing."), "{hover}");
    }
}
