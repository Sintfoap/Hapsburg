#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub Vec<String>);

impl Path {
    pub fn joined(&self) -> String {
        self.0.join("::")
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.joined())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Named(Path),
    Generic(Path, Box<Type>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Named(p) => write!(f, "{}", p),
            Type::Generic(p, t) => write!(f, "{}<{}>", p, t),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Trait {
    pub name: String,
    pub ty: Type,
    pub default: Option<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum MethodBody {
    Abstract,
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<Type>,
    pub body: MethodBody,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Dynasty {
    pub name: Path,
    pub parents: Vec<Path>,
    pub founder: bool,
    pub traits: Vec<Trait>,
    pub methods: Vec<Method>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub dynasties: Vec<Dynasty>,
    pub main_stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Arg {
    Positional(Expr),
    Named(String, Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Str(String),
    Bool(bool),
    SelfExpr,
    Ident(String),
    /// A `::`-separated path used as a value, e.g. `Integer::parse`, `Habsburg::Accumulator`.
    PathExpr(Path),
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Arg>),
    MethodCall(Box<Expr>, String, Vec<Arg>),
    Birth(Path, Vec<Arg>),
    Lambda(String, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Option<Type>, Option<Expr>, usize),
    Assign(Expr, Expr),
    ExprStmt(Expr),
    Succession { iter: Expr, binder: Option<String>, body: Vec<Stmt> },
    Claim { cond: Expr, then_body: Vec<Stmt>, else_body: Option<Vec<Stmt>> },
    Return(Option<Expr>),
    Abstract,
}
