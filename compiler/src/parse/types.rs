//! Type parsing.

use super::*;

impl Parser<'_> {
    /// Parses a value type: a type atom with an optional `?` suffix.
    pub(super) fn parse_type(&mut self) -> Type {
        let start = self.current_start();
        let kind = self.parse_type_atom();
        let nullable = self.eat_punct(Punct::Question);
        Type {
            kind,
            nullable,
            span: self.span_from(start),
        }
    }

    /// Parses `void` or a value type.
    pub(super) fn parse_return_type(&mut self) -> Type {
        let start = self.current_start();
        if self.at_keyword(Keyword::Void) {
            self.bump();
            return Type {
                kind: TypeKind::Void,
                nullable: false,
                span: self.span_from(start),
            };
        }
        self.parse_type()
    }

    /// Parses a type atom and any bracketed extents behind it.
    ///
    /// `raw` binds to the immediately following atom, so `raw u8[4]` becomes
    /// an array of four raw bytes.
    fn parse_type_atom(&mut self) -> TypeKind {
        let (base, base_span) = self.parse_type_atom_base();
        let mut extents = Vec::new();
        while self.at_punct(Punct::LBracket) {
            let start = self.current_start();
            self.bump();
            let size = if self.at_punct(Punct::RBracket) {
                None
            } else {
                Some(self.parse_expression())
            };
            self.expect_punct(Punct::RBracket);
            extents.push(Extent {
                size,
                span: self.span_from(start),
            });
        }
        if extents.is_empty() {
            base
        } else {
            TypeKind::Array {
                element: Box::new(Type {
                    kind: base,
                    nullable: false,
                    span: base_span,
                }),
                extents,
            }
        }
    }

    /// Parses one type atom without its extents.
    fn parse_type_atom_base(&mut self) -> (TypeKind, Span) {
        let start = self.current_start();
        let kind = match self.peek_kind().clone() {
            TokenKind::Keyword(keyword) => {
                if let Some(scalar) = super::scalar_type_of(keyword) {
                    self.bump();
                    TypeKind::Scalar(scalar)
                } else {
                    match keyword {
                        Keyword::Str => {
                            self.bump();
                            TypeKind::Str
                        }
                        Keyword::Cstring => {
                            self.bump();
                            TypeKind::CString
                        }
                        Keyword::Static => {
                            self.bump();
                            self.expect_keyword(Keyword::Str);
                            TypeKind::StaticStr
                        }
                        Keyword::Raw => {
                            self.bump();
                            TypeKind::Raw(Box::new(self.parse_raw_target()))
                        }
                        Keyword::Func => self.parse_function_type(),
                        _ => {
                            self.unexpected();
                            TypeKind::Error
                        }
                    }
                }
            }
            TokenKind::Punct(Punct::LParen) => {
                self.bump();
                let kind = self.parse_function_type();
                self.expect_punct(Punct::RParen);
                kind
            }
            TokenKind::Ident(_) => {
                let name = self.parse_qualified_name();
                let args = if self.at_punct(Punct::Lt) {
                    self.parse_type_args()
                } else {
                    Vec::new()
                };
                TypeKind::Named { name, args }
            }
            _ => {
                self.unexpected();
                TypeKind::Error
            }
        };
        (kind, self.span_from(start))
    }

    /// Parses the target atom of a `raw` type.
    pub(super) fn parse_raw_target(&mut self) -> Type {
        let start = self.current_start();
        let kind = match self.peek_kind().clone() {
            TokenKind::Keyword(keyword) => {
                if let Some(scalar) = super::scalar_type_of(keyword) {
                    self.bump();
                    TypeKind::Scalar(scalar)
                } else if keyword == Keyword::Str {
                    self.bump();
                    TypeKind::Str
                } else if keyword == Keyword::Cstring {
                    self.bump();
                    TypeKind::CString
                } else {
                    self.unexpected();
                    TypeKind::Error
                }
            }
            TokenKind::Ident(_) => {
                let name = self.parse_qualified_name();
                let args = if self.at_punct(Punct::Lt) {
                    self.parse_type_args()
                } else {
                    Vec::new()
                };
                TypeKind::Named { name, args }
            }
            _ => {
                self.unexpected();
                TypeKind::Error
            }
        };
        Type {
            kind,
            nullable: false,
            span: self.span_from(start),
        }
    }

    /// Parses `func(types): return`.
    fn parse_function_type(&mut self) -> TypeKind {
        self.expect_keyword(Keyword::Func);
        self.expect_punct(Punct::LParen);
        let mut params = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                let start = self.current_start();
                let mode = self.parse_mode();
                let ty = self.parse_type();
                params.push(FunctionTypeParam {
                    mode,
                    ty,
                    span: self.span_from(start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        self.expect_punct(Punct::Colon);
        let returns = self.parse_return_type();
        TypeKind::Function {
            params,
            returns: Box::new(returns),
        }
    }

    /// Parses explicit type arguments `<T, ...>`.
    pub(super) fn parse_type_args(&mut self) -> Vec<Type> {
        let mut args = Vec::new();
        self.expect_punct(Punct::Lt);
        loop {
            args.push(self.parse_type());
            if !self.eat_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::Gt);
        args
    }

    /// Tries to parse type arguments without recording diagnostics.
    ///
    /// Used in expression position, where `<` is type arguments only when the
    /// list closes with `>` directly before `(` or `{`.
    pub(super) fn try_type_args(&mut self) -> Option<Vec<Type>> {
        self.speculate(Self::parse_type_args)
    }

    /// Parses an optional `<T: bound, ...>` parameter list.
    pub(super) fn parse_type_params(&mut self) -> Vec<TypeParam> {
        let mut params = Vec::new();
        if !self.at_punct(Punct::Lt) {
            return params;
        }
        self.bump();
        loop {
            let start = self.current_start();
            let (name, _) = self.parse_declared_name(false);
            let bound = if self.eat_punct(Punct::Colon) {
                Some(self.parse_callable_bound())
            } else {
                None
            };
            params.push(TypeParam {
                name,
                bound,
                span: self.span_from(start),
            });
            if !self.eat_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::Gt);
        params
    }

    /// Parses `func(types): return` inside a type-parameter bound.
    fn parse_callable_bound(&mut self) -> CallableBound {
        let start = self.current_start();
        self.expect_keyword(Keyword::Func);
        self.expect_punct(Punct::LParen);
        let mut params = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                params.push(self.parse_type());
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        self.expect_punct(Punct::Colon);
        let returns = self.parse_return_type();
        CallableBound {
            params,
            returns: Box::new(returns),
            span: self.span_from(start),
        }
    }
}
