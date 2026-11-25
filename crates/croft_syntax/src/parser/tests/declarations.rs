use super::{only_context, parse_error, parse_ok};
use crate::ast::{ContextItem, FunctionKind};

#[test]
fn reads_a_context_header_as_the_rest_of_the_file() {
    let file = parse_ok("context Dungeon\n\nconst MAX: Int = 16\nidentity RoomId: Int");
    let context = only_context(&file);
    assert_eq!(context.name.name, "Dungeon");
    assert_eq!(context.items.len(), 2);
}

#[test]
fn reads_an_aggregate_header_as_the_rest_of_the_file() {
    let source = "context Dungeon\naggregate Room\n\nidentity id: RoomId\nerror NotFound\n\
                  command Reset = {\n    set lives = 3\n}";
    let file = parse_ok(source);
    let [ContextItem::Aggregate(room)] = &only_context(&file).items[..] else {
        panic!("expected one aggregate");
    };
    assert_eq!(room.name.name, "Room");
    assert!(room.fields[0].is_identity);
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
fn reads_a_contract_with_preconditions_and_uses() {
    let source = "context Dungeon {\n    service Play(id: RoomId): Unit {\n        calls Keyboard, Screen\n        \
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
    let file = parse_ok("context Dungeon {\n    // Moves the player.\n    fn walk(): Unit\n}");
    let ContextItem::Function(walk) = &only_context(&file).items[0] else {
        panic!("expected a function")
    };
    assert_eq!(walk.doc, vec!["Moves the player.".to_string()]);
}

#[test]
fn reads_a_domain() {
    let source = "domain Game {\n    contexts { Dungeon, Scoring }\n    use { PlayTurn } from Dungeon\n    calls Keyboard\n} = {\n    PlayTurn()?\n}";
    let file = parse_ok(source);
    let [domain] = &file.domains[..] else {
        panic!("expected one domain")
    };
    assert_eq!(domain.name.name, "Game");
    assert_eq!(domain.contexts.len(), 2);
    assert_eq!(domain.uses[0].from.text(), "Dungeon");
    assert_eq!(domain.calls[0].text(), "Keyboard");
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
        "the ports a body calls are listed with `calls`: `calls Screen, Keyboard`"
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
