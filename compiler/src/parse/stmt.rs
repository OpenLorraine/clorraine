//! Statement parsing.

use super::*;

impl Parser<'_> {
    /// Parses `{ statements }`.
    pub(super) fn parse_block(&mut self) -> Block {
        let start = self.current_start();
        self.expect_punct(Punct::LBrace);
        let mut statements = Vec::new();
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            match self.parse_statement() {
                Some(statement) => statements.push(statement),
                None => {
                    self.bump();
                }
            }
        }
        self.expect_punct(Punct::RBrace);
        Block {
            statements,
            span: self.span_from(start),
        }
    }

    /// Parses one statement, or reports and returns `None` when the cursor is
    /// not at a statement start.
    fn parse_statement(&mut self) -> Option<Stmt> {
        match self.peek_kind() {
            TokenKind::Keyword(Keyword::Let) => Some(Stmt::Let(self.parse_let())),
            TokenKind::Keyword(Keyword::Const) => Some(Stmt::Const(self.parse_const())),
            TokenKind::Keyword(Keyword::If) => Some(Stmt::If(self.parse_if())),
            TokenKind::Keyword(Keyword::While) => Some(Stmt::While(self.parse_while())),
            TokenKind::Keyword(Keyword::Do) => Some(Stmt::DoWhile(self.parse_do_while())),
            TokenKind::Keyword(Keyword::For) => Some(self.parse_for()),
            TokenKind::Keyword(Keyword::Switch) => Some(Stmt::Switch(self.parse_switch())),
            TokenKind::Keyword(Keyword::Return) => Some(self.parse_return()),
            TokenKind::Keyword(Keyword::Break) => {
                let token = self.bump();
                self.expect_punct(Punct::Semi);
                Some(Stmt::Break(token.span))
            }
            TokenKind::Keyword(Keyword::Continue) => {
                let token = self.bump();
                self.expect_punct(Punct::Semi);
                Some(Stmt::Continue(token.span))
            }
            TokenKind::Keyword(Keyword::Defer) => {
                self.bump();
                Some(Stmt::Defer(self.parse_block()))
            }
            TokenKind::Keyword(Keyword::Unsafe) if matches!(self.kind_at(1), TokenKind::Punct(Punct::LBrace)) => {
                self.bump();
                Some(Stmt::Unsafe(self.parse_block()))
            }
            TokenKind::Keyword(Keyword::Volatile) => {
                self.volatile_here();
                self.bump();
                self.parse_statement()
            }
            TokenKind::Punct(Punct::LBrace) => Some(Stmt::Block(self.parse_block())),
            _ => {
                let expr = self.parse_expression();
                self.expect_punct(Punct::Semi);
                Some(Stmt::Expr(expr))
            }
        }
    }

    /// Parses a `let` statement including its semicolon.
    fn parse_let(&mut self) -> LetStmt {
        let statement = self.parse_let_head();
        self.expect_punct(Punct::Semi);
        statement
    }

    /// Parses a `let` head without its semicolon.
    fn parse_let_head(&mut self) -> LetStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::Let);
        let (name, name_span) = self.parse_declared_name(false);
        let ty = if self.eat_punct(Punct::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        let value = if self.eat_punct(Punct::Eq) {
            Some(self.parse_expression())
        } else {
            None
        };
        LetStmt {
            name,
            name_span,
            ty,
            value,
            span: self.span_from(start),
        }
    }

    /// Parses a `const` statement including its semicolon.
    fn parse_const(&mut self) -> ConstStmt {
        let statement = self.parse_const_head();
        self.expect_punct(Punct::Semi);
        statement
    }

    /// Parses a `const` head without its semicolon.
    fn parse_const_head(&mut self) -> ConstStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::Const);
        let (name, name_span) = self.parse_declared_name(false);
        let ty = if self.eat_punct(Punct::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        self.expect_punct(Punct::Eq);
        let value = self.parse_expression();
        ConstStmt {
            name,
            name_span,
            ty,
            value,
            span: self.span_from(start),
        }
    }

    /// Parses an `if` statement, including an `else if` chain.
    fn parse_if(&mut self) -> IfStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::If);
        self.expect_punct(Punct::LParen);
        let condition = self.parse_expression();
        self.expect_punct(Punct::RParen);
        let then_block = self.parse_block();
        let else_branch = if self.eat_keyword(Keyword::Else) {
            if self.at_keyword(Keyword::If) {
                Some(ElseBranch::If(Box::new(self.parse_if())))
            } else {
                Some(ElseBranch::Block(self.parse_block()))
            }
        } else {
            None
        };
        IfStmt {
            condition,
            then_block,
            else_branch,
            span: self.span_from(start),
        }
    }

    /// Parses a `while` loop.
    fn parse_while(&mut self) -> WhileStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::While);
        self.expect_punct(Punct::LParen);
        let condition = self.parse_expression();
        self.expect_punct(Punct::RParen);
        let body = self.parse_block();
        WhileStmt {
            condition,
            body,
            span: self.span_from(start),
        }
    }

    /// Parses a `do ... while` loop.
    fn parse_do_while(&mut self) -> DoWhileStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::Do);
        let body = self.parse_block();
        self.expect_keyword(Keyword::While);
        self.expect_punct(Punct::LParen);
        let condition = self.parse_expression();
        self.expect_punct(Punct::RParen);
        self.expect_punct(Punct::Semi);
        DoWhileStmt {
            body,
            condition,
            span: self.span_from(start),
        }
    }

    /// Parses either `for` header form.
    fn parse_for(&mut self) -> Stmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::For);
        self.expect_punct(Punct::LParen);
        let element_form = self.at_keyword(Keyword::Borrow)
            || (matches!(self.peek_kind(), TokenKind::Ident(_))
                && matches!(self.kind_at(1), TokenKind::Keyword(Keyword::In)));
        if element_form {
            let borrow = self.eat_keyword(Keyword::Borrow);
            let (name, name_span) = self.parse_declared_name(false);
            self.expect_keyword(Keyword::In);
            let iterable = self.parse_expression();
            self.expect_punct(Punct::RParen);
            let body = self.parse_block();
            return Stmt::ForIn(ForInStmt {
                borrow,
                name,
                name_span,
                iterable,
                body,
                span: self.span_from(start),
            });
        }
        let init = if self.at_punct(Punct::Semi) {
            None
        } else if self.at_keyword(Keyword::Let) {
            Some(ForInit::Let(self.parse_let_head()))
        } else if self.at_keyword(Keyword::Const) {
            Some(ForInit::Const(self.parse_const_head()))
        } else {
            Some(ForInit::Expr(self.parse_expression_list()))
        };
        self.expect_punct(Punct::Semi);
        let condition = if self.at_punct(Punct::Semi) {
            None
        } else {
            Some(self.parse_expression())
        };
        self.expect_punct(Punct::Semi);
        let step = if self.at_punct(Punct::RParen) {
            Vec::new()
        } else {
            self.parse_expression_list()
        };
        self.expect_punct(Punct::RParen);
        let body = self.parse_block();
        Stmt::ForC(ForCStmt {
            init,
            condition,
            step,
            body,
            span: self.span_from(start),
        })
    }

    /// Parses a comma-separated expression list.
    fn parse_expression_list(&mut self) -> Vec<Expr> {
        let mut exprs = vec![self.parse_expression()];
        while self.eat_punct(Punct::Comma) {
            exprs.push(self.parse_expression());
        }
        exprs
    }

    /// Parses a `switch` statement.
    fn parse_switch(&mut self) -> SwitchStmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::Switch);
        self.expect_punct(Punct::LParen);
        let scrutinee = self.parse_expression();
        self.expect_punct(Punct::RParen);
        self.expect_punct(Punct::LBrace);
        let mut clauses = Vec::new();
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            let clause_start = self.current_start();
            let label = if self.eat_keyword(Keyword::Case) {
                SwitchLabel::Case(self.parse_expression())
            } else if self.eat_keyword(Keyword::Default) {
                SwitchLabel::Default
            } else {
                self.unexpected();
                self.bump();
                continue;
            };
            self.expect_punct(Punct::Colon);
            let mut statements = Vec::new();
            while !self.at_punct(Punct::RBrace)
                && !self.at_keyword(Keyword::Case)
                && !self.at_keyword(Keyword::Default)
                && !self.at_eof()
            {
                match self.parse_statement() {
                    Some(statement) => statements.push(statement),
                    None => {
                        self.bump();
                    }
                }
            }
            clauses.push(SwitchClause {
                label,
                statements,
                span: self.span_from(clause_start),
            });
        }
        self.expect_punct(Punct::RBrace);
        SwitchStmt {
            scrutinee,
            clauses,
            span: self.span_from(start),
        }
    }

    /// Parses a `return` statement with an optional value.
    fn parse_return(&mut self) -> Stmt {
        let start = self.current_start();
        self.expect_keyword(Keyword::Return);
        let value = if self.at_punct(Punct::Semi) {
            None
        } else {
            Some(self.parse_expression())
        };
        self.expect_punct(Punct::Semi);
        Stmt::Return {
            value,
            span: self.span_from(start),
        }
    }
}
