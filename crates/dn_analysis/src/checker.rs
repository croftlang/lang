use std::collections::{HashMap, HashSet};

use dn_syntax::ast::*;
use dn_syntax::{Diagnostic, Span};

use crate::naming::{is_pascal_case, is_snake_case, is_upper_snake_case, to_snake_case};
use crate::program::{Program, Scope, OUTPUT_NAME};
use crate::types::Type;

/// Checks a set of parsed modules as one program.
pub fn check(modules: Vec<Module>) -> Result<Program, Vec<Diagnostic>> {
    let program = Program::collect(modules)?;
    let mut checker = Checker {
        program: &program,
        errors: Vec::new(),
    };
    checker.check_all();
    if checker.errors.is_empty() {
        Ok(program)
    } else {
        let mut errors = checker.errors;
        errors.sort_by_key(|error| (error.span.source.0, error.span.line, error.span.column));
        Err(errors)
    }
}

struct Checker<'a> {
    program: &'a Program,
    errors: Vec<Diagnostic>,
}

#[derive(Clone, Copy)]
enum Casing {
    Pascal,
    Snake,
    UpperSnake,
}

impl<'a> Checker<'a> {
    fn report(&mut self, message: impl Into<String>, span: Span) {
        self.errors.push(Diagnostic::new(message, span));
    }

    fn report_result<T>(&mut self, result: Result<T, Diagnostic>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.errors.push(error);
                None
            }
        }
    }

    fn check_all(&mut self) {
        for item in &self.program.items {
            match item {
                Item::Struct(decl) => self.check_struct(decl),
                Item::Enum(decl) => self.check_enum(decl),
                Item::Const(decl) => self.check_const(decl),
                Item::Constraint(decl) => self.check_constraint(decl),
                Item::Port(decl) => self.check_port(decl),
                Item::Adapter(decl) => self.check_adapter(decl),
                Item::Function(decl) => self.check_function(decl),
            }
        }
        self.check_struct_cycles();
    }

    fn check_casing(&mut self, name: &Identifier, casing: Casing, what: &str) {
        let (valid, style, example) = match casing {
            Casing::Pascal => (is_pascal_case(&name.name), "PascalCase", "InvoiceLine"),
            Casing::Snake => (is_snake_case(&name.name), "snake_case", "issue_day"),
            Casing::UpperSnake => (
                is_upper_snake_case(&name.name),
                "UPPER_SNAKE_CASE",
                "MAX_LINES",
            ),
        };
        if matches!(name.name.as_str(), "self" | "crate" | "super" | "Self") {
            self.report(format!("`{}` is a reserved name", name.name), name.span);
            return;
        }
        if !valid {
            self.report(
                format!("{what} `{}` should be {style}, like `{example}`", name.name),
                name.span,
            );
        }
    }

    fn check_unique<'n>(&mut self, names: impl IntoIterator<Item = &'n Identifier>, what: &str) {
        let mut seen = HashSet::new();
        for name in names {
            if !seen.insert(name.name.as_str()) {
                self.report(
                    format!("{what} `{}` is declared more than once", name.name),
                    name.span,
                );
            }
        }
    }

    fn check_struct(&mut self, decl: &StructDecl) {
        self.check_casing(&decl.name, Casing::Pascal, "struct");
        self.check_unique(decl.fields.iter().map(|field| &field.name), "field");
        for field in &decl.fields {
            self.check_casing(&field.name, Casing::Snake, "field");
            let Some(field_type) = self.report_result(self.program.resolve_type(&field.ty, &[]))
            else {
                continue;
            };
            for constraint_use in &field.constraints {
                self.check_constraint_use(constraint_use, &field_type);
            }
        }
        let struct_type = Type::Struct(decl.name.name.clone());
        for constraint_use in &decl.constraints {
            self.check_constraint_use(constraint_use, &struct_type);
        }
    }

    /// `subject` is the field or struct the constraint is attached to.
    fn check_constraint_use(&mut self, constraint_use: &ConstraintUse, subject: &Type) {
        let name = &constraint_use.name;
        let Some(constraint) = self.program.constraint_decl(&name.name) else {
            self.report(format!("unknown constraint `{}`", name.name), name.span);
            return;
        };
        let expected = constraint.parameters.len().saturating_sub(1);
        if constraint_use.arguments.len() != expected {
            self.report(
                format!(
                    "constraint `{}` needs {expected} argument(s) here but {} were given",
                    name.name,
                    constraint_use.arguments.len()
                ),
                name.span,
            );
            return;
        }
        let mut types = vec![subject.clone()];
        for argument in &constraint_use.arguments {
            if matches!(argument.kind, ExprKind::Call(..)) {
                self.report(
                    "constraint arguments must be values, not calls",
                    argument.span,
                );
                return;
            }
            match self.program.type_of(argument, &Scope::new()) {
                Ok(ty) => types.push(ty),
                Err(error) => {
                    self.errors.push(error);
                    return;
                }
            }
        }
        let result = self
            .program
            .match_constraint_parameters(constraint, &types, name.span);
        self.report_result(result);
    }

    fn check_enum(&mut self, decl: &EnumDecl) {
        self.check_casing(&decl.name, Casing::Pascal, "enum");
        if decl.variants.is_empty() {
            self.report(
                format!("enum `{}` needs at least one variant", decl.name.name),
                decl.name.span,
            );
        }
        self.check_unique(&decl.variants, "variant");
        for variant in &decl.variants {
            self.check_casing(variant, Casing::Pascal, "variant");
        }
    }

    fn check_const(&mut self, decl: &ConstDecl) {
        self.check_casing(&decl.name, Casing::UpperSnake, "constant");
        let Some(declared) = self.report_result(self.program.resolve_type(&decl.ty, &[])) else {
            return;
        };
        if !matches!(
            declared,
            Type::Int | Type::Float | Type::Bool | Type::String
        ) {
            self.report(
                format!("constants must be Int, Float, Bool or String, found {declared}"),
                decl.ty.name.span,
            );
            return;
        }
        if contains_call(&decl.value) {
            self.report("constant values cannot call constraints", decl.value.span);
            return;
        }
        if let Some(actual) = self.report_result(self.program.type_of(&decl.value, &Scope::new())) {
            if actual != declared {
                self.report(
                    format!("constant is declared as {declared} but its value is {actual}"),
                    decl.value.span,
                );
            }
        }
    }

    fn check_constraint(&mut self, decl: &ConstraintDecl) {
        self.check_casing(&decl.name, Casing::Pascal, "constraint");
        self.check_unique(&decl.generics, "type parameter");
        if decl.parameters.is_empty() {
            self.report(
                format!(
                    "constraint `{}` needs at least one parameter: the value it checks",
                    decl.name.name
                ),
                decl.name.span,
            );
            return;
        }
        let Some(scope) = self.parameter_scope(&decl.parameters, &decl.generics) else {
            return;
        };
        self.expect_bool(&decl.body, &scope, "a constraint body");
    }

    fn parameter_scope(
        &mut self,
        parameters: &[Parameter],
        generics: &[Identifier],
    ) -> Option<Scope> {
        self.check_unique(
            parameters.iter().map(|parameter| &parameter.name),
            "parameter",
        );
        let mut scope = HashMap::new();
        let mut complete = true;
        for parameter in parameters {
            self.check_casing(&parameter.name, Casing::Snake, "parameter");
            if parameter.name.name == OUTPUT_NAME {
                self.report(
                    "`output` is reserved for a function's output in `ensures`",
                    parameter.name.span,
                );
            }
            match self.program.resolve_type(&parameter.ty, generics) {
                Ok(ty) => {
                    scope.insert(parameter.name.name.clone(), ty);
                }
                Err(error) => {
                    self.errors.push(error);
                    complete = false;
                }
            }
        }
        complete.then_some(scope)
    }

    fn expect_bool(&mut self, expr: &Expr, scope: &Scope, context: &str) {
        if let Some(ty) = self.report_result(self.program.type_of(expr, scope)) {
            if ty != Type::Bool {
                self.report(format!("{context} must be a Bool, found {ty}"), expr.span);
            }
        }
    }

    fn check_signature(&mut self, signature: &Signature) -> Option<(Scope, Type)> {
        self.check_casing(&signature.name, Casing::Snake, "function");
        let scope = self.parameter_scope(&signature.parameters, &[]);
        let output = self.report_result(self.program.resolve_type(&signature.output, &[]));
        Some((scope?, output?))
    }

    fn check_port(&mut self, decl: &PortDecl) {
        self.check_casing(&decl.name, Casing::Pascal, "port");
        self.check_unique(
            decl.operations.iter().map(|operation| &operation.name),
            "operation",
        );
        for operation in &decl.operations {
            self.check_signature(operation);
        }
    }

    fn check_adapter(&mut self, decl: &AdapterDecl) {
        self.check_casing(&decl.name, Casing::Pascal, "adapter");
        if self.program.port_decl(&decl.port.name).is_none() {
            self.report(format!("unknown port `{}`", decl.port.name), decl.port.span);
        }
    }

    fn check_function(&mut self, decl: &FunctionDecl) {
        let signature = self.check_signature(&decl.signature);
        self.check_unique(&decl.uses, "port");
        for port in &decl.uses {
            if self.program.port_decl(&port.name).is_none() {
                self.report(format!("unknown port `{}`", port.name), port.span);
                continue;
            }
            let parameter_name = to_snake_case(&port.name);
            if decl
                .signature
                .parameters
                .iter()
                .any(|parameter| parameter.name.name == parameter_name)
            {
                self.report(
                    format!(
                        "parameter `{parameter_name}` clashes with the name generated for port `{}`",
                        port.name
                    ),
                    port.span,
                );
            }
        }
        if decl.description.is_none() {
            self.report(
                format!(
                    "function `{}` needs a `description` saying what it does",
                    decl.signature.name.name
                ),
                decl.signature.name.span,
            );
        }
        let Some((mut scope, output)) = signature else {
            return;
        };
        for requirement in &decl.requires {
            self.expect_bool(requirement, &scope, "`requires`");
        }
        scope.insert(OUTPUT_NAME.to_string(), output);
        for guarantee in &decl.ensures {
            self.expect_bool(guarantee, &scope, "`ensures`");
        }
    }

    /// A struct cannot contain itself except through a `List`,
    /// otherwise it would have infinite size.
    fn check_struct_cycles(&mut self) {
        let mut reported = HashSet::new();
        for item in &self.program.items {
            let Item::Struct(decl) = item else { continue };
            let mut path = vec![decl.name.name.clone()];
            if self.contains_struct(&decl.name.name, &decl.name.name, &mut path) {
                let mut members = path.clone();
                members.sort();
                if reported.insert(members) {
                    self.report(
                        format!(
                            "struct `{}` contains itself ({}); wrap the field in a List",
                            decl.name.name,
                            path.join(" -> ")
                        ),
                        decl.name.span,
                    );
                }
            }
        }
    }

    fn contains_struct(&self, current: &str, target: &str, path: &mut Vec<String>) -> bool {
        let Some(decl) = self.program.struct_decl(current) else {
            return false;
        };
        for field in &decl.fields {
            let Some(name) = direct_struct(&field.ty) else {
                continue;
            };
            if name == target {
                path.push(name.to_string());
                return true;
            }
            if path.iter().any(|visited| visited == name) {
                continue;
            }
            path.push(name.to_string());
            if self.contains_struct(name, target, path) {
                return true;
            }
            path.pop();
        }
        false
    }
}

/// The struct a field holds directly, looking through `Option`.
fn direct_struct(ty: &TypeExpr) -> Option<&str> {
    match ty.name.name.as_str() {
        "Option" => ty.arguments.first().and_then(direct_struct),
        "List" | "Result" => None,
        name => Some(name),
    }
}

fn contains_call(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Call(..) => true,
        ExprKind::Field(base, _) => contains_call(base),
        ExprKind::Unary(_, operand) => contains_call(operand),
        ExprKind::Binary(_, left, right) => contains_call(left) || contains_call(right),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dn_syntax::{parse, SourceId};

    fn check_source(source: &str) -> Result<Program, Vec<Diagnostic>> {
        check(vec![
            parse(source, SourceId(0)).expect("source should parse")
        ])
    }

    fn errors(source: &str) -> Vec<String> {
        match check_source(source) {
            Ok(_) => Vec::new(),
            Err(errors) => errors.into_iter().map(|error| error.message).collect(),
        }
    }

    #[test]
    fn accepts_valid_program() {
        let source = r#"
            const MAX_LINES: Int = 100
            enum Currency { Gbp, Usd }
            struct Money { amount: Int is Positive  currency: Currency }
            struct Invoice { lines: List<Money> is NonEmpty, MaxItems(MAX_LINES) }
            constraint Positive(value: Int) = value > 0
            constraint NonEmpty<T>(list: List<T>) = list.length > 0
            constraint MaxItems<T>(list: List<T>, limit: Int) = list.length <= limit
            port Store { fn save(invoice: Invoice) -> Result<Unit, Currency> }
            adapter MemoryStore for Store
            fn total(invoice: Invoice) -> Money {
                uses Store
                requires NonEmpty(invoice.lines)
                ensures output.currency == Currency.Gbp
                description "Adds up the invoice."
            }
        "#;
        assert_eq!(errors(source), Vec::<String>::new());
    }

    #[test]
    fn reports_unknown_names() {
        let found = errors(
            "struct A { b: Missing is Nope }
             fn f(a: A) -> Bool { uses Ghost description \"x\" }",
        );
        assert!(found.iter().any(|e| e.contains("unknown type `Missing`")));
        assert!(found.iter().any(|e| e.contains("unknown port `Ghost`")));
    }

    #[test]
    fn reports_type_errors_in_constraints() {
        let found = errors("constraint Bad(text: String) = text > 3");
        assert!(found[0].contains("needs two numbers"), "{found:?}");
    }

    #[test]
    fn reports_wrong_constraint_subject() {
        let found = errors(
            "constraint Positive(value: Int) = value > 0
             struct A { name: String is Positive }",
        );
        assert!(found[0].contains("expects Int"), "{found:?}");
    }

    #[test]
    fn requires_description_on_functions() {
        let found = errors("fn f(x: Int) -> Int { }");
        assert!(found[0].contains("needs a `description`"));
    }

    #[test]
    fn enforces_naming_conventions() {
        let found = errors("struct invoice { IssueDay: Int }");
        assert!(found.iter().any(|e| e.contains("should be PascalCase")));
        assert!(found.iter().any(|e| e.contains("should be snake_case")));
    }

    #[test]
    fn rejects_self_containing_struct() {
        let found = errors("struct Node { next: Option<Node> }");
        assert!(found[0].contains("contains itself"));
        assert!(errors("struct Tree { children: List<Tree> }").is_empty());
    }

    #[test]
    fn output_is_only_visible_in_ensures() {
        let found = errors("fn f(x: Int) -> Int { requires output > 0 description \"x\" }");
        assert!(found[0].contains("unknown name `output`"));
    }
}
