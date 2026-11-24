use super::{function_body, statements};
use crate::ast::{BinaryOp, ExprKind, Statement};

#[test]
fn binds_multiplication_tighter_than_addition() {
    let ExprKind::Binary(BinaryOp::Add, _, right) = function_body("fn f(): Int = 1 + 2 * 3").kind
    else {
        panic!("expected an addition");
    };
    assert!(matches!(
        right.kind,
        ExprKind::Binary(BinaryOp::Multiply, _, _)
    ));
}

#[test]
fn ends_an_expression_at_a_new_line() {
    let body = statements("let a = 1\n-2");
    assert_eq!(body.len(), 2);
}

#[test]
fn continues_a_chain_of_calls_across_lines() {
    let body =
        function_body("fn f(): Int = enemies\n    .min_by(e -> e.hit_points)\n    .map(e -> e.id)");
    let ExprKind::Call(callee, _) = body.kind else {
        panic!("expected a call")
    };
    assert!(matches!(callee.kind, ExprKind::Field(_, ref name) if name.name == "map"));
}

#[test]
fn reads_records_and_with() {
    let body = statements("let start = Position { x: 10, y: 20 }\nstart with { x: 11 }");
    let Statement::Let(_, value) = &body[0] else {
        panic!("expected let")
    };
    assert!(matches!(value.kind, ExprKind::Record(_, ref fields) if fields.len() == 2));
    assert!(matches!(&body[1], Statement::Expr(e) if matches!(e.kind, ExprKind::With(_, _))));
}

#[test]
fn reads_the_block_after_an_if_condition() {
    let body = statements("if Ready {\n    emit Started\n}\nStopped");
    let Statement::Expr(guard) = &body[0] else {
        panic!("expected an if")
    };
    let ExprKind::If(condition, _) = &guard.kind else {
        panic!("expected an if")
    };
    assert!(matches!(condition.kind, ExprKind::Name(ref name) if name == "Ready"));
}

#[test]
fn applies_try_to_the_whole_issue() {
    let body = statements("issue Attack(enemy_id: target, damage: 5) on Room(room_id)?");
    let Statement::Expr(value) = &body[0] else {
        panic!("expected an expression")
    };
    let ExprKind::Try(inner) = &value.kind else {
        panic!("expected a try")
    };
    let ExprKind::Issue(issue) = &inner.kind else {
        panic!("expected an issue")
    };
    assert!(matches!(issue.target.kind, ExprKind::Call(_, _)));
}

#[test]
fn reads_or_as_a_replacement_error() {
    let body = statements("issue Reset on Scoreboard(MAIN) or GameError.ScoreboardUnavailable");
    let Statement::Expr(value) = &body[0] else {
        panic!("expected an expression")
    };
    assert!(matches!(value.kind, ExprKind::OrElse(_, _)));
}

#[test]
fn reads_match_arms_with_bindings() {
    let body = function_body(
        "fn f(): Int = match kind {\n    Coins(count) -> count * 10\n    Flask -> 50\n}",
    );
    let ExprKind::Match(_, arms) = body.kind else {
        panic!("expected a match")
    };
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].pattern.bindings[0].name, "count");
}

#[test]
fn reads_event_as_a_name_in_a_policy() {
    let body = statements("issue DropTreasure(at: event.at) on Room(event.source)");
    assert_eq!(body.len(), 1);
}
