use crate::ast::*;
use crate::lexer::tokenize;
use crate::span::{Diagnostic, SourceId};
use crate::token::{Token, TokenKind};

/// Parses one source file into a module.
pub fn parse(source: &str, source_id: SourceId) -> Result<Module, Diagnostic> {
    let tokens = tokenize(source, source_id)?;
    Parser {
        tokens,
        position: 0,
    }
    .module()
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

type ParseResult<T> = Result<T, Diagnostic>;

impl Parser {
    // ---- token helpers ----

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if token.kind != TokenKind::EndOfFile {
            self.position += 1;
        }
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn advance_if(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, context: &str) -> ParseResult<Token> {
        if self.check(&kind) {
            Ok(self.advance())
        } else {
            Err(self.error_expected(&kind.describe(), context))
        }
    }

    fn error_expected(&self, expected: &str, context: &str) -> Diagnostic {
        let found = self.peek();
        Diagnostic::new(
            format!(
                "expected {expected} {context}, found {}",
                found.kind.describe()
            ),
            found.span,
        )
    }

    fn identifier(&mut self, context: &str) -> ParseResult<Identifier> {
        match self.peek_kind().clone() {
            TokenKind::Identifier(name) => {
                let span = self.advance().span;
                Ok(Identifier { name, span })
            }
            _ => Err(self.error_expected("a name", context)),
        }
    }

    // ---- items ----

    fn module(mut self) -> ParseResult<Module> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::EndOfFile) {
            items.push(self.item()?);
        }
        Ok(Module { items })
    }

    fn item(&mut self) -> ParseResult<Item> {
        match self.peek_kind() {
            TokenKind::Struct => self.struct_decl().map(Item::Struct),
            TokenKind::Enum => self.enum_decl().map(Item::Enum),
            TokenKind::Const => self.const_decl().map(Item::Const),
            TokenKind::Constraint => self.constraint_decl().map(Item::Constraint),
            TokenKind::Port => self.port_decl().map(Item::Port),
            TokenKind::Adapter => self.adapter_decl().map(Item::Adapter),
            TokenKind::Fn => self.function_decl().map(Item::Function),
            _ => Err(self.error_expected(
                "`struct`, `enum`, `const`, `constraint`, `port`, `adapter` or `fn`",
                "at the start of a declaration",
            )),
        }
    }

    fn struct_decl(&mut self) -> ParseResult<StructDecl> {
        self.expect(TokenKind::Struct, "")?;
        let name = self.identifier("after `struct`")?;
        self.expect(TokenKind::LeftBrace, "to open the struct")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            let field_name = self.identifier("for a field")?;
            self.expect(TokenKind::Colon, "after the field name")?;
            let ty = self.type_expr()?;
            let constraints = self.constraint_uses()?;
            fields.push(Field {
                name: field_name,
                ty,
                constraints,
            });
        }
        self.expect(TokenKind::RightBrace, "to close the struct")?;
        let constraints = self.constraint_uses()?;
        Ok(StructDecl {
            name,
            fields,
            constraints,
        })
    }

    /// Reads an optional `is A, B(1)` list.
    fn constraint_uses(&mut self) -> ParseResult<Vec<ConstraintUse>> {
        let mut uses = Vec::new();
        if !self.advance_if(&TokenKind::Is) {
            return Ok(uses);
        }
        loop {
            let name = self.identifier("for a constraint after `is`")?;
            let arguments = if self.check(&TokenKind::LeftParen) {
                self.arguments()?
            } else {
                Vec::new()
            };
            uses.push(ConstraintUse { name, arguments });
            if !self.advance_if(&TokenKind::Comma) {
                return Ok(uses);
            }
        }
    }

    fn enum_decl(&mut self) -> ParseResult<EnumDecl> {
        self.expect(TokenKind::Enum, "")?;
        let name = self.identifier("after `enum`")?;
        self.expect(TokenKind::LeftBrace, "to open the enum")?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            variants.push(self.identifier("for an enum variant")?);
            self.advance_if(&TokenKind::Comma);
        }
        self.expect(TokenKind::RightBrace, "to close the enum")?;
        Ok(EnumDecl { name, variants })
    }

    fn const_decl(&mut self) -> ParseResult<ConstDecl> {
        self.expect(TokenKind::Const, "")?;
        let name = self.identifier("after `const`")?;
        self.expect(TokenKind::Colon, "after the constant name")?;
        let ty = self.type_expr()?;
        self.expect(TokenKind::Equal, "before the constant value")?;
        let value = self.expression()?;
        Ok(ConstDecl { name, ty, value })
    }

    fn constraint_decl(&mut self) -> ParseResult<ConstraintDecl> {
        self.expect(TokenKind::Constraint, "")?;
        let name = self.identifier("after `constraint`")?;
        let mut generics = Vec::new();
        if self.advance_if(&TokenKind::Less) {
            loop {
                generics.push(self.identifier("for a type parameter")?);
                if !self.advance_if(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "to close the type parameters")?;
        }
        let parameters = self.parameters()?;
        self.expect(TokenKind::Equal, "before the constraint body")?;
        let body = self.expression()?;
        Ok(ConstraintDecl {
            name,
            generics,
            parameters,
            body,
        })
    }

    fn port_decl(&mut self) -> ParseResult<PortDecl> {
        self.expect(TokenKind::Port, "")?;
        let name = self.identifier("after `port`")?;
        self.expect(TokenKind::LeftBrace, "to open the port")?;
        let mut operations = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.expect(TokenKind::Fn, "to start a port operation")?;
            operations.push(self.signature()?);
        }
        self.expect(TokenKind::RightBrace, "to close the port")?;
        Ok(PortDecl { name, operations })
    }

    fn adapter_decl(&mut self) -> ParseResult<AdapterDecl> {
        self.expect(TokenKind::Adapter, "")?;
        let name = self.identifier("after `adapter`")?;
        self.expect(TokenKind::For, "after the adapter name")?;
        let port = self.identifier("for the port this adapter implements")?;
        Ok(AdapterDecl { name, port })
    }

    fn function_decl(&mut self) -> ParseResult<FunctionDecl> {
        self.expect(TokenKind::Fn, "")?;
        let signature = self.signature()?;
        let mut function = FunctionDecl {
            signature,
            uses: Vec::new(),
            requires: Vec::new(),
            ensures: Vec::new(),
            description: None,
        };
        self.expect(TokenKind::LeftBrace, "to open the function contract")?;
        while !self.check(&TokenKind::RightBrace) {
            // Contract words are contextual, so structs can still have
            // fields called `description` or `uses`.
            let word = match self.peek_kind() {
                TokenKind::Identifier(word) => word.clone(),
                _ => String::new(),
            };
            match word.as_str() {
                "uses" => {
                    self.advance();
                    loop {
                        function
                            .uses
                            .push(self.identifier("for a port after `uses`")?);
                        if !self.advance_if(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                "requires" => {
                    self.advance();
                    function.requires.push(self.expression()?);
                }
                "ensures" => {
                    self.advance();
                    function.ensures.push(self.expression()?);
                }
                "description" => {
                    let span = self.advance().span;
                    if function.description.is_some() {
                        return Err(Diagnostic::new(
                            "a function can only have one description",
                            span,
                        ));
                    }
                    match self.peek_kind().clone() {
                        TokenKind::String(text) => {
                            self.advance();
                            function.description = Some(text);
                        }
                        _ => return Err(self.error_expected("a string", "after `description`")),
                    }
                }
                _ => {
                    return Err(self.error_expected(
                        "`uses`, `requires`, `ensures`, `description` or `}`",
                        "in the function contract",
                    ))
                }
            }
        }
        self.expect(TokenKind::RightBrace, "to close the function contract")?;
        Ok(function)
    }

    fn signature(&mut self) -> ParseResult<Signature> {
        let name = self.identifier("for the function name")?;
        let parameters = self.parameters()?;
        self.expect(TokenKind::Arrow, "before the output type")?;
        let output = self.type_expr()?;
        Ok(Signature {
            name,
            parameters,
            output,
        })
    }

    fn parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        self.expect(TokenKind::LeftParen, "to open the parameter list")?;
        let mut parameters = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            let name = self.identifier("for a parameter")?;
            self.expect(TokenKind::Colon, "after the parameter name")?;
            let ty = self.type_expr()?;
            parameters.push(Parameter { name, ty });
            if !self.advance_if(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RightParen, "to close the parameter list")?;
        Ok(parameters)
    }

    fn type_expr(&mut self) -> ParseResult<TypeExpr> {
        let name = self.identifier("for a type")?;
        let mut arguments = Vec::new();
        if self.advance_if(&TokenKind::Less) {
            loop {
                arguments.push(self.type_expr()?);
                if !self.advance_if(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "to close the type arguments")?;
        }
        Ok(TypeExpr { name, arguments })
    }

    // ---- expressions ----

    fn expression(&mut self) -> ParseResult<Expr> {
        self.binary(0)
    }

    fn binary_op(kind: &TokenKind) -> Option<(BinaryOp, u8)> {
        let pair = match kind {
            TokenKind::Or => (BinaryOp::Or, 1),
            TokenKind::And => (BinaryOp::And, 2),
            TokenKind::EqualEqual => (BinaryOp::Equal, 3),
            TokenKind::NotEqual => (BinaryOp::NotEqual, 3),
            TokenKind::Less => (BinaryOp::Less, 4),
            TokenKind::LessEqual => (BinaryOp::LessEqual, 4),
            TokenKind::Greater => (BinaryOp::Greater, 4),
            TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, 4),
            TokenKind::Plus => (BinaryOp::Add, 5),
            TokenKind::Minus => (BinaryOp::Subtract, 5),
            TokenKind::Star => (BinaryOp::Multiply, 6),
            TokenKind::Slash => (BinaryOp::Divide, 6),
            TokenKind::Percent => (BinaryOp::Remainder, 6),
            _ => return None,
        };
        Some(pair)
    }

    /// Precedence climbing. All binary operators are left-associative.
    fn binary(&mut self, min_precedence: u8) -> ParseResult<Expr> {
        let mut left = self.unary()?;
        while let Some((op, precedence)) = Self::binary_op(self.peek_kind()) {
            if precedence <= min_precedence {
                break;
            }
            let span = self.advance().span;
            let right = self.binary(precedence)?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> ParseResult<Expr> {
        let op = match self.peek_kind() {
            TokenKind::Not => UnaryOp::Not,
            TokenKind::Minus => UnaryOp::Negate,
            _ => return self.postfix(),
        };
        let span = self.advance().span;
        let operand = self.unary()?;
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
        })
    }

    fn postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;
        while self.advance_if(&TokenKind::Dot) {
            let field = self.identifier("after `.`")?;
            let span = field.span;
            expr = Expr {
                kind: ExprKind::Field(Box::new(expr), field),
                span,
            };
        }
        Ok(expr)
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        let token = self.peek().clone();
        let span = token.span;
        let kind = match token.kind {
            TokenKind::Int(value) => {
                self.advance();
                ExprKind::Int(value)
            }
            TokenKind::Float(value) => {
                self.advance();
                ExprKind::Float(value)
            }
            TokenKind::String(value) => {
                self.advance();
                ExprKind::String(value)
            }
            TokenKind::True => {
                self.advance();
                ExprKind::Bool(true)
            }
            TokenKind::False => {
                self.advance();
                ExprKind::Bool(false)
            }
            TokenKind::LeftParen => {
                self.advance();
                let inner = self.expression()?;
                self.expect(TokenKind::RightParen, "to close the parenthesis")?;
                return Ok(inner);
            }
            TokenKind::Identifier(name) => {
                self.advance();
                if self.check(&TokenKind::LeftParen) {
                    let arguments = self.arguments()?;
                    ExprKind::Call(Identifier { name, span }, arguments)
                } else {
                    ExprKind::Name(name)
                }
            }
            _ => return Err(self.error_expected("an expression", "")),
        };
        Ok(Expr { kind, span })
    }

    fn arguments(&mut self) -> ParseResult<Vec<Expr>> {
        self.expect(TokenKind::LeftParen, "to open the argument list")?;
        let mut arguments = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            arguments.push(self.expression()?);
            if !self.advance_if(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RightParen, "to close the argument list")?;
        Ok(arguments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(source: &str) -> Module {
        parse(source, SourceId(0)).unwrap_or_else(|error| panic!("{error:?}"))
    }

    #[test]
    fn parses_struct_with_field_and_struct_constraints() {
        let module = parse_ok(
            "struct Invoice {
                lines: List<Line> is NonEmpty, MaxLength(10)
                total: Money
            } is Balanced",
        );
        let Item::Struct(invoice) = &module.items[0] else {
            panic!("expected a struct");
        };
        assert_eq!(invoice.fields.len(), 2);
        assert_eq!(invoice.fields[0].constraints.len(), 2);
        assert_eq!(invoice.fields[0].constraints[1].arguments.len(), 1);
        assert_eq!(invoice.constraints[0].name.name, "Balanced");
    }

    #[test]
    fn parses_function_contract() {
        let module = parse_ok(
            r#"fn settle(id: InvoiceId) -> Result<Receipt, SettleError> {
                uses InvoiceRepository, PaymentGateway
                requires id.value.length > 0
                ensures output.is_ok || output.is_err
                description "Settles an invoice."
            }"#,
        );
        let Item::Function(function) = &module.items[0] else {
            panic!("expected a function");
        };
        assert_eq!(function.uses.len(), 2);
        assert_eq!(function.requires.len(), 1);
        assert_eq!(function.ensures.len(), 1);
        assert_eq!(function.description.as_deref(), Some("Settles an invoice."));
    }

    #[test]
    fn respects_operator_precedence() {
        let module = parse_ok("const X: Bool = 1 + 2 * 3 == 7 && true");
        let Item::Const(constant) = &module.items[0] else {
            panic!("expected a const");
        };
        let ExprKind::Binary(BinaryOp::And, left, _) = &constant.value.kind else {
            panic!("expected && at the top");
        };
        let ExprKind::Binary(BinaryOp::Equal, sum, _) = &left.kind else {
            panic!("expected == under &&");
        };
        assert!(matches!(sum.kind, ExprKind::Binary(BinaryOp::Add, _, _)));
    }

    #[test]
    fn reports_position_of_errors() {
        let error = parse("struct {", SourceId(0)).unwrap_err();
        assert_eq!((error.span.line, error.span.column), (1, 8));
        assert!(error.message.contains("after `struct`"));
    }
}
