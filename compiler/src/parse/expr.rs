//! Expression parsing: precedence climbing plus the primary forms.

use super::*;
use crate::lex::{TemplatePart, TokenKind};

impl Parser<'_> {
    /// Parses a full expression, including assignment.
    pub(super) fn parse_expression(&mut self) -> Expr {
        self.parse_assignment()
    }

    /// Parses assignment, which is right-associative.
    fn parse_assignment(&mut self) -> Expr {
        let start = self.current_start();
        let target = self.parse_conditional();
        let Some(op) = self.assign_op() else {
            return target;
        };
        self.bump();
        let value = self.parse_assignment();
        Expr {
            kind: ExprKind::Assign {
                op,
                target: Box::new(target),
                value: Box::new(value),
            },
            span: self.span_from(start),
        }
    }

    /// Assignment operator at the cursor, if any.
    fn assign_op(&self) -> Option<AssignOp> {
        let op = match self.peek_kind() {
            TokenKind::Punct(punct) => match punct {
                Punct::Eq => AssignOp::Assign,
                Punct::PlusEq => AssignOp::Add,
                Punct::MinusEq => AssignOp::Sub,
                Punct::StarEq => AssignOp::Mul,
                Punct::SlashEq => AssignOp::Div,
                Punct::PercentEq => AssignOp::Rem,
                Punct::AmpEq => AssignOp::BitAnd,
                Punct::PipeEq => AssignOp::BitOr,
                Punct::CaretEq => AssignOp::BitXor,
                Punct::ShlEq => AssignOp::Shl,
                Punct::ShrEq => AssignOp::Shr,
                _ => return None,
            },
            _ => return None,
        };
        Some(op)
    }

    /// Parses the conditional `? :` operator, which is right-associative.
    fn parse_conditional(&mut self) -> Expr {
        let start = self.current_start();
        let condition = self.parse_binary(1);
        if !self.eat_punct(Punct::Question) {
            return condition;
        }
        let then_value = self.parse_expression();
        self.expect_punct(Punct::Colon);
        let else_value = self.parse_conditional();
        Expr {
            kind: ExprKind::Conditional {
                condition: Box::new(condition),
                then_value: Box::new(then_value),
                else_value: Box::new(else_value),
            },
            span: self.span_from(start),
        }
    }

    /// Precedence-climbing loop over the binary operator table.
    fn parse_binary(&mut self, min_precedence: u8) -> Expr {
        let start = self.current_start();
        let mut left = self.parse_unary();
        while let Some(op) = self.binary_op() {
            let precedence = op.precedence();
            if precedence < min_precedence {
                break;
            }
            self.bump();
            let right = self.parse_binary(precedence + 1);
            left = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span: self.span_from(start),
            };
        }
        left
    }

    /// Binary operator at the cursor, if any.
    fn binary_op(&self) -> Option<BinaryOp> {
        let op = match self.peek_kind() {
            TokenKind::Punct(Punct::PipePipe) => BinaryOp::LogicalOr,
            TokenKind::Punct(Punct::AmpAmp) => BinaryOp::LogicalAnd,
            TokenKind::Punct(Punct::Pipe) => BinaryOp::BitOr,
            TokenKind::Punct(Punct::Caret) => BinaryOp::BitXor,
            TokenKind::Punct(Punct::Amp) => BinaryOp::BitAnd,
            TokenKind::Punct(Punct::EqEq) => BinaryOp::Eq,
            TokenKind::Punct(Punct::BangEq) => BinaryOp::Ne,
            TokenKind::Punct(Punct::Lt) => BinaryOp::Lt,
            TokenKind::Punct(Punct::LtEq) => BinaryOp::Le,
            TokenKind::Punct(Punct::Gt) => BinaryOp::Gt,
            TokenKind::Punct(Punct::GtEq) => BinaryOp::Ge,
            TokenKind::Punct(Punct::Shl) => BinaryOp::Shl,
            TokenKind::Punct(Punct::Shr) => BinaryOp::Shr,
            TokenKind::Punct(Punct::Plus) => BinaryOp::Add,
            TokenKind::Punct(Punct::Minus) => BinaryOp::Sub,
            TokenKind::Punct(Punct::Star) => BinaryOp::Mul,
            TokenKind::Punct(Punct::Slash) => BinaryOp::Div,
            TokenKind::Punct(Punct::Percent) => BinaryOp::Rem,
            _ => return None,
        };
        Some(op)
    }

    /// Parses prefix operators, `unsafe` expressions, and postfix chains.
    fn parse_unary(&mut self) -> Expr {
        let start = self.current_start();
        if self.at_keyword(Keyword::Unsafe) {
            self.bump();
            let operand = self.parse_unary();
            return Expr {
                kind: ExprKind::Unsafe(Box::new(operand)),
                span: self.span_from(start),
            };
        }
        let op = match self.peek_kind() {
            TokenKind::Punct(Punct::Bang) => Some(UnaryOp::Not),
            TokenKind::Punct(Punct::Tilde) => Some(UnaryOp::BitNot),
            TokenKind::Punct(Punct::Minus) => Some(UnaryOp::Neg),
            TokenKind::Punct(Punct::Amp) => Some(UnaryOp::AddressOf),
            TokenKind::Punct(Punct::Star) => Some(UnaryOp::Deref),
            TokenKind::Punct(Punct::PlusPlus) => Some(UnaryOp::PreIncrement),
            TokenKind::Punct(Punct::MinusMinus) => Some(UnaryOp::PreDecrement),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let operand = self.parse_unary();
            return Expr {
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
                span: self.span_from(start),
            };
        }
        self.parse_postfix()
    }

    /// Parses a primary expression followed by its postfix operators.
    fn parse_postfix(&mut self) -> Expr {
        let start = self.current_start();
        let mut expr = self.parse_primary();
        loop {
            if self.at_punct(Punct::LParen) {
                let args = self.parse_call_args();
                expr = Expr {
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        type_args: Vec::new(),
                        args,
                    },
                    span: self.span_from(start),
                };
            } else if self.at_punct(Punct::LBracket) {
                self.bump();
                let index = self.parse_expression();
                self.expect_punct(Punct::RBracket);
                expr = Expr {
                    kind: ExprKind::Index {
                        base: Box::new(expr),
                        index: Box::new(index),
                    },
                    span: self.span_from(start),
                };
            } else if self.at_punct(Punct::Dot) {
                self.bump();
                let Some((name, _)) = self.expect_ident() else {
                    break;
                };
                expr = Expr {
                    kind: ExprKind::Member {
                        base: Box::new(expr),
                        name,
                    },
                    span: self.span_from(start),
                };
            } else if self.at_punct(Punct::PlusPlus) {
                self.bump();
                expr = self.postfix_update(expr, UnaryOp::PostIncrement, start);
            } else if self.at_punct(Punct::MinusMinus) {
                self.bump();
                expr = self.postfix_update(expr, UnaryOp::PostDecrement, start);
            } else {
                break;
            }
        }
        expr
    }

    /// Wraps an expression in a postfix `++`/`--` node.
    fn postfix_update(&self, expr: Expr, op: UnaryOp, start: usize) -> Expr {
        Expr {
            kind: ExprKind::Unary {
                op,
                operand: Box::new(expr),
            },
            span: self.span_from(start),
        }
    }

    /// Parses `( arguments )`.
    fn parse_call_args(&mut self) -> Vec<Arg> {
        let mut args = Vec::new();
        self.expect_punct(Punct::LParen);
        if !self.at_punct(Punct::RParen) {
            loop {
                let start = self.current_start();
                let marker = self.parse_mode();
                let value = self.parse_expression();
                args.push(Arg {
                    marker,
                    value,
                    span: self.span_from(start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        args
    }

    /// Parses one primary expression.
    fn parse_primary(&mut self) -> Expr {
        let start = self.current_start();
        let kind = match self.peek_kind().clone() {
            TokenKind::Int(value) => {
                self.bump();
                ExprKind::Int(value)
            }
            TokenKind::Float(value) => {
                self.bump();
                ExprKind::Float(value)
            }
            TokenKind::Char(value) => {
                self.bump();
                ExprKind::Char(value)
            }
            TokenKind::Str(bytes) => {
                self.bump();
                ExprKind::Str(bytes)
            }
            TokenKind::Template(parts) => {
                self.bump();
                return self.build_template(parts, start);
            }
            TokenKind::Keyword(Keyword::True) => {
                self.bump();
                ExprKind::Bool(true)
            }
            TokenKind::Keyword(Keyword::False) => {
                self.bump();
                ExprKind::Bool(false)
            }
            TokenKind::Keyword(Keyword::Null) => {
                self.bump();
                ExprKind::Null
            }
            TokenKind::Keyword(Keyword::Raw) => return self.parse_raw_cast(start),
            TokenKind::Keyword(keyword) if super::scalar_type_of(keyword).is_some() => {
                return self.parse_scalar_cast(start);
            }
            TokenKind::Punct(Punct::LParen) => {
                if let Some(lambda) = self.try_lambda() {
                    return lambda;
                }
                self.bump();
                let inner = self.parse_expression();
                self.expect_punct(Punct::RParen);
                return Expr {
                    kind: inner.kind,
                    span: self.span_from(start),
                };
            }
            TokenKind::Punct(Punct::LBracket) => {
                if let Some(lambda) = self.try_lambda() {
                    return lambda;
                }
                return self.parse_array_literal(start);
            }
            TokenKind::Ident(name) => return self.parse_name_expression(name, start),
            _ => {
                self.unexpected();
                self.bump();
                ExprKind::Error
            }
        };
        Expr {
            kind,
            span: self.span_from(start),
        }
    }

    /// Parses a scalar cast `T(expression)`.
    fn parse_scalar_cast(&mut self, start: usize) -> Expr {
        let scalar = match self.peek_kind() {
            TokenKind::Keyword(keyword) => super::scalar_type_of(*keyword),
            _ => None,
        };
        let Some(scalar) = scalar else {
            self.unexpected();
            return Expr {
                kind: ExprKind::Error,
                span: self.span_from(start),
            };
        };
        self.bump();
        if !self.at_punct(Punct::LParen) {
            self.unexpected();
            return Expr {
                kind: ExprKind::Error,
                span: self.span_from(start),
            };
        }
        self.bump();
        let value = self.parse_expression();
        self.expect_punct(Punct::RParen);
        let ty = Type {
            kind: TypeKind::Scalar(scalar),
            nullable: false,
            span: self.span_from(start),
        };
        Expr {
            kind: ExprKind::Cast {
                ty,
                value: Box::new(value),
            },
            span: self.span_from(start),
        }
    }

    /// Parses a raw-value cast `raw T(expression)`.
    fn parse_raw_cast(&mut self, start: usize) -> Expr {
        self.bump();
        let ty = self.parse_raw_target();
        self.expect_punct(Punct::LParen);
        let value = self.parse_expression();
        self.expect_punct(Punct::RParen);
        Expr {
            kind: ExprKind::RawCast {
                ty,
                value: Box::new(value),
            },
            span: self.span_from(start),
        }
    }

    /// Parses a name-led expression: plain names, dotted names, calls,
    /// generic references, struct literals, and intrinsic calls.
    fn parse_name_expression(&mut self, first: String, start: usize) -> Expr {
        if crate::token::is_reserved_builtin(&first)
            && matches!(self.kind_at(1), TokenKind::Punct(Punct::LParen))
        {
            return self.parse_intrinsic_call(first, start);
        }
        self.bump();
        let mut segments = vec![first];
        while self.at_punct(Punct::Dot) && matches!(self.kind_at(1), TokenKind::Ident(_)) {
            self.bump();
            if let TokenKind::Ident(segment) = self.peek_kind().clone() {
                segments.push(segment);
                self.bump();
            }
        }
        let name = QualifiedName {
            segments,
            span: self.span_from(start),
        };
        if self.at_punct(Punct::Lt) {
            if let Some(args) = self.try_type_args() {
                if self.at_punct(Punct::LParen) {
                    let call_args = self.parse_call_args();
                    let callee = self.name_expression(name, start);
                    return Expr {
                        kind: ExprKind::Call {
                            callee: Box::new(callee),
                            type_args: args,
                            args: call_args,
                        },
                        span: self.span_from(start),
                    };
                }
                if self.at_punct(Punct::LBrace) {
                    let fields = self.parse_field_inits();
                    return Expr {
                        kind: ExprKind::StructLiteral {
                            name,
                            type_args: args,
                            fields,
                        },
                        span: self.span_from(start),
                    };
                }
                return Expr {
                    kind: ExprKind::GenericRef { name, args },
                    span: self.span_from(start),
                };
            }
        }
        if self.at_punct(Punct::LBrace) {
            let fields = self.parse_field_inits();
            return Expr {
                kind: ExprKind::StructLiteral {
                    name,
                    type_args: Vec::new(),
                    fields,
                },
                span: self.span_from(start),
            };
        }
        self.name_expression(name, start)
    }

    /// Builds a `Name` or `Qualified` expression for a dotted name.
    fn name_expression(&self, name: QualifiedName, start: usize) -> Expr {
        let kind = if name.segments.len() == 1 {
            ExprKind::Name(name.segments[0].clone())
        } else {
            ExprKind::Qualified(name)
        };
        Expr {
            kind,
            span: self.span_from(start),
        }
    }

    /// Parses `{ field: value, ... }`.
    fn parse_field_inits(&mut self) -> Vec<FieldInit> {
        let mut fields = Vec::new();
        self.expect_punct(Punct::LBrace);
        if !self.at_punct(Punct::RBrace) {
            loop {
                let start = self.current_start();
                let (name, name_span) = self.parse_declared_name(false);
                self.expect_punct(Punct::Colon);
                let value = self.parse_expression();
                fields.push(FieldInit {
                    name,
                    name_span,
                    value,
                    span: self.span_from(start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
                if self.at_punct(Punct::RBrace) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RBrace);
        fields
    }

    /// Parses an array literal in list or fill form.
    fn parse_array_literal(&mut self, start: usize) -> Expr {
        self.expect_punct(Punct::LBracket);
        let mut elements = Vec::new();
        let mut fill = None;
        if self.eat_punct(Punct::Semi) {
            fill = Some(Box::new(self.parse_expression()));
        } else if !self.at_punct(Punct::RBracket) {
            elements.push(self.parse_expression());
            while self.eat_punct(Punct::Comma) {
                if self.at_punct(Punct::RBracket) {
                    break;
                }
                elements.push(self.parse_expression());
            }
            if self.eat_punct(Punct::Semi) {
                fill = Some(Box::new(self.parse_expression()));
            }
        }
        self.expect_punct(Punct::RBracket);
        Expr {
            kind: ExprKind::ArrayLiteral { elements, fill },
            span: self.span_from(start),
        }
    }

    /// Parses a call of a reserved built-in name.
    ///
    /// `sizeof` and `offsetof` take type-shaped arguments; the other
    /// intrinsics take expressions.
    fn parse_intrinsic_call(&mut self, name: String, start: usize) -> Expr {
        self.bump();
        self.expect_punct(Punct::LParen);
        let mut args = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                let arg = match name.as_str() {
                    "sizeof" => IntrinsicArg::Type(self.parse_type()),
                    "offsetof" if args.is_empty() => IntrinsicArg::Type(self.parse_type()),
                    _ => IntrinsicArg::Expr(self.parse_expression()),
                };
                args.push(arg);
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        Expr {
            kind: ExprKind::IntrinsicCall { name, args },
            span: self.span_from(start),
        }
    }

    /// Builds a parsed template expression from the lexer's pieces.
    fn build_template(&mut self, parts: Vec<TemplatePart>, start: usize) -> Expr {
        let mut pieces = Vec::new();
        for part in parts {
            match part {
                TemplatePart::Literal(bytes) => pieces.push(TemplatePiece::Literal(bytes)),
                TemplatePart::Placeholder(span) => {
                    let expr = self.parse_placeholder(span);
                    pieces.push(TemplatePiece::Placeholder(expr));
                }
            }
        }
        Expr {
            kind: ExprKind::Template(pieces),
            span: self.span_from(start),
        }
    }

    /// Parses the expression inside one template placeholder.
    ///
    /// A top-level `:` is the deferred format specifier and reports `E10501`
    /// here, before any semantic stage runs.
    fn parse_placeholder(&mut self, span: Span) -> Expr {
        let tokens: Vec<Token> = Lexer::range(self.sources, self.file, span, self.diagnostics)
            .lex()
            .into_iter()
            .filter(|token| token.kind != TokenKind::Error)
            .collect();
        let mut inner = Parser {
            tokens,
            pos: 0,
            sources: self.sources,
            file: self.file,
            diagnostics: &mut *self.diagnostics,
            speculation: 0,
            speculation_failed: false,
        };
        let expr = inner.parse_expression();
        if inner.at_punct(Punct::Colon) {
            let colon = inner.peek().span;
            inner.report(
                Diagnostic::error("E10501", "format specifiers are not supported in v1", colon)
                    .with_help("use a conversion function"),
            );
            while !inner.at_eof() {
                inner.bump();
            }
        }
        if !inner.at_eof() {
            inner.unexpected();
        }
        expr
    }

    /// Tries to parse a lambda without committing, for `(` and `[` starts.
    fn try_lambda(&mut self) -> Option<Expr> {
        let plausible = match self.peek_kind() {
            TokenKind::Punct(Punct::LParen) => {
                matches!(self.kind_at(1), TokenKind::Punct(Punct::RParen))
                    || (matches!(self.kind_at(1), TokenKind::Ident(_))
                        && matches!(self.kind_at(2), TokenKind::Punct(Punct::Colon)))
            }
            TokenKind::Punct(Punct::LBracket) => true,
            _ => false,
        };
        if !plausible {
            return None;
        }
        self.speculate(Self::parse_lambda)
    }

    /// Parses `[captures](params) => body`.
    fn parse_lambda(&mut self) -> Expr {
        let start = self.current_start();
        let captures = if self.at_punct(Punct::LBracket) {
            self.parse_captures()
        } else {
            Vec::new()
        };
        self.expect_punct(Punct::LParen);
        let mut params = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                let param_start = self.current_start();
                let (name, name_span) = self.parse_declared_name(false);
                self.expect_punct(Punct::Colon);
                let ty = self.parse_type();
                params.push(LambdaParam {
                    name,
                    name_span,
                    ty,
                    span: self.span_from(param_start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        self.expect_punct(Punct::FatArrow);
        let body = if self.at_punct(Punct::LBrace) {
            LambdaBody::Block(self.parse_block())
        } else {
            LambdaBody::Expr(Box::new(self.parse_expression()))
        };
        Expr {
            kind: ExprKind::Lambda {
                captures,
                params,
                body,
            },
            span: self.span_from(start),
        }
    }

    /// Parses `[capture, ...]`.
    fn parse_captures(&mut self) -> Vec<Capture> {
        let mut captures = Vec::new();
        self.expect_punct(Punct::LBracket);
        if !self.at_punct(Punct::RBracket) {
            loop {
                let start = self.current_start();
                let mode = if self.eat_keyword(Keyword::Borrow) {
                    Mode::Borrow
                } else if self.eat_keyword(Keyword::Take) {
                    Mode::Take
                } else {
                    Mode::None
                };
                let (name, _) = self.parse_declared_name(false);
                captures.push(Capture {
                    mode,
                    name,
                    span: self.span_from(start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RBracket);
        captures
    }
}
