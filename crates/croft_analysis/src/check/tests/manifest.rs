use croft_syntax::{parse, SourceId};

use crate::manifest::Manifest;

const GAME: &str = "context Game {\n\
    port Screen {\n fn clear(): Unit\n}\n\
    port Clock {\n fn tick(): Unit\n}\n\
    adapter TestScreen for Screen = {\n fn clear() = Unit\n}\n\
    adapter TestClock for Clock = {\n fn tick() = Unit\n}\n\
    service Main(): Unit {\n calls Screen\n} = Unit\n\
    service Turn(count: Int): Unit = Unit\n}";

/// Checks the game with a `croft.toml`, giving each error with its line in the file.
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

const SHOP: &str = "context Shop {\n\
    identity ShelfId for Shelf: Int where Range(1, 9)\n\
    aggregate Shelf {\n id: ShelfId\n error NotFound\n factory Open(id: ShelfId) = Shelf { id }\n }\n\
    enum ShopError {\n include Shelf.Error\n include StorageError\n }\n\
    repository Shelves for Shelf\n\
    service Main(): Result<Unit, ShopError> {\n calls Shelves\n} = shelves.save(Shelf.Open(id: 1)?)\n}";

fn check_shop(adapters: &str) -> Vec<String> {
    check_source(SHOP, adapters)
}

fn check_source(source: &str, adapters: &str) -> Vec<String> {
    let text = toml(&format!(
        "[target.test]\nmain = \"Main\"\n\n[target.test.adapters]\n{adapters}"
    ));

    let manifest = Manifest::parse(&text, SourceId(1)).unwrap_or_else(|e| panic!("{}", e.message));
    let file = parse(source, SourceId(0)).expect("valid syntax");

    match crate::check(vec![file], Some(manifest)) {
        Ok(_) => Vec::new(),
        Err(errors) => errors
            .into_iter()
            .map(|e| format!("{}: {}", e.span.line, e.message))
            .collect(),
    }
}

#[test]
fn keeps_repositories_in_memory_or_in_files() {
    assert_eq!(check_shop(""), Vec::<String>::new());

    assert_eq!(
        check_shop("Shelves = \"MemoryStorage\"\n"),
        Vec::<String>::new()
    );

    assert_eq!(
        check_shop("Files = \"StdioFiles\"\nShelves = \"FileStorage\"\n"),
        Vec::<String>::new()
    );

    assert_eq!(
        check_shop("Shelves = \"FileStorage\"\n"),
        vec!["8: `FileStorage` saves through `Files`, so target `test` needs an adapter for `Files`, such as `Files = \"StdioFiles\"`"]
    );

    assert_eq!(
        check_shop("Shelves = \"StdioFiles\"\nFiles = \"StdioFiles\"\n"),
        vec!["8: `StdioFiles` is not an adapter for `Storage` or for `Shelves`, so it can't keep its aggregates"]
    );

    assert_eq!(
        check_shop("Shelves = \"Cloud\"\n"),
        vec!["8: cannot find an adapter named `Cloud`"]
    );
}

#[test]
fn keeps_in_storage_only_finds_saves_and_removes() {
    let shop = |operations: &str| {
        format!(
            "context Shop {{\n\
             identity ShelfId for Shelf: Int where Range(1, 9)\n\
             aggregate Shelf {{\n id: ShelfId\n error NotFound\n }}\n\
             enum Lookup {{\n include Shelf.NotFound\n include StorageError\n }}\n\
             repository Shelves for Shelf {{\n{operations}\n}}\n\
             service Main(): Result<Unit, StorageError> {{\n calls Shelves\n}} = Ok(Unit)\n}}"
        )
    };

    let kept = "fn find(id: ShelfId): Result<Shelf, Lookup>\n\
                fn find_kept(id: ShelfId): Result<Option<Shelf>, StorageError>\n\
                fn save(shelf: Shelf): Result<Unit, StorageError>\n\
                fn remove(id: ShelfId): Result<Unit, StorageError>";

    assert_eq!(check_source(&shop(kept), ""), Vec::<String>::new());

    let empty = "fn empty(): Result<List<Shelf>, StorageError>";

    assert_eq!(
        check_source(&shop(empty), "Shelves = \"MemoryStorage\"\n"),
        vec!["8: `Storage` can't provide `Shelves.empty`, so target `test` needs an adapter written for `Shelves`"]
    );

    assert_eq!(
        check_source(&shop(empty), ""),
        vec!["4: `Storage` can't provide `Shelves.empty`, so target `test` needs an adapter written for `Shelves`"]
    );
}

#[test]
fn needs_no_adapter_for_the_standard_console() {
    let source = "context Game {\n\
        service Main(): Unit {\n calls Console\n} = console.print(\"hi\")\n}";

    assert_eq!(check_source(source, ""), Vec::<String>::new());

    assert_eq!(
        check_source(source, "Console = \"Mine\"\n"),
        vec!["8: `Console` is built in, so it needs no adapter"]
    );
}

#[test]
fn a_port_of_the_programs_own_replaces_the_standard_one() {
    let source = "context Game {\n\
        port Console {\n fn beep(): Unit\n}\n\
        adapter Beeper for Console = {\n fn beep() = Unit\n}\n\
        service Main(): Unit {\n calls Console\n} = console.beep()\n}";

    assert_eq!(
        check_source(source, "Console = \"Beeper\"\n"),
        Vec::<String>::new()
    );

    assert_eq!(
        check_source(source, ""),
        vec!["4: target `test` has no adapter for port `Console`"]
    );
}
