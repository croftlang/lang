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
    .parse_module()
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
            Err(self.expected_error(&kind.describe(), context))
        }
    }

    fn expected_error(&self, expected: &str, context: &str) -> Diagnostic {
        let found = self.peek();
        Diagnostic::new(
            format!(
                "expected {expected} {context}, found {}",
                found.kind.describe()
            ),
            found.span,
        )
    }

    fn parse_identifier(&mut self, context: &str) -> ParseResult<Identifier> {
        match self.peek_kind().clone() {
            TokenKind::Identifier(name) => {
                let span = self.advance().span;
                Ok(Identifier { name, span })
            }
            _ => Err(self.expected_error("a name", context)),
        }
    }

    // ---- items ----

    fn parse_module(mut self) -> ParseResult<Module> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::EndOfFile) {
            items.push(self.parse_item()?);
        }
        Ok(Module { items })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match self.peek_kind() {
            TokenKind::Struct => self.parse_struct().map(Item::Struct),
            TokenKind::Enum => self.parse_enum().map(Item::Enum),
            TokenKind::Const => self.parse_const().map(Item::Const),
            TokenKind::Constraint => self.parse_constraint().map(Item::Constraint),
            TokenKind::Port => self.parse_port().map(Item::Port),
            TokenKind::Adapter => self.parse_adapter().map(Item::Adapter),
            TokenKind::Fn => self.parse_function().map(Item::Function),
            _ => Err(self.expected_error(
                "`struct`, `enum`, `const`, `constraint`, `port`, `adapter` or `fn`",
                "at the start of a declaration",
            )),
        }
    }

    fn parse_struct(&mut self) -> ParseResult<StructDecl> {
        self.expect(TokenKind::Struct, "")?;
        let name = self.parse_identifier("after `struct`")?;
        self.expect(TokenKind::LeftBrace, "to open the struct")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            let field_name = self.parse_identifier("for a field")?;
            self.expect(TokenKind::Colon, "after the field name")?;
            let ty = self.parse_type()?;
            let constraints = self.parse_constraint_uses()?;
            fields.push(Field {
                name: field_name,
                ty,
                constraints,
            });
        }
        self.expect(TokenKind::RightBrace, "to close the struct")?;
        let constraints = self.parse_constraint_uses()?;
        Ok(StructDecl {
            name,
            fields,
            constraints,
        })
    }

    /// Reads an optional `is A, B(1)` list.
    fn parse_constraint_uses(&mut self) -> ParseResult<Vec<ConstraintUse>> {
        let mut uses = Vec::new();
        if !self.advance_if(&TokenKind::Is) {
            return Ok(uses);
        }
        loop {
            let name = self.parse_identifier("for a constraint after `is`")?;
            let arguments = if self.check(&TokenKind::LeftParen) {
                self.parse_arguments()?
            } else {
                Vec::new()
            };
            uses.push(ConstraintUse { name, arguments });
            if !self.advance_if(&TokenKind::Comma) {
                return Ok(uses);
            }
        }
    }

    fn parse_enum(&mut self) -> ParseResult<EnumDecl> {
        self.expect(TokenKind::Enum, "")?;
        let name = self.parse_identifier("after `enum`")?;
        self.expect(TokenKind::LeftBrace, "to open the enum")?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            variants.push(self.parse_identifier("for an enum variant")?);
            self.advance_if(&TokenKind::Comma);
        }
        self.expect(TokenKind::RightBrace, "to close the enum")?;
        Ok(EnumDecl { name, variants })
    }

    fn parse_const(&mut self) -> ParseResult<ConstDecl> {
        self.expect(TokenKind::Const, "")?;
        let name = self.parse_identifier("after `const`")?;
        self.expect(TokenKind::Colon, "after the constant name")?;
        let ty = self.parse_type()?;
        self.expect(TokenKind::Equal, "before the constant value")?;
        let value = self.parse_expression()?;
        Ok(ConstDecl { name, ty, value })
    }

    fn parse_constraint(&mut self) -> ParseResult<ConstraintDecl> {
        self.expect(TokenKind::Constraint, "")?;
        let name = self.parse_identifier("after `constraint`")?;
        let mut generics = Vec::new();
        if self.advance_if(&TokenKind::Less) {
            loop {
                generics.push(self.parse_identifier("for a type parameter")?);
                if !self.advance_if(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "to close the type parameters")?;
        }
        let parameters = self.parse_parameters()?;
        self.expect(TokenKind::Equal, "before the constraint body")?;
        let body = self.parse_expression()?;
        Ok(ConstraintDecl {
            name,
            generics,
            parameters,
            body,
        })
    }

    fn parse_port(&mut self) -> ParseResult<PortDecl> {
        self.expect(TokenKind::Port, "")?;
        let name = self.parse_identifier("after `port`")?;
        self.expect(TokenKind::LeftBrace, "to open the port")?;
        let mut operations = Vec::new();
        while !self.check(&TokenKind::RightBrace) {
            self.expect(TokenKind::Fn, "to start a port operation")?;
            operations.push(self.parse_signature()?);
        }
        self.expect(TokenKind::RightBrace, "to close the port")?;
        Ok(PortDecl { name, operations })
    }

    fn parse_adapter(&mut self) -> ParseResult<AdapterDecl> {
        self.expect(TokenKind::Adapter, "")?;
        let name = self.parse_identifier("after `adapter`")?;
        self.expect(TokenKind::For, "after the adapter name")?;
        let port = self.parse_identifier("for the port this adapter implements")?;
        Ok(AdapterDecl { name, port })
    }

    fn parse_function(&mut self) -> ParseResult<FunctionDecl> {
        self.expect(TokenKind::Fn, "")?;
        let signature = self.parse_signature()?;
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
                            .push(self.parse_identifier("for a port after `uses`")?);
                        if !self.advance_if(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
                "requires" => {
                    self.advance();
                    function.requires.push(self.parse_expression()?);
                }
                "ensures" => {
                    self.advance();
                    function.ensures.push(self.parse_expression()?);
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
                        _ => return Err(self.expected_error("a string", "after `description`")),
                    }
                }
                _ => {
                    return Err(self.expected_error(
                        "`uses`, `requires`, `ensures`, `description` or `}`",
                        "in the function contract",
                    ))
                }
            }
        }
        self.expect(TokenKind::RightBrace, "to close the function contract")?;
        Ok(function)
    }

    fn parse_signature(&mut self) -> ParseResult<Signature> {
        let name = self.parse_identifier("for the function name")?;
        let parameters = self.parse_parameters()?;
        self.expect(TokenKind::Arrow, "before the output type")?;
        let output = self.parse_type()?;
        Ok(Signature {
            name,
            parameters,
            output,
        })
    }

    fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        self.expect(TokenKind::LeftParen, "to open the parameter list")?;
        let mut parameters = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            let name = self.parse_identifier("for a parameter")?;
            self.expect(TokenKind::Colon, "after the parameter name")?;
            let ty = self.parse_type()?;
            parameters.push(Parameter { name, ty });
            if !self.advance_if(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RightParen, "to close the parameter list")?;
        Ok(parameters)
    }

    fn parse_type(&mut self) -> ParseResult<TypeExpr> {
        let name = self.parse_identifier("for a type")?;
        let mut arguments = Vec::new();
        if self.advance_if(&TokenKind::Less) {
            loop {
                arguments.push(self.parse_type()?);
                if !self.advance_if(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Greater, "to close the type arguments")?;
        }
        Ok(TypeExpr { name, arguments })
    }

    // ---- expressions ----

    fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_binary(0)
    }

    fn binary_operator(kind: &TokenKind) -> Option<(BinaryOp, u8)> {
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
    fn parse_binary(&mut self, min_precedence: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary()?;
        while let Some((op, precedence)) = Self::binary_operator(self.peek_kind()) {
            if precedence <= min_precedence {
                break;
            }
            let span = self.advance().span;
            let right = self.parse_binary(precedence)?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> ParseResult<Expr> {
        let op = match self.peek_kind() {
            TokenKind::Not => UnaryOp::Not,
            TokenKind::Minus => UnaryOp::Negate,
            _ => return self.parse_postfix(),
        };
        let span = self.advance().span;
        let operand = self.parse_unary()?;
        Ok(Expr {
            kind: ExprKind::Unary(op, Box::new(operand)),
            span,
        })
    }

    fn parse_postfix(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary()?;
        while self.advance_if(&TokenKind::Dot) {
            let field = self.parse_identifier("after `.`")?;
            let span = field.span;
            expr = Expr {
                kind: ExprKind::Field(Box::new(expr), field),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> ParseResult<Expr> {
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
                let inner = self.parse_expression()?;
                self.expect(TokenKind::RightParen, "to close the parenthesis")?;
                return Ok(inner);
            }
            TokenKind::Identifier(name) => {
                self.advance();
                if self.check(&TokenKind::LeftParen) {
                    let arguments = self.parse_arguments()?;
                    ExprKind::Call(Identifier { name, span }, arguments)
                } else {
                    ExprKind::Name(name)
                }
            }
            _ => return Err(self.expected_error("an expression", "")),
        };
        Ok(Expr { kind, span })
    }

    fn parse_arguments(&mut self) -> ParseResult<Vec<Expr>> {
        self.expect(TokenKind::LeftParen, "to open the argument list")?;
        let mut arguments = Vec::new();
        while !self.check(&TokenKind::RightParen) {
            arguments.push(self.parse_expression()?);
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
