use crate::ast::*;
use crate::lexer::{SpannedTok, Tok};

pub struct Parser {
    toks: Vec<SpannedTok>,
    pos: usize,
}

type PResult<T> = Result<T, String>;

impl Parser {
    pub fn new(toks: Vec<SpannedTok>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn cur(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn line(&self) -> usize {
        self.toks[self.pos].line
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, want: &Tok) -> PResult<()> {
        if self.cur() == want {
            self.bump();
            Ok(())
        } else {
            Err(format!(
                "line {}: expected {:?}, found {:?}",
                self.line(),
                want,
                self.cur()
            ))
        }
    }

    fn expect_ident(&mut self) -> PResult<String> {
        match self.cur().clone() {
            Tok::Ident(s) => {
                self.bump();
                Ok(s)
            }
            other => Err(format!("line {}: expected identifier, found {:?}", self.line(), other)),
        }
    }

    pub fn parse_program(&mut self) -> PResult<Program> {
        let mut dynasties = Vec::new();
        let mut main_stmts = Vec::new();
        while *self.cur() != Tok::Eof {
            if *self.cur() == Tok::Dynasty {
                dynasties.push(self.parse_dynasty()?);
            } else {
                main_stmts.push(self.parse_stmt()?);
            }
        }
        Ok(Program { dynasties, main_stmts })
    }

    fn parse_path(&mut self) -> PResult<Path> {
        let mut segs = vec![self.expect_ident()?];
        while *self.cur() == Tok::ColonColon {
            self.bump();
            segs.push(self.expect_ident()?);
        }
        Ok(Path(segs))
    }

    fn parse_type(&mut self) -> PResult<Type> {
        let p = self.parse_path()?;
        if *self.cur() == Tok::Lt {
            self.bump();
            let inner = self.parse_type()?;
            self.expect(&Tok::Gt)?;
            Ok(Type::Generic(p, Box::new(inner)))
        } else {
            Ok(Type::Named(p))
        }
    }

    fn parse_dynasty(&mut self) -> PResult<Dynasty> {
        let line = self.line();
        self.expect(&Tok::Dynasty)?;
        let name = self.parse_path()?;
        let mut parents = Vec::new();
        if *self.cur() == Tok::Descends {
            self.bump();
            parents.push(self.parse_path()?);
            while *self.cur() == Tok::Comma {
                self.bump();
                parents.push(self.parse_path()?);
            }
        }
        let founder = if *self.cur() == Tok::Founder {
            self.bump();
            true
        } else {
            false
        };
        self.expect(&Tok::LBrace)?;
        let mut traits = Vec::new();
        let mut methods = Vec::new();
        while *self.cur() != Tok::RBrace {
            match self.cur() {
                Tok::Trait => traits.push(self.parse_trait()?),
                Tok::Override => methods.push(self.parse_method()?),
                other => return Err(format!("line {}: expected 'trait' or 'override' in dynasty body, found {:?}", self.line(), other)),
            }
        }
        self.expect(&Tok::RBrace)?;
        Ok(Dynasty { name, parents, founder, traits, methods, line })
    }

    fn parse_trait(&mut self) -> PResult<Trait> {
        let line = self.line();
        self.expect(&Tok::Trait)?;
        let name = self.expect_ident()?;
        self.expect(&Tok::Colon)?;
        let ty = self.parse_type()?;
        let default = if *self.cur() == Tok::Eq {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(Trait { name, ty, default, line })
    }

    fn parse_method(&mut self) -> PResult<Method> {
        let line = self.line();
        self.expect(&Tok::Override)?;
        let name = self.expect_ident()?;
        self.expect(&Tok::LParen)?;
        let mut params = Vec::new();
        if *self.cur() != Tok::RParen {
            params.push(self.parse_param()?);
            while *self.cur() == Tok::Comma {
                self.bump();
                params.push(self.parse_param()?);
            }
        }
        self.expect(&Tok::RParen)?;
        let ret = if *self.cur() == Tok::Arrow {
            self.bump();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&Tok::LBrace)?;
        let stmts = self.parse_stmts_until(&Tok::RBrace)?;
        self.expect(&Tok::RBrace)?;
        let body = if stmts.len() == 1 && matches!(stmts[0], Stmt::Abstract) {
            MethodBody::Abstract
        } else {
            MethodBody::Block(stmts)
        };
        Ok(Method { name, params, ret, body, line })
    }

    fn parse_param(&mut self) -> PResult<Param> {
        let name = self.expect_ident()?;
        self.expect(&Tok::Colon)?;
        let ty = self.parse_type()?;
        Ok(Param { name, ty })
    }

    fn parse_stmts_until(&mut self, end: &Tok) -> PResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while self.cur() != end {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(&Tok::LBrace)?;
        let stmts = self.parse_stmts_until(&Tok::RBrace)?;
        self.expect(&Tok::RBrace)?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.cur() {
            Tok::Let => {
                self.bump();
                let name = self.expect_ident()?;
                let ty = if *self.cur() == Tok::Colon {
                    self.bump();
                    Some(self.parse_type()?)
                } else {
                    None
                };
                let init = if *self.cur() == Tok::Eq {
                    self.bump();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                Ok(Stmt::Let(name, ty, init))
            }
            Tok::Succession => {
                self.bump();
                self.expect(&Tok::Over)?;
                let iter = self.parse_expr()?;
                self.expect(&Tok::As)?;
                let binder = if *self.cur() == Tok::Underscore {
                    self.bump();
                    None
                } else {
                    Some(self.expect_ident()?)
                };
                let body = self.parse_block()?;
                Ok(Stmt::Succession { iter, binder, body })
            }
            Tok::Claim => {
                self.bump();
                let cond = self.parse_expr()?;
                let then_body = self.parse_block()?;
                let else_body = if *self.cur() == Tok::Contested {
                    self.bump();
                    Some(self.parse_block()?)
                } else {
                    None
                };
                Ok(Stmt::Claim { cond, then_body, else_body })
            }
            Tok::Return => {
                self.bump();
                if self.stmt_terminated() {
                    Ok(Stmt::Return(None))
                } else {
                    Ok(Stmt::Return(Some(self.parse_expr()?)))
                }
            }
            Tok::Abstract => {
                self.bump();
                Ok(Stmt::Abstract)
            }
            _ => {
                let e = self.parse_expr()?;
                if *self.cur() == Tok::Eq {
                    self.bump();
                    let rhs = self.parse_expr()?;
                    Ok(Stmt::Assign(e, rhs))
                } else {
                    Ok(Stmt::ExprStmt(e))
                }
            }
        }
    }

    /// A statement is "terminated" (e.g. bare `return`) if the next token
    /// closes the current block or starts a new statement.
    fn stmt_terminated(&self) -> bool {
        matches!(self.cur(), Tok::RBrace)
    }

    // ---- expressions ----

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_and()?;
        while *self.cur() == Tok::Or {
            self.bump();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary(BinOp::Or, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_equality()?;
        while *self.cur() == Tok::And {
            self.bump();
            let rhs = self.parse_equality()?;
            lhs = Expr::Binary(BinOp::And, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_comparison()?;
        loop {
            let op = match self.cur() {
                Tok::EqEq => BinOp::Eq,
                Tok::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_comparison()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_additive()?;
        loop {
            let op = match self.cur() {
                Tok::Lt => BinOp::Lt,
                Tok::Gt => BinOp::Gt,
                Tok::LtEq => BinOp::LtEq,
                Tok::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_additive()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_additive(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.cur() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> PResult<Expr> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.cur() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_unary()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> PResult<Expr> {
        if *self.cur() == Tok::Minus {
            self.bump();
            let e = self.parse_unary()?;
            return Ok(Expr::Unary(UnOp::Neg, Box::new(e)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> PResult<Expr> {
        let mut e = self.parse_primary()?;
        loop {
            match self.cur() {
                Tok::Dot => {
                    self.bump();
                    let name = self.expect_ident()?;
                    if *self.cur() == Tok::LParen {
                        let args = self.parse_call_args()?;
                        e = Expr::MethodCall(Box::new(e), name, args);
                    } else {
                        e = Expr::Field(Box::new(e), name);
                    }
                }
                Tok::LParen => {
                    let args = self.parse_call_args()?;
                    e = Expr::Call(Box::new(e), args);
                }
                Tok::LBracket => {
                    self.bump();
                    let idx = self.parse_expr()?;
                    self.expect(&Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(idx));
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn parse_call_args(&mut self) -> PResult<Vec<Arg>> {
        self.expect(&Tok::LParen)?;
        let mut args = Vec::new();
        if *self.cur() != Tok::RParen {
            args.push(self.parse_arg()?);
            while *self.cur() == Tok::Comma {
                self.bump();
                args.push(self.parse_arg()?);
            }
        }
        self.expect(&Tok::RParen)?;
        Ok(args)
    }

    fn parse_arg(&mut self) -> PResult<Arg> {
        if let Tok::Ident(name) = self.cur().clone() {
            if self.toks[self.pos + 1].tok == Tok::Colon {
                self.bump(); // ident
                self.bump(); // colon
                let e = self.parse_expr()?;
                return Ok(Arg::Named(name, e));
            }
        }
        Ok(Arg::Positional(self.parse_expr()?))
    }

    fn parse_primary(&mut self) -> PResult<Expr> {
        match self.cur().clone() {
            Tok::IntLit(n) => {
                self.bump();
                Ok(Expr::Int(n))
            }
            Tok::StrLit(s) => {
                self.bump();
                Ok(Expr::Str(s))
            }
            Tok::True => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            Tok::False => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            Tok::SelfKw => {
                self.bump();
                Ok(Expr::SelfExpr)
            }
            Tok::LParen => {
                self.bump();
                let e = self.parse_expr()?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Tok::Pipe => {
                self.bump();
                let param = self.expect_ident()?;
                self.expect(&Tok::Pipe)?;
                let body = self.parse_expr()?;
                Ok(Expr::Lambda(param, Box::new(body)))
            }
            Tok::Birth => {
                self.bump();
                self.expect(&Tok::LParen)?;
                let class = self.parse_path()?;
                let mut args = Vec::new();
                while *self.cur() == Tok::Comma {
                    self.bump();
                    args.push(self.parse_arg()?);
                }
                self.expect(&Tok::RParen)?;
                Ok(Expr::Birth(class, args))
            }
            Tok::Ident(_) => {
                let p = self.parse_path()?;
                if p.0.len() == 1 {
                    Ok(Expr::Ident(p.0.into_iter().next().unwrap()))
                } else {
                    Ok(Expr::PathExpr(p))
                }
            }
            other => Err(format!("line {}: unexpected token {:?} in expression", self.line(), other)),
        }
    }
}

pub fn parse(src: &str) -> PResult<Program> {
    let toks = crate::lexer::Lexer::new(src).tokenize()?;
    Parser::new(toks).parse_program()
}
