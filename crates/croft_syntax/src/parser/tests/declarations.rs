//! Parser tests for declarations.

use super::{only_context, parse_error, parse_ok};
use crate::ast::{ContextItem, FunctionKind};

#[test]
fn reads_a_context_header_as_the_rest_of_the_file() {
    let file = parse_ok("context Dungeon\n\nconst MAX: Int = 16\nidentity RoomId for Room: Int");
    let context = only_context(&file);
    assert_eq!(context.name.name, "Dungeon");
    assert_eq!(context.items.len(), 2);
}

#[test]
fn reads_an_aggregate_header_as_the_rest_of_the_file() {
    let source = "context Dungeon\naggregate Room\n\nid: RoomId\nerror NotFound\n\
                  command Reset = {\n    set lives = 3\n}";
    let file = parse_ok(source);

    let [ContextItem::Aggregate(room)] = &only_context(&file).items[..] else {
        panic!("expected one aggregate");
    };

    assert_eq!(room.name.name, "Room");
    assert_eq!(room.errors.len(), 1);
    assert_eq!(room.commands[0].kind, FunctionKind::Command);
    assert!(room.commands[0].parameters.is_none());
}

#[test]
fn needs_braces_for_an_aggregate_inside_a_braced_context() {
    let message = parse_error("context Dungeon {\n    aggregate Room\n    error NotFound\n}");
    assert_eq!(message, "expected `{`, found `error`");
}

#[test]
fn reads_rules_on_fields_and_parameters() {
    let source = "context Dungeon {\n    value Room {\n        enemies: List<Enemy> where Length(0, 16), each Alive\n    }\n\
                  fn heal(amount: Int where Range(1, 50), times: Int): Int\n}";
    let file = parse_ok(source);
    let items = &only_context(&file).items;

    let ContextItem::Value(room) = &items[0] else {
        panic!("expected a value")
    };

    let constraints = &room.fields[0].constraints;
    assert_eq!(constraints.len(), 2);
    assert!(constraints[1].each);

    let ContextItem::Function(heal) = &items[1] else {
        panic!("expected a function")
    };

    let parameters = heal.parameters.as_ref().unwrap();
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0].constraints.len(), 1);
}

#[test]
fn reads_a_contract_with_constraints_and_calls() {
    let source = "context Dungeon {\n    service Play(id: RoomId): Unit {\n        calls { Keyboard, Screen }\n        \
                  id > 0 or NotFound\n    } = Unit\n}";
    let file = parse_ok(source);

    let ContextItem::Function(play) = &only_context(&file).items[0] else {
        panic!("expected a service")
    };

    assert_eq!(play.calls.len(), 2);
    assert_eq!(play.preconditions.len(), 1);
}

#[test]
fn reads_enums_with_data_and_includes() {
    let source = "context Dungeon {\n    enum GameError {\n        include Room.Error\n        \
                  Coins(count: Int where Range(1, 99))\n        Flask\n    }\n}";
    let file = parse_ok(source);

    let ContextItem::Enum(error) = &only_context(&file).items[0] else {
        panic!("expected an enum")
    };

    assert_eq!(error.includes[0].text(), "Room.Error");
    assert_eq!(error.variants.len(), 2);
    assert_eq!(error.variants[0].fields[0].constraints.len(), 1);
}

#[test]
fn reads_an_adapter_without_a_target() {
    let file = parse_ok("context Platform {\n    adapter NullScreen for Screen = {\n        fn clear() = Unit\n    }\n}");

    let ContextItem::Adapter(adapter) = &only_context(&file).items[0] else {
        panic!("expected an adapter")
    };

    assert_eq!(adapter.port.text(), "Screen");
    assert_eq!(adapter.operations.len(), 1);
}

#[test]
fn keeps_doc_comments() {
    let file = parse_ok(
        "// Rooms and what is in them.\ncontext Dungeon {\n    // Moves the player.\n    fn walk(): Unit\n}",
    );

    assert_eq!(only_context(&file).doc, vec!["Rooms and what is in them."]);

    let ContextItem::Function(walk) = &only_context(&file).items[0] else {
        panic!("expected a function")
    };

    assert_eq!(walk.doc, vec!["Moves the player.".to_string()]);
}

#[test]
fn reads_a_domain() {
    let source = "domain Game {\n    contexts { Dungeon, Scoring }\n    use { PlayTurn } from Dungeon\n    calls { Keyboard }\n} = {\n    PlayTurn()?\n}";
    let file = parse_ok(source);

    let [domain] = &file.domains[..] else {
        panic!("expected one domain")
    };

    assert_eq!(domain.name.name, "Game");
    assert_eq!(domain.contexts.len(), 2);
    assert_eq!(domain.uses[0].from.text(), "Dungeon");
    assert_eq!(domain.calls[0].name.text(), "Keyboard");
}

#[test]
fn reads_a_context_map() {
    let source = "domain Game {\n    contexts {\n        Dungeon -> Scoring -> Reports\n        Settings\n    }\n} = Unit";
    let file = parse_ok(source);
    let domain = &file.domains[0];
    let names: Vec<&str> = domain.contexts.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Dungeon", "Scoring", "Reports", "Settings"]);

    let arrows: Vec<(&str, &str)> = domain
        .relations
        .iter()
        .map(|(a, b)| (a.name.as_str(), b.name.as_str()))
        .collect();

    assert_eq!(arrows, [("Dungeon", "Scoring"), ("Scoring", "Reports")]);
}

#[test]
fn reads_the_starting_value_of_an_adapter_field() {
    let source = "context Platform {\n    adapter ScriptedKeyboard for Keyboard = {\n        keys: List<Key> where Length(0, 32) = [Up, Fire]\n    }\n}";
    let file = parse_ok(source);

    let ContextItem::Adapter(adapter) = &only_context(&file).items[0] else {
        panic!("expected an adapter")
    };

    let field = &adapter.fields[0];
    assert_eq!(field.constraints.len(), 1);

    assert!(
        matches!(field.value.as_ref().map(|v| &v.kind), Some(crate::ast::ExprKind::List(items)) if items.len() == 2)
    );
}

#[test]
fn explains_that_an_entity_belongs_in_an_aggregate() {
    assert_eq!(
        parse_error("context Shop {\n entity Item {\n identity id: ItemId\n }\n}"),
        "an entity belongs inside an aggregate; a thing with identity on its own is an `aggregate` with no entities"
    );
}

#[test]
fn explains_that_an_identity_type_belongs_in_the_context() {
    assert_eq!(
        parse_error("context Shop {\n aggregate Cart {\n identity CartId: String\n }\n}"),
        "an identity type is declared in the context, beside the aggregate: `identity CartId for Cart: Int`; the field that holds it is written `id: CartId`"
    );
}

#[test]
fn reads_what_an_identity_is_for() {
    let file = parse_ok("context Shop {\n identity LineId for Cart.Line: Int\n}");

    let ContextItem::Identity(identity) = &only_context(&file).items[0] else {
        panic!("expected an identity")
    };

    assert_eq!(identity.owner.text(), "Cart.Line");
}

#[test]
fn explains_that_an_identity_names_what_it_identifies() {
    assert_eq!(
        parse_error("context Shop {\n identity CartId: String\n}"),
        "an identity names the aggregate or entity it identifies: `identity CartId for Name: Int`"
    );
}

#[test]
fn explains_that_an_identity_field_needs_no_marker() {
    assert_eq!(
        parse_error("context Shop {\n aggregate Cart {\n identity id: CartId\n }\n}"),
        "an identity field needs no marker: write `id: CartId`, and declare the type in the context with what it identifies: `identity CartId for Cart: Int`"
    );
}

#[test]
fn explains_that_a_service_belongs_in_the_context() {
    assert_eq!(
        parse_error("context Shop {\n aggregate Cart {\n service Checkout() = Unit\n }\n}"),
        "`service` is declared in the context, not inside an aggregate; an aggregate holds fields, entities, invariants, errors, events and commands"
    );
}

#[test]
fn explains_that_aggregates_do_not_nest() {
    assert_eq!(
        parse_error("context Shop {\n aggregate Cart {\n aggregate Line {}\n }\n}"),
        "aggregates do not nest; declare each aggregate in the context and refer to another by its identity"
    );
}

#[test]
fn explains_that_an_event_belongs_in_an_aggregate() {
    assert_eq!(
        parse_error("context Shop {\n event CartEmptied {}\n}"),
        "an event belongs inside the aggregate that emits it"
    );
}

#[test]
fn explains_that_declarations_go_inside_a_context() {
    assert_eq!(
        parse_error("identity CartId for Cart: String"),
        "`identity` is declared inside a context: `context Name { ... }`"
    );
}

#[test]
fn explains_where_an_adapter_body_goes() {
    assert_eq!(
        parse_error(
            "context Platform {\n adapter NullScreen for Screen {\n fn clear() = Unit\n }\n}"
        ),
        "an adapter's body comes after `=`: `adapter Name for Port = { ... }`"
    );
}

#[test]
fn explains_that_ports_are_listed_with_calls() {
    assert_eq!(
        parse_error("context Dungeon {\n service Play(): Unit {\n uses Screen\n } = Unit\n}"),
        "the ports a body calls are listed with `calls`: `calls { Screen, Keyboard }`"
    );
}

#[test]
fn reads_calls_with_names_after_as_and_the_context_after_from() {
    let source = "context Dungeon {\n    service Play(from: Int, to: Int): Unit {\n        \
                  calls { Keyboard as keys, Screen } from Platform\n        calls { Console }\n        \
                  from != to\n    } = Unit\n}";
    let file = parse_ok(source);

    let ContextItem::Function(play) = &only_context(&file).items[0] else {
        panic!("expected a service")
    };

    let written: Vec<String> = play.calls.iter().map(|c| c.name.text()).collect();
    let from: Vec<Option<String>> = (play.calls.iter())
        .map(|c| c.from.as_ref().map(|path| path.text()))
        .collect();

    assert_eq!(written, ["Keyboard as keys", "Screen", "Console"]);
    assert_eq!(play.calls[0].name.local_name(), "keys");
    assert_eq!(
        from,
        [
            Some("Platform".to_string()),
            Some("Platform".to_string()),
            None
        ]
    );
    assert_eq!(play.preconditions.len(), 1);
}

#[test]
fn reads_names_after_as_in_use() {
    let file = parse_ok(
        "context Scoring {\n use { Room.EnemyDefeated as Defeated, RoomId } from Dungeon\n}",
    );

    let ContextItem::Use(use_decl) = &only_context(&file).items[0] else {
        panic!("expected a use")
    };

    let local: Vec<String> = use_decl.names.iter().map(|n| n.local_name()).collect();
    assert_eq!(local, ["Defeated", "RoomId"]);
}

#[test]
fn explains_that_called_ports_go_in_braces() {
    assert_eq!(
        parse_error("context Dungeon {\n service Play(): Unit {\n calls Screen\n } = Unit\n}"),
        "the ports a body calls go in braces: `calls { Screen }`"
    );
}

#[test]
fn adds_rules_written_in_a_contract_to_their_parameter() {
    let file = parse_ok(
        "context Dungeon {\n fn attack(damage: Int where Range(1, 20), target: EnemyId): Int {\n damage where Even\n target where Alive, Near\n damage > 2\n } = damage\n}",
    );

    let ContextItem::Function(attack) = &only_context(&file).items[0] else {
        panic!("expected a function");
    };

    let parameters = attack.parameters.as_ref().expect("parameters");

    let constraints = |index: usize| -> Vec<String> {
        parameters[index]
            .constraints
            .iter()
            .map(|constraint| constraint.name.text())
            .collect()
    };

    assert_eq!(constraints(0), ["Range", "Even"]);
    assert_eq!(constraints(1), ["Alive", "Near"]);
    assert_eq!(attack.preconditions.len(), 1);
}

#[test]
fn reports_rules_in_a_contract_for_a_name_that_is_not_a_parameter() {
    assert_eq!(
        parse_error("context Dungeon {\n fn f(a: Int): Int {\n b where Even\n } = a\n}"),
        "`b` is not a parameter, so it can't have constraints here"
    );
}

#[test]
fn explains_that_storage_is_chosen_in_the_project_file() {
    assert_eq!(
        parse_error("context Shop {\n repository Shelves for Shelf in Storage\n}"),
        "a repository's storage is chosen in croft.toml, such as `Rooms = \"FileStorage\"`"
    );
}

#[test]
fn reads_a_repository_with_and_without_operations() {
    let source = "context Shop {\n    repository Tills for Till\n    repository Shelves for Shelf {\n        \
                  fn find(id: ShelfId): Result<Option<Shelf>, StorageError>\n        \
                  fn empty(): Result<List<Shelf>, StorageError>\n    }\n}";

    let file = parse_ok(source);
    let items = &only_context(&file).items;

    let ContextItem::Repository(tills) = &items[0] else {
        panic!("expected a repository")
    };

    assert!(tills.operations.is_none());

    let ContextItem::Repository(shelves) = &items[1] else {
        panic!("expected a repository")
    };

    let operations = shelves.operations.as_ref().unwrap();
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[1].name.name, "empty");
    assert_eq!(operations[1].kind, FunctionKind::Operation);
}
