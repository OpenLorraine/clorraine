//! Declaration parsing: module declaration, imports, and top-level items.

use super::*;

impl Parser<'_> {
    /// Module and imports

    /// Parses the leading `module` declaration.
    ///
    /// A missing declaration produces a placeholder with an empty name; the
    /// module stage reports it so this stage stays syntax-only.
    pub(super) fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.current_start();
        if !self.eat_keyword(Keyword::Module) {
            return ModuleDecl {
                name: QualifiedName {
                    segments: Vec::new(),
                    span: Span::new(self.file, 0, 0),
                },
                span: Span::new(self.file, 0, 0),
            };
        }
        let name = self.parse_qualified_name();
        self.expect_punct(Punct::Semi);
        ModuleDecl {
            name,
            span: self.span_from(start),
        }
    }

    /// Parses one import statement.
    pub(super) fn parse_import(&mut self) -> Option<ImportDecl> {
        let start = self.current_start();
        self.expect_keyword(Keyword::Import);
        self.expect_punct(Punct::LBrace);
        let mut items = Vec::new();
        if !self.at_punct(Punct::RBrace) {
            loop {
                let item_start = self.current_start();
                let (name, name_span) = self.parse_declared_name(false);
                let alias = if self.at_ident("as") {
                    self.bump();
                    let (alias, _) = self.parse_declared_name(false);
                    Some(alias)
                } else {
                    None
                };
                items.push(ImportItem {
                    name,
                    name_span,
                    alias,
                    span: self.span_from(item_start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RBrace);
        self.expect_keyword(Keyword::From);
        let from = self.parse_qualified_name();
        self.expect_punct(Punct::Semi);
        Some(ImportDecl {
            items,
            from,
            span: self.span_from(start),
        })
    }

    /// Declarations

    /// Parses one top-level item, or reports and returns `None`.
    pub(super) fn parse_item(&mut self) -> Option<Item> {
        let exported = self.eat_keyword(Keyword::Export);
        let item = match self.peek_kind() {
            TokenKind::Keyword(Keyword::Func) => Some(Item::Func(self.parse_func(exported))),
            TokenKind::Keyword(Keyword::Comptime) => {
                self.bump();
                Some(Item::Comptime(self.parse_func(exported)))
            }
            TokenKind::Keyword(Keyword::Struct) => Some(Item::Struct(self.parse_struct(exported))),
            TokenKind::Keyword(Keyword::Enum) => Some(Item::Enum(self.parse_enum(exported))),
            TokenKind::Keyword(Keyword::Handle) => Some(Item::Handle(self.parse_handle(exported))),
            TokenKind::Keyword(Keyword::Ffi) => Some(Item::Ffi(self.parse_ffi_block(exported))),
            TokenKind::Keyword(Keyword::Intrinsic) => Some(Item::Intrinsic(self.parse_intrinsic())),
            TokenKind::Keyword(Keyword::Let) => Some(Item::GlobalLet(self.parse_global_let(exported, false))),
            TokenKind::Keyword(Keyword::Volatile) => {
                if matches!(self.kind_at(1), TokenKind::Keyword(Keyword::Let)) {
                    self.bump();
                    Some(Item::GlobalLet(self.parse_global_let(exported, true)))
                } else {
                    self.volatile_here();
                    self.bump();
                    self.parse_item()
                }
            }
            TokenKind::Keyword(Keyword::Const) => {
                Some(Item::GlobalConst(self.parse_global_const(exported)))
            }
            TokenKind::Error => {
                self.bump();
                None
            }
            _ => {
                self.unexpected();
                None
            }
        };
        item
    }

    /// Reports the misplaced-`volatile` diagnostic with its help text.
    pub(super) fn volatile_here(&mut self) {
        self.report(
            Diagnostic::error(
                "E10208",
                "'volatile' is not allowed here",
                self.peek().span,
            )
            .with_help("declare it on a field or a module `let`"),
        );
    }

    /// True when the cursor is at a `union` declaration start.
    pub(super) fn at_union_decl(&self) -> bool {
        self.at_ident("union") && matches!(self.kind_at(1), TokenKind::Ident(_))
    }

    /// Reports the tagged-union diagnostic and skips the declaration.
    pub(super) fn parse_union_decl(&mut self) {
        let start = self.current_start();
        self.bump();
        if let Some((_, name_span)) = self.expect_ident() {
            let span = Span::new(self.file, start, name_span.end as usize);
            self.report(
                Diagnostic::error("E10204", "tagged unions are not part of Clorraine", span)
                    .with_help("model the data as a C-style tagged struct and use `switch`"),
            );
        }
        if self.at_punct(Punct::LBrace) {
            self.skip_braced_block();
        } else {
            self.skip_to_semicolon();
        }
    }

    /// Parses a `func` declaration; the leading keyword is at the cursor.
    fn parse_func(&mut self, exported: bool) -> FuncDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Func);
        let (name, name_span) = self.parse_declared_name(false);
        let type_params = self.parse_type_params();
        let params = self.parse_params();
        self.expect_punct(Punct::Colon);
        let returns = self.parse_return_type();
        let body = self.parse_block();
        FuncDecl {
            exported,
            name,
            name_span,
            type_params,
            params,
            returns,
            body,
            span: self.span_from(start),
        }
    }

    /// Parses `( parameters )`.
    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        self.expect_punct(Punct::LParen);
        if !self.at_punct(Punct::RParen) {
            loop {
                let start = self.current_start();
                let mode = self.parse_mode();
                let (name, name_span) = self.parse_declared_name(false);
                self.expect_punct(Punct::Colon);
                let ty = self.parse_type();
                params.push(Param {
                    mode,
                    name,
                    name_span,
                    ty,
                    span: self.span_from(start),
                });
                if !self.eat_punct(Punct::Comma) {
                    break;
                }
            }
        }
        self.expect_punct(Punct::RParen);
        params
    }

    /// Parses `struct` fields.
    fn parse_struct(&mut self, exported: bool) -> StructDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Struct);
        let (name, name_span) = self.parse_declared_name(false);
        let type_params = self.parse_type_params();
        let freed_by = if self.eat_keyword(Keyword::FreedBy) {
            Some(self.parse_qualified_name())
        } else {
            None
        };
        let mut fields = Vec::new();
        self.expect_punct(Punct::LBrace);
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            let field_start = self.current_start();
            let (field_name, field_name_span) = self.parse_declared_name(false);
            self.expect_punct(Punct::Colon);
            let volatile = self.eat_keyword(Keyword::Volatile);
            let ty = self.parse_type();
            self.expect_punct(Punct::Semi);
            fields.push(StructField {
                name: field_name,
                name_span: field_name_span,
                volatile,
                ty,
                span: self.span_from(field_start),
            });
        }
        self.expect_punct(Punct::RBrace);
        StructDecl {
            exported,
            name,
            name_span,
            type_params,
            freed_by,
            fields,
            span: self.span_from(start),
        }
    }

    /// Parses an `enum` declaration.
    fn parse_enum(&mut self, exported: bool) -> EnumDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Enum);
        let (name, name_span) = self.parse_declared_name(false);
        self.expect_punct(Punct::Colon);
        let backing = self.parse_type();
        let mut members = Vec::new();
        self.expect_punct(Punct::LBrace);
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            let member_start = self.current_start();
            let (member_name, member_name_span) = self.parse_declared_name(false);
            let value = if self.eat_punct(Punct::Eq) {
                Some(self.parse_expression())
            } else {
                None
            };
            members.push(EnumMember {
                name: member_name,
                name_span: member_name_span,
                value,
                span: self.span_from(member_start),
            });
            if !self.eat_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::RBrace);
        EnumDecl {
            exported,
            name,
            name_span,
            backing,
            members,
            span: self.span_from(start),
        }
    }

    /// Parses a `handle` declaration.
    fn parse_handle(&mut self, exported: bool) -> HandleDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Handle);
        let (name, name_span) = self.parse_declared_name(false);
        self.expect_keyword(Keyword::FreedBy);
        let freed_by = self.parse_qualified_name();
        self.expect_punct(Punct::Semi);
        HandleDecl {
            exported,
            name,
            name_span,
            freed_by,
            span: self.span_from(start),
        }
    }

    /// Parses an `ffi` block.
    fn parse_ffi_block(&mut self, exported: bool) -> FfiBlock {
        let start = self.current_start();
        self.expect_keyword(Keyword::Ffi);
        let (name, name_span) = self.parse_declared_name(false);
        let mut decls = Vec::new();
        self.expect_punct(Punct::LBrace);
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            if !self.at_keyword(Keyword::Func) {
                self.unexpected();
                self.bump();
                continue;
            }
            decls.push(self.parse_signature());
        }
        self.expect_punct(Punct::RBrace);
        FfiBlock {
            exported,
            name,
            name_span,
            decls,
            span: self.span_from(start),
        }
    }

    /// Parses a bodyless `func` signature.
    fn parse_signature(&mut self) -> FuncSignature {
        let start = self.current_start();
        self.expect_keyword(Keyword::Func);
        let (name, name_span) = self.parse_declared_name(false);
        let params = self.parse_params();
        self.expect_punct(Punct::Colon);
        let returns = self.parse_return_type();
        self.expect_punct(Punct::Semi);
        FuncSignature {
            name,
            name_span,
            params,
            returns,
            span: self.span_from(start),
        }
    }

    /// Parses an `intrinsic func` declaration, whose name may be reserved.
    fn parse_intrinsic(&mut self) -> FuncSignature {
        let start = self.current_start();
        self.expect_keyword(Keyword::Intrinsic);
        self.expect_keyword(Keyword::Func);
        let (name, name_span) = self.parse_declared_name(true);
        let params = self.parse_params();
        self.expect_punct(Punct::Colon);
        let returns = self.parse_return_type();
        self.expect_punct(Punct::Semi);
        FuncSignature {
            name,
            name_span,
            params,
            returns,
            span: self.span_from(start),
        }
    }

    /// Parses a module-level `let`.
    fn parse_global_let(&mut self, exported: bool, volatile: bool) -> GlobalLetDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Let);
        let (name, name_span) = self.parse_declared_name(false);
        self.expect_punct(Punct::Colon);
        let ty = self.parse_type();
        self.expect_punct(Punct::Eq);
        let value = self.parse_expression();
        self.expect_punct(Punct::Semi);
        GlobalLetDecl {
            exported,
            volatile,
            name,
            name_span,
            ty,
            value,
            span: self.span_from(start),
        }
    }

    /// Parses a module-level `const`.
    fn parse_global_const(&mut self, exported: bool) -> GlobalConstDecl {
        let start = self.current_start();
        self.expect_keyword(Keyword::Const);
        let (name, name_span) = self.parse_declared_name(false);
        self.expect_punct(Punct::Colon);
        let ty = self.parse_type();
        self.expect_punct(Punct::Eq);
        let value = self.parse_expression();
        self.expect_punct(Punct::Semi);
        GlobalConstDecl {
            exported,
            name,
            name_span,
            ty,
            value,
            span: self.span_from(start),
        }
    }

}
