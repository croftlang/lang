//! The type of each kind of expression, and operators.

use croft_syntax::ast::{BinaryOp, Expr, ExprKind, UnaryOp};

use super::BodyChecker;
use crate::facts::Meaning;
use crate::types::{Type, MAX_SHIFT};

/// `2147483648`, which is a whole number only as `-2147483648`.
const LOWEST_MAGNITUDE: i64 = 1 << 31;

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// The type of an expression. `expected`, when known, lets bare enum
    /// variants such as `North` be found.
    pub(super) fn expr(&mut self, value: &Expr, expected: Option<&Type>) -> Type {
        let ty = self.expr_kind(value, expected);
        self.checker.facts.types.insert(value.id, ty.clone());
        self.note_name_reference(value);
        ty
    }

    /// A name such as `NotFound` or `act` can be resolved through the type
    /// it is expected to have rather than `resolve`, so its meaning is noted
    /// here too, for editors. In a call, the callee's name is noted.
    fn note_name_reference(&mut self, value: &Expr) {
        let (name, span, id) = match &value.kind {
            ExprKind::Name(name) => (name, value.span, value.id),
            ExprKind::Call(callee, _) => match &callee.kind {
                ExprKind::Name(name) => (name, callee.span, value.id),
                _ => return,
            },
            _ => return,
        };

        let facts = &self.checker.facts;

        let meaning = facts.meanings.get(&id).or_else(|| match &value.kind {
            ExprKind::Call(callee, _) => facts.meanings.get(&callee.id),
            _ => None,
        });

        let target = match meaning {
            Some(Meaning::Const(found) | Meaning::Error(found) | Meaning::Declared(found)) => found,
            Some(Meaning::Variant(_, _, Some(error))) => error,
            Some(Meaning::Variant(enum_type, _, None)) => enum_type,
            _ => return,
        };

        let reference = crate::facts::Reference {
            span,
            name: name.clone(),
            target: target.clone(),
        };

        self.checker.references.get_mut().push(reference);
    }

    /// The type of an expression, by its kind. `expr` records it.
    fn expr_kind(&mut self, value: &Expr, expected: Option<&Type>) -> Type {
        let span = value.span;

        match &value.kind {
            ExprKind::Int(number) => {
                self.check_int_literal(*number, span);
                Type::Int
            }
            ExprKind::Float(text) => self.float(text, span, expected),
            ExprKind::String(_) => Type::String,
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::Name(name) => self.name(Some(value.id), name, span, expected),
            ExprKind::Field(inner, field) => self.field(value, inner, field, expected),
            ExprKind::Call(callee, arguments) => {
                self.call(value.id, callee, arguments, span, expected)
            }
            ExprKind::List(items) => self.list_literal(items, expected),
            ExprKind::Map(entries) => self.map_literal(entries, expected, span),
            ExprKind::Record(path, fields) => self.record(path, fields, expected),
            ExprKind::With(base, fields) => self.with(base, fields),
            ExprKind::Unary(op, operand) => {
                let ty = self.unary(*op, operand, expected);

                if *op == UnaryOp::Negate {
                    self.check_known_negation(value, operand, &ty);
                }

                ty
            }
            ExprKind::Binary(op, left, right) => {
                let ty = self.binary(*op, left, right);
                self.check_known_arithmetic(value, (*op, left, right), &ty);
                ty
            }
            ExprKind::OrElse(value, error) => self.or_else(value, error),
            ExprKind::Try(inner) => self.try_value(inner),
            ExprKind::Lambda(..) => {
                self.error(
                    "a lambda can only be passed to a list or option method",
                    span,
                );

                Type::Unknown
            }
            ExprKind::If(condition, block) => {
                self.expect(condition, &Type::Bool);
                self.expr(block, None);
                Type::Unit
            }
            ExprKind::Match(subject, arms) => self.check_match(subject, arms, expected),
            ExprKind::Block(statements) => self.block(statements, expected),
            ExprKind::Issue(issue) => self.issue(value.id, issue, span),
            ExprKind::Emit(event, fields) => self.emit(event, fields, span),
            ExprKind::Native(function, arguments) => {
                self.native(function, arguments, span, expected)
            }
            ExprKind::Loop(counter, block) => self.check_loop(counter.as_ref(), block, span),
            ExprKind::Break => self.check_break(span),
        }
    }

    /// `!` takes a `Bool`, and `-` a number.
    pub(super) fn unary(&mut self, op: UnaryOp, operand: &Expr, expected: Option<&Type>) -> Type {
        match op {
            UnaryOp::Not => {
                self.expect(operand, &Type::Bool);
                Type::Bool
            }
            // The lowest whole number is written as `-2147483648`, and only
            // there is `2147483648` allowed.
            UnaryOp::Negate if operand.kind == ExprKind::Int(LOWEST_MAGNITUDE) => {
                self.checker.facts.types.insert(operand.id, Type::Int);
                Type::Int
            }
            UnaryOp::Negate => {
                let ty = self.expr(operand, expected);

                if !ty.is_number() {
                    self.error(format!("cannot negate `{ty}`"), operand.span);
                }

                ty
            }
            UnaryOp::BitNot => {
                self.expect(operand, &Type::Int);
                Type::Int
            }
        }
    }

    /// The type of a binary operator's result, after checking both sides.
    pub(super) fn binary(&mut self, op: BinaryOp, left: &Expr, right: &Expr) -> Type {
        use BinaryOp::*;

        match op {
            And | Or => {
                self.expect(left, &Type::Bool);
                self.expect(right, &Type::Bool);
                Type::Bool
            }
            Equal | NotEqual => {
                let ty = self.expr(left, None);
                self.expect(right, &ty);
                Type::Bool
            }
            Less | LessEqual | Greater | GreaterEqual => {
                let ty = self.expr(left, None);

                if !ty.is_number() && !self.checker.is_int_identity(&ty) {
                    self.error(format!("cannot compare `{ty}` by size"), left.span);
                }

                self.expect(right, &ty);

                Type::Bool
            }
            Add | Subtract | Multiply | Divide | Remainder => {
                // A list of numbers is also how `Bytes` are written, so
                // `[0] + data` joins `Bytes`, unless `data` is a list.
                if op == Add && is_list_literal(left) && !is_list_literal(right) {
                    return self.join_onto_literal(left, right);
                }

                let ty = match op {
                    Add if is_list_literal(left) => self.expr(left, Some(&Type::Bytes)),
                    _ => self.expr(left, None),
                };

                if let Some(ty) = self.decimal_arithmetic(op, &ty, left, right) {
                    return ty;
                }

                let joins = op == Add && matches!(ty, Type::String | Type::Bytes | Type::List(_));

                if !ty.is_number() && !joins {
                    self.error(format!("cannot do arithmetic on `{ty}`"), left.span);
                    return Type::Unknown;
                }

                self.expect(right, &ty);

                ty
            }
            BitAnd | BitOr | BitXor => {
                self.expect(left, &Type::Int);
                self.expect(right, &Type::Int);
                Type::Int
            }
            ShiftLeft | ShiftRight => {
                self.expect(left, &Type::Int);
                self.expect(right, &Type::Int);

                if let ExprKind::Int(amount) = right.kind {
                    if !(0..MAX_SHIFT).contains(&amount) {
                        self.error(
                            format!("cannot shift by {amount}, only by 0 to {}", MAX_SHIFT - 1),
                            right.span,
                        );
                    }
                }

                Type::Int
            }
        }
    }
}

impl<'c, 'a> BodyChecker<'c, 'a> {
    /// `[item] + rest`: a list when `rest` is one, and otherwise `Bytes`.
    fn join_onto_literal(&mut self, left: &Expr, right: &Expr) -> Type {
        let right_type = self.expr(right, None);

        let expected = match right_type {
            Type::List(_) => right_type.clone(),
            _ => Type::Bytes,
        };

        let left_type = self.expect(left, &expected);

        if !self.accepts(&left_type, &right_type, right) {
            self.error(
                format!("expected `{left_type}`, found `{right_type}`"),
                right.span,
            );
        }

        left_type
    }
}

/// Whether an expression is a list literal, such as `[1, 2]`.
fn is_list_literal(value: &Expr) -> bool {
    matches!(value.kind, ExprKind::List(_))
}
