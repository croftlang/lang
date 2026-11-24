use std::collections::HashMap;

use dn_syntax::ast::*;
use dn_syntax::{Diagnostic, Span};

use crate::types::{unify, Type};

/// Local names visible inside an expression, with their types.
pub type Scope = HashMap<String, Type>;

/// The name `ensures` expressions use for a function's output.
pub const OUTPUT_NAME: &str = "output";

const BUILTIN_TYPES: [&str; 8] = [
    "Int", "Float", "Bool", "String", "Unit", "List", "Option", "Result",
];

pub fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains(&name)
}

/// Every declaration in a compilation, indexed by name.
#[derive(Debug, Default)]
pub struct Program {
    pub items: Vec<Item>,
    index: HashMap<String, usize>,
}

impl Program {
    /// Collects items from all modules. Fails on duplicate or reserved names.
    pub fn collect(modules: Vec<Module>) -> Result<Program, Vec<Diagnostic>> {
        let mut program = Program::default();
        let mut errors = Vec::new();
        for item in modules.into_iter().flat_map(|module| module.items) {
            let name = item.name().clone();
            if is_builtin_type(&name.name) {
                errors.push(Diagnostic::new(
                    format!(
                        "`{}` is a built-in type and cannot be redeclared",
                        name.name
                    ),
                    name.span,
                ));
                continue;
            }
            if program.index.contains_key(&name.name) {
                errors.push(Diagnostic::new(
                    format!("`{}` is declared more than once", name.name),
                    name.span,
                ));
                continue;
            }
            program.index.insert(name.name.clone(), program.items.len());
            program.items.push(item);
        }
        if errors.is_empty() {
            Ok(program)
        } else {
            Err(errors)
        }
    }

    pub fn item(&self, name: &str) -> Option<&Item> {
        self.index.get(name).map(|&index| &self.items[index])
    }

    pub fn struct_decl(&self, name: &str) -> Option<&StructDecl> {
        match self.item(name) {
            Some(Item::Struct(item)) => Some(item),
            _ => None,
        }
    }

    pub fn enum_decl(&self, name: &str) -> Option<&EnumDecl> {
        match self.item(name) {
            Some(Item::Enum(item)) => Some(item),
            _ => None,
        }
    }

    pub fn const_decl(&self, name: &str) -> Option<&ConstDecl> {
        match self.item(name) {
            Some(Item::Const(item)) => Some(item),
            _ => None,
        }
    }

    pub fn constraint_decl(&self, name: &str) -> Option<&ConstraintDecl> {
        match self.item(name) {
            Some(Item::Constraint(item)) => Some(item),
            _ => None,
        }
    }

    pub fn port_decl(&self, name: &str) -> Option<&PortDecl> {
        match self.item(name) {
            Some(Item::Port(item)) => Some(item),
            _ => None,
        }
    }

    /// Resolves a written type. `generics` lists type parameters in scope.
    pub fn resolve_type(&self, ty: &TypeExpr, generics: &[Identifier]) -> Result<Type, Diagnostic> {
        let name = ty.name.name.as_str();
        let expected_arguments = match name {
            "List" | "Option" => 1,
            "Result" => 2,
            _ => 0,
        };
        if ty.arguments.len() != expected_arguments {
            return Err(Diagnostic::new(
                format!(
                    "`{name}` takes {expected_arguments} type argument(s) but {} were given",
                    ty.arguments.len()
                ),
                ty.name.span,
            ));
        }
        let argument = |index: usize| -> Result<Box<Type>, Diagnostic> {
            self.resolve_type(&ty.arguments[index], generics)
                .map(Box::new)
        };
        let resolved = match name {
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Unit" => Type::Unit,
            "List" => Type::List(argument(0)?),
            "Option" => Type::Option(argument(0)?),
            "Result" => Type::Result(argument(0)?, argument(1)?),
            _ if generics.iter().any(|generic| generic.name == name) => {
                Type::Generic(name.to_string())
            }
            _ => match self.item(name) {
                Some(Item::Struct(_)) => Type::Struct(name.to_string()),
                Some(Item::Enum(_)) => Type::Enum(name.to_string()),
                Some(_) => {
                    return Err(Diagnostic::new(
                        format!("`{name}` is not a struct or enum, so it cannot be used as a type"),
                        ty.name.span,
                    ))
                }
                None => {
                    return Err(Diagnostic::new(
                        format!("unknown type `{name}`"),
                        ty.name.span,
                    ))
                }
            },
        };
        Ok(resolved)
    }

    /// The type of a struct field. Assumes the struct has been checked.
    pub fn field_type(&self, struct_name: &str, field_name: &str) -> Option<Type> {
        let decl = self.struct_decl(struct_name)?;
        let field = decl
            .fields
            .iter()
            .find(|field| field.name.name == field_name)?;
        self.resolve_type(&field.ty, &[]).ok()
    }

    /// Works out the type of an expression, reporting the first error found.
    pub fn type_of(&self, expr: &Expr, scope: &Scope) -> Result<Type, Diagnostic> {
        let error = |message: String| Err(Diagnostic::new(message, expr.span));
        match &expr.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Float(_) => Ok(Type::Float),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::String(_) => Ok(Type::String),
            ExprKind::Name(name) => {
                if let Some(ty) = scope.get(name) {
                    return Ok(ty.clone());
                }
                match self.item(name) {
                    Some(Item::Const(constant)) => self.resolve_type(&constant.ty, &[]),
                    Some(Item::Enum(_)) => Ok(Type::EnumName(name.clone())),
                    Some(_) => error(format!("`{name}` cannot be used as a value")),
                    None => error(format!("unknown name `{name}`")),
                }
            }
            ExprKind::Field(base, field) => {
                let base_type = self.type_of(base, scope)?;
                self.field_access_type(&base_type, field)
            }
            ExprKind::Call(name, arguments) => {
                let Some(constraint) = self.constraint_decl(&name.name) else {
                    return error(format!(
                        "`{}` is not a constraint; only constraints can be called",
                        name.name
                    ));
                };
                if constraint.parameters.len() != arguments.len() {
                    return error(format!(
                        "constraint `{}` takes {} argument(s) but {} were given",
                        name.name,
                        constraint.parameters.len(),
                        arguments.len()
                    ));
                }
                let types = arguments
                    .iter()
                    .map(|argument| self.type_of(argument, scope))
                    .collect::<Result<Vec<_>, _>>()?;
                self.match_constraint_parameters(constraint, &types, expr.span)?;
                Ok(Type::Bool)
            }
            ExprKind::Unary(op, operand) => {
                let operand_type = self.type_of(operand, scope)?;
                match (op, &operand_type) {
                    (UnaryOp::Not, Type::Bool) => Ok(Type::Bool),
                    (UnaryOp::Negate, ty) if ty.is_numeric() => Ok(operand_type),
                    (UnaryOp::Not, ty) => error(format!("`!` needs a Bool, found {ty}")),
                    (UnaryOp::Negate, ty) => error(format!("`-` needs a number, found {ty}")),
                }
            }
            ExprKind::Binary(op, left, right) => {
                let left_type = self.type_of(left, scope)?;
                let right_type = self.type_of(right, scope)?;
                let symbol = op.symbol();
                if op.is_logical() {
                    if left_type == Type::Bool && right_type == Type::Bool {
                        return Ok(Type::Bool);
                    }
                    return error(format!(
                        "`{symbol}` needs Bool on both sides, found {left_type} and {right_type}"
                    ));
                }
                if op.is_equality() {
                    if left_type == right_type && !matches!(left_type, Type::EnumName(_)) {
                        return Ok(Type::Bool);
                    }
                    return error(format!(
                        "cannot compare {left_type} with {right_type} using `{symbol}`"
                    ));
                }
                if left_type != right_type || !left_type.is_numeric() {
                    return error(format!(
                        "`{symbol}` needs two numbers of the same type, found {left_type} and {right_type}"
                    ));
                }
                if op.is_comparison() {
                    Ok(Type::Bool)
                } else {
                    Ok(left_type)
                }
            }
        }
    }

    fn field_access_type(&self, base: &Type, field: &Identifier) -> Result<Type, Diagnostic> {
        let name = field.name.as_str();
        let found = match (base, name) {
            (Type::Struct(struct_name), _) => self.field_type(struct_name, name),
            (Type::EnumName(enum_name), _) => self
                .enum_decl(enum_name)
                .filter(|decl| decl.variants.iter().any(|variant| variant.name == name))
                .map(|_| Type::Enum(enum_name.clone())),
            (Type::List(_) | Type::String, "length") => Some(Type::Int),
            (Type::Option(_), "is_some" | "is_none") => Some(Type::Bool),
            (Type::Result(_, _), "is_ok" | "is_err") => Some(Type::Bool),
            _ => None,
        };
        found.ok_or_else(|| Diagnostic::new(format!("{base} has no field `{name}`"), field.span))
    }

    /// Checks argument types against a constraint's parameters, binding generics.
    pub fn match_constraint_parameters(
        &self,
        constraint: &ConstraintDecl,
        argument_types: &[Type],
        span: Span,
    ) -> Result<(), Diagnostic> {
        let mut bindings = HashMap::new();
        for (parameter, actual) in constraint.parameters.iter().zip(argument_types) {
            let expected = self.resolve_type(&parameter.ty, &constraint.generics)?;
            if !unify(&expected, actual, &mut bindings) {
                return Err(Diagnostic::new(
                    format!(
                        "constraint `{}` expects {expected} for `{}`, found {actual}",
                        constraint.name.name, parameter.name.name
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }
}
