use crate::ast::*;
use crate::resolve::{Resolved, Resolver};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HType {
    Int,
    Str,
    Bool,
    Void,
    ListInt,
    ListStr,
    ListListInt,
    Accumulator,
    Class(String),
}

impl HType {
    fn c_type(&self) -> String {
        match self {
            HType::Int => "int64_t".into(),
            HType::Str => "char*".into(),
            HType::Bool => "int".into(),
            HType::Void => "void".into(),
            HType::ListInt => "HbListInt".into(),
            HType::ListStr => "HbListStr".into(),
            HType::ListListInt => "HbListListInt".into(),
            HType::Accumulator => "HbAccumulator*".into(),
            HType::Class(name) => format!("Hb_{}*", mangle(name)),
        }
    }

    /// How this type reads in Hapsburg source, not C — for hover text and
    /// diagnostics aimed at someone editing a `.hb` file.
    pub fn hapsburg_display(&self) -> String {
        match self {
            HType::Int => "Integer".into(),
            HType::Str => "String".into(),
            HType::Bool => "Bool".into(),
            HType::Void => "Void".into(),
            HType::ListInt => "List<Integer>".into(),
            HType::ListStr => "List<String>".into(),
            HType::ListListInt => "List<List<Integer>>".into(),
            HType::Accumulator => "Habsburg::Accumulator".into(),
            HType::Class(name) => name.clone(),
        }
    }
}

fn mangle(joined_path: &str) -> String {
    joined_path.replace("::", "__")
}

fn is_path(e: &Expr, segs: &[&str]) -> bool {
    match e {
        Expr::PathExpr(p) => p.0.len() == segs.len() && p.0.iter().zip(segs).all(|(a, b)| a == b),
        Expr::Ident(s) if segs.len() == 1 => s == segs[0],
        _ => false,
    }
}

/// `x.marry(Type)` (or the bare `marry(Type)` form recognized specially
/// inside `.map(...)`) — Hapsburg's type-cast operator. Marrying into your
/// own house is a no-op (`None` = identity, use the operand unchanged);
/// marrying into an unrelated one calls the named runtime conversion
/// function; anything not on this list has "no legitimate marriage".
fn marry_conversion(from: &HType, target_name: &str) -> CResult<(HType, Option<&'static str>)> {
    let target = match target_name {
        "Integer" => HType::Int,
        "String" => HType::Str,
        "Bool" => HType::Bool,
        other => {
            return Err(format!(
                "error: '{}' is not a lineage anything can marry into (only Integer, String, Bool)",
                other
            ))
        }
    };
    if *from == target {
        return Ok((target, None));
    }
    match (from, &target) {
        (HType::Str, HType::Int) => Ok((HType::Int, Some("hb_integer_parse"))),
        (HType::Int, HType::Str) => Ok((HType::Str, Some("hb_integer_to_string"))),
        (HType::Bool, HType::Str) => Ok((HType::Str, Some("hb_bool_to_string"))),
        (from, _) => Err(format!(
            "error: no legitimate marriage between {} and {}",
            from.hapsburg_display(),
            target.hapsburg_display()
        )),
    }
}

/// Pulls the bare type name out of `marry(TypeName)`'s single argument —
/// the same restricted "just a name" position `type_of_annotation` handles
/// for real type syntax, but marry's target is an ordinary expression
/// (parsed via the normal call-argument grammar), so it needs its own
/// extraction here rather than going through `Type`.
fn marry_target_name(e: &Expr) -> Option<&str> {
    match e {
        Expr::Ident(s) => Some(s),
        Expr::PathExpr(p) if p.0.len() == 1 => Some(&p.0[0]),
        _ => None,
    }
}

fn c_string_literal(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

struct FnCtx {
    leaf: Option<String>,
    scopes: Vec<HashMap<String, HType>>,
    tmp: usize,
}

impl FnCtx {
    fn new(leaf: Option<String>) -> Self {
        FnCtx {
            leaf,
            scopes: vec![HashMap::new()],
            tmp: 0,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn declare(&mut self, name: &str, ty: HType) {
        self.scopes.last_mut().unwrap().insert(name.to_string(), ty);
    }
    fn lookup(&self, name: &str) -> Option<&HType> {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.get(name) {
                return Some(t);
            }
        }
        None
    }
    fn fresh_tmp(&mut self) -> String {
        self.tmp += 1;
        format!("__t{}", self.tmp)
    }
}

pub struct Codegen<'a> {
    resolver: &'a Resolver<'a>,
    pub structs: String,
    pub func_decls: String,
    pub func_impls: String,
    lambda_ctr: usize,
    /// (source line, variable name, inferred type) for every `heir` and
    /// method parameter seen while generating code — recorded purely as a
    /// side channel for tooling (the LSP's hover), unused by codegen
    /// itself. Populated even for classes whose codegen later fails,
    /// since the failure only affects statements *after* the error.
    pub var_hints: Vec<(usize, String, HType)>,
}

type CResult<T> = Result<T, String>;

impl<'a> Codegen<'a> {
    pub fn new(resolver: &'a Resolver<'a>) -> Self {
        Codegen {
            resolver,
            structs: String::new(),
            func_decls: String::new(),
            func_impls: String::new(),
            lambda_ctr: 0,
            var_hints: Vec::new(),
        }
    }

    fn type_of_annotation(&self, ty: &Type) -> CResult<HType> {
        match ty {
            Type::Named(p) if p.0.len() == 1 => match p.0[0].as_str() {
                "Integer" => Ok(HType::Int),
                "String" => Ok(HType::Str),
                "Bool" => Ok(HType::Bool),
                "Void" => Ok(HType::Void),
                other => {
                    if self.resolver.classes.contains_key(other) {
                        Ok(HType::Class(other.to_string()))
                    } else {
                        Err(format!("error: unknown type '{}'", other))
                    }
                }
            },
            Type::Named(p) => {
                let joined = p.joined();
                if self.resolver.classes.contains_key(&joined) {
                    Ok(HType::Class(joined))
                } else {
                    Err(format!("error: unknown type '{}'", joined))
                }
            }
            Type::Generic(p, inner) if p.0.len() == 1 && p.0[0] == "List" => {
                match self.type_of_annotation(inner)? {
                    HType::Int => Ok(HType::ListInt),
                    HType::Str => Ok(HType::ListStr),
                    HType::ListInt => Ok(HType::ListListInt),
                    other => Err(format!("error: List<{:?}> is not supported (only List<Integer>, List<String>, List<List<Integer>>)", other)),
                }
            }
            Type::Generic(p, _) => Err(format!("error: unknown generic type '{}'", p.joined())),
        }
    }

    /// Discover every concrete class the program actually `birth()`s, by
    /// scanning all method bodies (of all classes) and the top-level script.
    /// Over-approximates (dead code included) but that's fine for a toy AOT
    /// compiler with no separate compilation.
    pub fn discover_birthed(&self, program: &Program) -> Vec<String> {
        let mut found = Vec::new();
        fn walk_expr(e: &Expr, found: &mut Vec<String>) {
            match e {
                Expr::Birth(path, args) => {
                    let j = path.joined();
                    if j != "Habsburg::Accumulator" && !found.contains(&j) {
                        found.push(j);
                    }
                    for a in args {
                        walk_expr(arg_expr(a), found);
                    }
                }
                Expr::Field(b, _) => walk_expr(b, found),
                Expr::Index(b, i) => {
                    walk_expr(b, found);
                    walk_expr(i, found);
                }
                Expr::Call(c, args) => {
                    walk_expr(c, found);
                    for a in args {
                        walk_expr(arg_expr(a), found);
                    }
                }
                Expr::MethodCall(b, _, args) => {
                    walk_expr(b, found);
                    for a in args {
                        walk_expr(arg_expr(a), found);
                    }
                }
                Expr::Lambda(_, body) => walk_expr(body, found),
                Expr::Unary(_, e) => walk_expr(e, found),
                Expr::Binary(_, l, r) => {
                    walk_expr(l, found);
                    walk_expr(r, found);
                }
                _ => {}
            }
        }
        fn arg_expr(a: &Arg) -> &Expr {
            match a {
                Arg::Positional(e) => e,
                Arg::Named(_, e) => e,
            }
        }
        fn walk_stmt(s: &Stmt, found: &mut Vec<String>) {
            match s {
                Stmt::Heir(_, _, Some(e), _) => walk_expr(e, found),
                Stmt::Heir(_, _, None, _) => {}
                Stmt::Assign(l, r) => {
                    walk_expr(l, found);
                    walk_expr(r, found);
                }
                Stmt::ExprStmt(e) => walk_expr(e, found),
                Stmt::Succession { iter, body, .. } => {
                    walk_expr(iter, found);
                    for s in body {
                        walk_stmt(s, found);
                    }
                }
                Stmt::Claim {
                    cond,
                    then_body,
                    else_body,
                } => {
                    walk_expr(cond, found);
                    for s in then_body {
                        walk_stmt(s, found);
                    }
                    if let Some(eb) = else_body {
                        for s in eb {
                            walk_stmt(s, found);
                        }
                    }
                }
                Stmt::Return(Some(e)) => walk_expr(e, found),
                Stmt::Return(None) | Stmt::Abstract => {}
            }
        }
        for d in &program.dynasties {
            for m in &d.methods {
                if let MethodBody::Block(stmts) = &m.body {
                    for s in stmts {
                        walk_stmt(s, &mut found);
                    }
                }
            }
        }
        for s in &program.main_stmts {
            walk_stmt(s, &mut found);
        }
        found
    }

    pub fn gen_class(&mut self, class: &str) -> CResult<()> {
        let resolved = self.resolver.resolve(class).map_err(|e| e.0)?;
        let mangled = mangle(class);

        // struct
        self.structs
            .push_str(&format!("typedef struct Hb_{m} Hb_{m};\n", m = mangled));
        let mut body = String::new();
        for (name, t) in &resolved.traits {
            let ty = self.type_of_annotation(&t.ty)?;
            body.push_str(&format!("    {} {};\n", ty.c_type(), name));
        }
        self.structs
            .push_str(&format!("struct Hb_{} {{\n{}}};\n\n", mangled, body));

        // methods
        let method_names: Vec<String> = resolved.methods.keys().cloned().collect();
        for mname in &method_names {
            let (_, m) = resolved.methods.get(mname).unwrap();
            match &m.body {
                MethodBody::Abstract => {
                    return Err(format!(
                        "error: line {}: cannot birth '{}': '{}' has no heir to inherit it — line of succession broken (still abstract)",
                        m.line, class, mname
                    ));
                }
                MethodBody::Block(_) => {}
            }
        }

        for mname in &method_names {
            self.gen_method(class, &resolved, mname)?;
        }
        Ok(())
    }

    fn gen_method(&mut self, class: &str, resolved: &Resolved<'a>, mname: &str) -> CResult<()> {
        let (_, m) = resolved.methods.get(mname).unwrap();
        let stmts = match &m.body {
            MethodBody::Block(s) => s.clone(),
            MethodBody::Abstract => unreachable!(),
        };
        let ret_ty = match &m.ret {
            Some(t) => self.type_of_annotation(t)?,
            None => HType::Void,
        };
        let mangled = mangle(class);
        let mut params_c = format!("Hb_{}* self", mangled);
        let mut fctx = FnCtx::new(Some(class.to_string()));
        for p in &m.params {
            let pty = self.type_of_annotation(&p.ty)?;
            params_c.push_str(&format!(", {} {}", pty.c_type(), p.name));
            self.var_hints.push((m.line, p.name.clone(), pty.clone()));
            fctx.declare(&p.name, pty);
        }

        let mut out = String::new();
        self.gen_block(&mut fctx, &mut out, &stmts, &ret_ty)?;

        // Defensive trailing return so every non-void path is well-formed C,
        // even if our own control-flow analysis is incomplete (e.g. a claim
        // that doesn't cover every branch, or a call to hb_assassinate the
        // compiler doesn't prove is noreturn).
        if ret_ty != HType::Void {
            out.push_str(&format!("    return ({}){{0}};\n", ret_ty.c_type()));
        }

        let sig = format!(
            "{} hb__{}__{}({})",
            ret_ty.c_type(),
            mangled,
            mname,
            params_c
        );
        self.func_decls.push_str(&format!("{};\n", sig));
        self.func_impls
            .push_str(&format!("{} {{\n{}}}\n\n", sig, out));
        Ok(())
    }

    fn gen_block(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        stmts: &[Stmt],
        ret_ty: &HType,
    ) -> CResult<()> {
        for s in stmts {
            self.gen_stmt(fctx, out, s, ret_ty)?;
        }
        Ok(())
    }

    fn gen_stmt(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        s: &Stmt,
        ret_ty: &HType,
    ) -> CResult<()> {
        match s {
            Stmt::Heir(name, ann, init, line) => {
                let init = init
                    .as_ref()
                    .ok_or_else(|| format!("error: 'let {}' needs an initializer", name))?;
                let (ity, icode) = self.gen_expr(fctx, out, init)?;
                let declty = match ann {
                    Some(t) => self.type_of_annotation(t)?,
                    None => ity,
                };
                out.push_str(&format!("    {} {} = {};\n", declty.c_type(), name, icode));
                self.var_hints.push((*line, name.clone(), declty.clone()));
                fctx.declare(name, declty);
                Ok(())
            }
            Stmt::Assign(lhs, rhs) => {
                let (_, rcode) = self.gen_expr(fctx, out, rhs)?;
                match lhs {
                    Expr::Ident(name) => {
                        if fctx.lookup(name).is_none() {
                            return Err(format!("error: assignment to unknown name '{}'", name));
                        }
                        out.push_str(&format!("    {} = {};\n", name, rcode));
                    }
                    Expr::Field(obj, fname) => {
                        let (_, ocode) = self.gen_expr(fctx, out, obj)?;
                        out.push_str(&format!("    {}->{} = {};\n", ocode, fname, rcode));
                    }
                    _ => return Err("error: unsupported assignment target".to_string()),
                }
                Ok(())
            }
            Stmt::ExprStmt(e) => {
                let (_, code) = self.gen_expr(fctx, out, e)?;
                out.push_str(&format!("    {};\n", code));
                Ok(())
            }
            Stmt::Succession { iter, binder, body } => {
                self.gen_succession(fctx, out, iter, binder, body, ret_ty)
            }
            Stmt::Claim {
                cond,
                then_body,
                else_body,
            } => {
                let (cty, ccode) = self.gen_expr(fctx, out, cond)?;
                if cty != HType::Bool {
                    return Err(format!(
                        "error: 'claim' condition must be Bool, found {:?}",
                        cty
                    ));
                }
                out.push_str(&format!("    if ({}) {{\n", ccode));
                fctx.push_scope();
                self.gen_block(fctx, out, then_body, ret_ty)?;
                fctx.pop_scope();
                out.push_str("    }\n");
                if let Some(eb) = else_body {
                    out.push_str("    else {\n");
                    fctx.push_scope();
                    self.gen_block(fctx, out, eb, ret_ty)?;
                    fctx.pop_scope();
                    out.push_str("    }\n");
                }
                Ok(())
            }
            Stmt::Return(e) => {
                match e {
                    Some(e) => {
                        let (_, code) = self.gen_expr(fctx, out, e)?;
                        out.push_str(&format!("    return {};\n", code));
                    }
                    None => out.push_str("    return;\n"),
                }
                Ok(())
            }
            Stmt::Abstract => {
                Err("error: 'abstract' may only appear as a method's entire body".to_string())
            }
        }
    }

    fn gen_succession(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        iter: &Expr,
        binder: &Option<String>,
        body: &[Stmt],
        ret_ty: &HType,
    ) -> CResult<()> {
        if let Expr::Call(callee, _) = iter {
            if is_path(callee, &["Habsburg", "Range", "infinite"]) {
                let v = binder.clone().unwrap_or_else(|| fctx.fresh_tmp());
                out.push_str(&format!("    for (int64_t {v} = 0;; {v}++) {{\n", v = v));
                fctx.push_scope();
                fctx.declare(&v, HType::Int);
                self.gen_block(fctx, out, body, ret_ty)?;
                fctx.pop_scope();
                out.push_str("    }\n");
                return Ok(());
            }
        }
        if let Expr::Call(callee, args) = iter {
            if is_path(callee, &["Habsburg", "Range", "up_to"]) {
                if args.len() != 1 {
                    return Err(
                        "error: Habsburg::Range::up_to expects exactly one argument".to_string()
                    );
                }
                let (nty, ncode) = self.gen_expr(fctx, out, arg_val(&args[0]))?;
                if nty != HType::Int {
                    return Err("error: Habsburg::Range::up_to expects an Integer".to_string());
                }
                let v = binder.clone().unwrap_or_else(|| fctx.fresh_tmp());
                let bound = fctx.fresh_tmp();
                out.push_str(&format!("    int64_t {b} = {n};\n", b = bound, n = ncode));
                out.push_str(&format!(
                    "    for (int64_t {v} = 0; {v} < {b}; {v}++) {{\n",
                    v = v,
                    b = bound
                ));
                fctx.push_scope();
                fctx.declare(&v, HType::Int);
                self.gen_block(fctx, out, body, ret_ty)?;
                fctx.pop_scope();
                out.push_str("    }\n");
                return Ok(());
            }
        }
        if let Expr::Field(inner, fname) = iter {
            if fname == "indices" {
                let (ity, icode) = self.gen_expr(fctx, out, inner)?;
                if !matches!(ity, HType::ListInt | HType::ListStr | HType::ListListInt) {
                    return Err("error: '.indices' is only defined on lists".to_string());
                }
                let listvar = self.materialize(fctx, out, &ity, &icode);
                let v = binder.clone().unwrap_or_else(|| fctx.fresh_tmp());
                out.push_str(&format!(
                    "    for (int64_t {v} = 0; {v} < {l}.len; {v}++) {{\n",
                    v = v,
                    l = listvar
                ));
                fctx.push_scope();
                fctx.declare(&v, HType::Int);
                self.gen_block(fctx, out, body, ret_ty)?;
                fctx.pop_scope();
                out.push_str("    }\n");
                return Ok(());
            }
        }

        // generic: foreach over a list-valued expression
        let (ity, icode) = self.gen_expr(fctx, out, iter)?;
        let (elem_ty, accessor) = match ity {
            HType::ListInt => (HType::Int, "data"),
            HType::ListStr => (HType::Str, "data"),
            HType::ListListInt => (HType::ListInt, "data"),
            other => {
                return Err(format!(
                    "error: 'succession over' expects a list or a Habsburg::Range, found {:?}",
                    other
                ))
            }
        };
        let listvar = self.materialize(fctx, out, &ity, &icode);
        let idx = fctx.fresh_tmp();
        out.push_str(&format!(
            "    for (int64_t {i} = 0; {i} < {l}.len; {i}++) {{\n",
            i = idx,
            l = listvar
        ));
        fctx.push_scope();
        if let Some(v) = binder {
            out.push_str(&format!(
                "        {} {} = {}.{}[{}];\n",
                elem_ty.c_type(),
                v,
                listvar,
                accessor,
                idx
            ));
            fctx.declare(v, elem_ty);
        }
        self.gen_block(fctx, out, body, ret_ty)?;
        fctx.pop_scope();
        out.push_str("    }\n");
        Ok(())
    }

    fn materialize(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        ty: &HType,
        code: &str,
    ) -> String {
        if is_simple_ident(code) {
            return code.to_string();
        }
        let v = fctx.fresh_tmp();
        out.push_str(&format!("    {} {} = {};\n", ty.c_type(), v, code));
        v
    }

    fn gen_expr(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        e: &Expr,
    ) -> CResult<(HType, String)> {
        match e {
            Expr::Int(n) => Ok((HType::Int, n.to_string())),
            Expr::Str(s) => Ok((HType::Str, c_string_literal(s))),
            Expr::Bool(b) => Ok((HType::Bool, if *b { "1".into() } else { "0".into() })),
            Expr::SelfExpr => {
                let leaf = fctx
                    .leaf
                    .clone()
                    .ok_or_else(|| "error: 'self' used outside a dynasty method".to_string())?;
                Ok((HType::Class(leaf), "self".to_string()))
            }
            Expr::Ident(name) => {
                let ty = fctx
                    .lookup(name)
                    .cloned()
                    .ok_or_else(|| format!("error: unknown name '{}'", name))?;
                Ok((ty, name.clone()))
            }
            Expr::PathExpr(p) => Err(format!(
                "error: '{}' cannot be used as a plain value here",
                p.joined()
            )),
            Expr::Field(obj, name) => self.gen_field(fctx, out, obj, name),
            Expr::Index(obj, idx) => self.gen_index(fctx, out, obj, idx),
            Expr::Call(callee, args) => self.gen_call(fctx, out, callee, args),
            Expr::MethodCall(obj, name, args) => self.gen_method_call(fctx, out, obj, name, args),
            Expr::Birth(path, args) => self.gen_birth(fctx, out, path, args),
            Expr::Lambda(..) => {
                Err("error: a lambda is only supported as the argument to .map(...)".to_string())
            }
            Expr::Unary(UnOp::Neg, inner) => {
                let (ty, code) = self.gen_expr(fctx, out, inner)?;
                if ty != HType::Int {
                    return Err("error: unary '-' expects an Integer".to_string());
                }
                Ok((HType::Int, format!("(-{})", code)))
            }
            Expr::Binary(op, l, r) => self.gen_binary(fctx, out, *op, l, r),
        }
    }

    fn gen_field(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        obj: &Expr,
        name: &str,
    ) -> CResult<(HType, String)> {
        let (oty, ocode) = self.gen_expr(fctx, out, obj)?;
        match &oty {
            HType::Class(cls) => {
                let resolved = self.resolver.resolve(cls).map_err(|e| e.0)?;
                let t = resolved
                    .traits
                    .iter()
                    .find(|(n, _)| n == name)
                    .ok_or_else(|| format!("error: '{}' has no trait '{}'", cls, name))?;
                let ty = self.type_of_annotation(&t.1.ty)?;
                Ok((ty, format!("{}->{}", ocode, name)))
            }
            HType::Accumulator if name == "value" => Ok((HType::Int, format!("{}->value", ocode))),
            HType::Str if name == "length" => {
                Ok((HType::Int, format!("hb_string_length({})", ocode)))
            }
            HType::ListInt | HType::ListStr | HType::ListListInt if name == "length" => {
                Ok((HType::Int, format!("({}).len", ocode)))
            }
            HType::ListInt | HType::ListStr | HType::ListListInt if name == "indices" => Err(
                "error: '.indices' is only valid directly as a 'succession over ... as' source"
                    .to_string(),
            ),
            other => Err(format!("error: no field '{}' on type {:?}", name, other)),
        }
    }

    fn gen_index(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        obj: &Expr,
        idx: &Expr,
    ) -> CResult<(HType, String)> {
        let (oty, ocode) = self.gen_expr(fctx, out, obj)?;
        let (ity, icode) = self.gen_expr(fctx, out, idx)?;
        if ity != HType::Int {
            return Err("error: list index must be an Integer".to_string());
        }
        match oty {
            HType::ListInt => {
                let v = self.materialize(fctx, out, &HType::ListInt, &ocode);
                Ok((HType::Int, format!("hb_list_int_get(&{}, {})", v, icode)))
            }
            HType::ListStr => {
                let v = self.materialize(fctx, out, &HType::ListStr, &ocode);
                Ok((HType::Str, format!("hb_list_str_get(&{}, {})", v, icode)))
            }
            HType::ListListInt => {
                let v = self.materialize(fctx, out, &HType::ListListInt, &ocode);
                Ok((
                    HType::ListInt,
                    format!("hb_list_list_int_get(&{}, {})", v, icode),
                ))
            }
            other => Err(format!("error: cannot index into type {:?}", other)),
        }
    }

    fn gen_call(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        callee: &Expr,
        args: &[Arg],
    ) -> CResult<(HType, String)> {
        if is_path(callee, &["Habsburg", "Range", "infinite"])
            || is_path(callee, &["Habsburg", "Range", "up_to"])
        {
            return Err(
                "error: Habsburg::Range::* is only valid directly as a 'succession over' source"
                    .to_string(),
            );
        }
        if is_path(callee, &["Habsburg", "Correspondence", "receive_line"]) {
            if !args.is_empty() {
                return Err(
                    "error: Habsburg::Correspondence::receive_line expects no arguments"
                        .to_string(),
                );
            }
            return Ok((HType::Str, "hb_correspondence_receive_line()".to_string()));
        }
        if is_path(callee, &["Habsburg", "Correspondence", "receive_all"]) {
            if !args.is_empty() {
                return Err(
                    "error: Habsburg::Correspondence::receive_all expects no arguments".to_string(),
                );
            }
            return Ok((HType::Str, "hb_correspondence_receive_all()".to_string()));
        }
        if is_path(callee, &["print"]) {
            if args.len() != 1 {
                return Err("error: print expects exactly one argument".to_string());
            }
            let (aty, acode) = self.gen_expr(fctx, out, arg_val(&args[0]))?;
            let call = match aty {
                HType::Int => format!("hb_print_int({})", acode),
                HType::Str => format!("hb_print_str({})", acode),
                other => {
                    return Err(format!(
                        "error: print doesn't know how to display {:?}",
                        other
                    ))
                }
            };
            return Ok((HType::Void, call));
        }
        if is_path(callee, &["abs"]) {
            if args.len() != 1 {
                return Err("error: abs expects exactly one argument".to_string());
            }
            let (aty, acode) = self.gen_expr(fctx, out, arg_val(&args[0]))?;
            if aty != HType::Int {
                return Err("error: abs expects an Integer".to_string());
            }
            return Ok((HType::Int, format!("hb_abs({})", acode)));
        }
        if is_path(callee, &["assassinate"]) {
            let exc_name = match args.first() {
                Some(Arg::Positional(Expr::Ident(n))) => n.clone(),
                Some(Arg::Positional(Expr::PathExpr(p))) => p.joined(),
                _ => "InbreedingError".to_string(),
            };
            let reason = args
                .iter()
                .find_map(|a| match a {
                    Arg::Named(n, e) if n == "reason" => Some(e),
                    _ => None,
                })
                .ok_or_else(|| "error: assassinate(...) needs a 'reason:' argument".to_string())?;
            let (rty, rcode) = self.gen_expr(fctx, out, reason)?;
            if rty != HType::Str {
                return Err("error: assassinate's 'reason:' must be a String".to_string());
            }
            return Ok((
                HType::Void,
                format!("hb_assassinate({}, {})", c_string_literal(&exc_name), rcode),
            ));
        }
        Err(format!(
            "error: Hapsburg has no free functions beyond the Habsburg:: intrinsics — unknown call '{:?}'",
            callee
        ))
    }

    fn gen_method_call(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        obj: &Expr,
        name: &str,
        args: &[Arg],
    ) -> CResult<(HType, String)> {
        let is_self = matches!(obj, Expr::SelfExpr);
        let (oty, ocode) = self.gen_expr(fctx, out, obj)?;

        if name == "marry" {
            if args.len() != 1 {
                return Err(
                    "error: marry(...) expects exactly one argument: the type to marry into"
                        .to_string(),
                );
            }
            let target_name = marry_target_name(arg_val(&args[0])).ok_or_else(|| {
                "error: marry(...) expects a bare type name, e.g. .marry(Integer)".to_string()
            })?;
            let (rty, conv) = marry_conversion(&oty, target_name)?;
            let code = match conv {
                Some(f) => format!("{}({})", f, ocode),
                None => ocode, // already of that lineage -- no-op
            };
            return Ok((rty, code));
        }

        match &oty {
            HType::Class(cls) => {
                let target_cls = if is_self {
                    fctx.leaf.clone().unwrap()
                } else {
                    cls.clone()
                };
                let resolved = self.resolver.resolve(&target_cls).map_err(|e| e.0)?;
                let (_, m) = resolved.methods.get(name).ok_or_else(|| {
                    format!(
                        "error: '{}' has no method '{}' in its lineage",
                        target_cls, name
                    )
                })?;
                let ret_ty = match &m.ret {
                    Some(t) => self.type_of_annotation(t)?,
                    None => HType::Void,
                };
                let mut argcodes = vec![ocode];
                for a in args {
                    let (_, c) = self.gen_expr(fctx, out, arg_val(a))?;
                    argcodes.push(c);
                }
                Ok((
                    ret_ty,
                    format!(
                        "hb__{}__{}({})",
                        mangle(&target_cls),
                        name,
                        argcodes.join(", ")
                    ),
                ))
            }
            HType::Accumulator => {
                if name != "absorb" {
                    return Err(format!(
                        "error: Habsburg::Accumulator has no method '{}'",
                        name
                    ));
                }
                if args.len() != 1 {
                    return Err("error: absorb expects exactly one argument".to_string());
                }
                let (aty, acode) = self.gen_expr(fctx, out, arg_val(&args[0]))?;
                if aty != HType::Int {
                    return Err("error: absorb expects an Integer".to_string());
                }
                Ok((
                    HType::Void,
                    format!("hb_accumulator_absorb({}, {})", ocode, acode),
                ))
            }
            HType::Str => match name {
                "chars" => Ok((HType::ListStr, format!("hb_string_chars({})", ocode))),
                "lines" => Ok((HType::ListStr, format!("hb_string_lines({})", ocode))),
                "split_whitespace" => Ok((
                    HType::ListStr,
                    format!("hb_string_split_whitespace({})", ocode),
                )),
                other => Err(format!("error: String has no method '{}'", other)),
            },
            HType::ListInt => match name {
                "max" => {
                    let v = self.materialize(fctx, out, &HType::ListInt, &ocode);
                    Ok((HType::Int, format!("hb_list_int_max(&{})", v)))
                }
                "min" => {
                    let v = self.materialize(fctx, out, &HType::ListInt, &ocode);
                    Ok((HType::Int, format!("hb_list_int_min(&{})", v)))
                }
                "map" => self.gen_map(fctx, out, HType::ListInt, ocode, arg_val(&args[0])),
                other => Err(format!("error: List<Integer> has no method '{}'", other)),
            },
            HType::ListStr => match name {
                "map" => self.gen_map(fctx, out, HType::ListStr, ocode, arg_val(&args[0])),
                other => Err(format!("error: List<String> has no method '{}'", other)),
            },
            other => Err(format!("error: no method '{}' on type {:?}", name, other)),
        }
    }

    fn gen_map(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        input_ty: HType,
        input_code: String,
        func_arg: &Expr,
    ) -> CResult<(HType, String)> {
        let elem_ty = match input_ty {
            HType::ListStr => HType::Str,
            HType::ListInt => HType::Int,
            _ => {
                return Err(
                    "error: map() is only supported on List<String> or List<Integer>".to_string(),
                )
            }
        };
        let listvar = self.materialize(fctx, out, &input_ty, &input_code);

        // (result element type, per-element C expression producing it from `elem`)
        let (result_elem_ty, call_fn): (HType, String) = if let Expr::Call(callee, cargs) = func_arg
        {
            if is_path(callee, &["marry"]) && cargs.len() == 1 {
                let target_name = marry_target_name(arg_val(&cargs[0]))
                    .ok_or_else(|| "error: marry(...) inside map() expects a bare type name, e.g. marry(Integer)".to_string())?;
                let (rty, conv) = marry_conversion(&elem_ty, target_name)?;
                let f = conv.ok_or_else(|| {
                    format!(
                        "error: marry({}) inside map() is a no-op here — {} is already legitimate, drop the .map(...)",
                        target_name, elem_ty.hapsburg_display()
                    )
                })?;
                (rty, f.to_string())
            } else {
                return Err("error: map() expects marry(Type) or a |x| lambda".to_string());
            }
        } else if let Expr::Lambda(param, body) = func_arg {
            let mut lctx = FnCtx::new(fctx.leaf.clone());
            lctx.declare(param, elem_ty.clone());
            let mut lbody_out = String::new();
            let (bty, bcode) = self.gen_expr(&mut lctx, &mut lbody_out, body)?;
            self.lambda_ctr += 1;
            let fname = format!("hb_lambda_{}", self.lambda_ctr);
            let sig = format!(
                "static {} {}({} {})",
                bty.c_type(),
                fname,
                elem_ty.c_type(),
                param
            );
            self.func_decls.push_str(&format!("{};\n", sig));
            self.func_impls.push_str(&format!(
                "{} {{\n{}    return {};\n}}\n\n",
                sig, lbody_out, bcode
            ));
            (bty, fname)
        } else {
            return Err("error: map() expects marry(Type) or a |x| lambda".to_string());
        };

        let result_list_ty = match result_elem_ty {
            HType::Int => HType::ListInt,
            HType::ListInt => HType::ListListInt,
            HType::Str => HType::ListStr,
            other => {
                return Err(format!(
                    "error: map() producing {:?} elements is not supported",
                    other
                ))
            }
        };
        let push_fn = match result_list_ty {
            HType::ListInt => "hb_list_int_push",
            HType::ListListInt => "hb_list_list_int_push",
            HType::ListStr => "hb_list_str_push",
            _ => unreachable!(),
        };
        let new_fn = match result_list_ty {
            HType::ListInt => "hb_list_int_new",
            HType::ListListInt => "hb_list_list_int_new",
            HType::ListStr => "hb_list_str_new",
            _ => unreachable!(),
        };

        let outvar = fctx.fresh_tmp();
        let idx = fctx.fresh_tmp();
        out.push_str(&format!(
            "    {} {} = {}();\n",
            result_list_ty.c_type(),
            outvar,
            new_fn
        ));
        out.push_str(&format!(
            "    for (int64_t {i} = 0; {i} < {l}.len; {i}++) {{\n        {push}(&{o}, {f}({l}.data[{i}]));\n    }}\n",
            i = idx, l = listvar, push = push_fn, o = outvar, f = call_fn
        ));
        Ok((result_list_ty, outvar))
    }

    fn gen_birth(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        path: &Path,
        args: &[Arg],
    ) -> CResult<(HType, String)> {
        if path.joined() == "Habsburg::Accumulator" {
            let seed = args
                .iter()
                .find_map(|a| match a {
                    Arg::Named(n, e) if n == "seed" => Some(e),
                    _ => None,
                })
                .ok_or_else(|| {
                    "error: birth(Habsburg::Accumulator, ...) needs a 'seed:' argument".to_string()
                })?;
            let (sty, scode) = self.gen_expr(fctx, out, seed)?;
            if sty != HType::Int {
                return Err("error: Habsburg::Accumulator's seed must be an Integer".to_string());
            }
            let v = fctx.fresh_tmp();
            out.push_str(&format!(
                "    HbAccumulator* {} = hb_accumulator_new({});\n",
                v, scode
            ));
            return Ok((HType::Accumulator, v));
        }

        let cls = path.joined();
        let resolved = self.resolver.resolve(&cls).map_err(|e| e.0)?;
        let mangled = mangle(&cls);
        let v = fctx.fresh_tmp();
        out.push_str(&format!(
            "    Hb_{m}* {v} = (Hb_{m}*)malloc(sizeof(Hb_{m}));\n",
            m = mangled,
            v = v
        ));

        for (name, t) in &resolved.traits {
            let provided = args.iter().find_map(|a| match a {
                Arg::Named(n, e) if n == name => Some(e),
                _ => None,
            });
            let ty = self.type_of_annotation(&t.ty)?;
            let code = if let Some(e) = provided {
                let (ety, ecode) = self.gen_expr(fctx, out, e)?;
                if ety != ty {
                    return Err(format!(
                        "error: birth({}) trait '{}' expects {:?}, got {:?}",
                        cls, name, ty, ety
                    ));
                }
                ecode
            } else if let Some(def) = &t.default {
                let mut dctx = FnCtx::new(None);
                let (_, dcode) = self.gen_expr(&mut dctx, out, def)?;
                dcode
            } else {
                return Err(format!(
                    "error: line {}: birth({}) is missing required trait '{}' and it has no default",
                    t.line, cls, name
                ));
            };
            out.push_str(&format!("    {}->{} = {};\n", v, name, code));
        }

        // Reject unknown named args (typos in birth(...) calls).
        for a in args {
            if let Arg::Named(n, _) = a {
                if !resolved.traits.iter().any(|(tn, _)| tn == n) {
                    return Err(format!("error: birth({}) has no trait '{}'", cls, n));
                }
            }
        }

        Ok((HType::Class(cls), v))
    }

    fn gen_binary(
        &mut self,
        fctx: &mut FnCtx,
        out: &mut String,
        op: BinOp,
        l: &Expr,
        r: &Expr,
    ) -> CResult<(HType, String)> {
        let (lty, lcode) = self.gen_expr(fctx, out, l)?;
        let (rty, rcode) = self.gen_expr(fctx, out, r)?;

        if (op == BinOp::Eq || op == BinOp::NotEq) && lty == HType::Str && rty == HType::Str {
            let cmp = format!(
                "(strcmp({}, {}) {} 0)",
                lcode,
                rcode,
                if op == BinOp::Eq { "==" } else { "!=" }
            );
            return Ok((HType::Bool, cmp));
        }

        match op {
            BinOp::And | BinOp::Or => {
                if lty != HType::Bool || rty != HType::Bool {
                    return Err(format!(
                        "error: '{}' expects Bool operands",
                        if op == BinOp::And { "and" } else { "or" }
                    ));
                }
                let c = if op == BinOp::And { "&&" } else { "||" };
                Ok((HType::Bool, format!("({} {} {})", lcode, c, rcode)))
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                if lty != HType::Int || rty != HType::Int {
                    return Err(format!(
                        "error: comparison expects Integer operands, found {:?} and {:?}",
                        lty, rty
                    ));
                }
                let c = match op {
                    BinOp::Eq => "==",
                    BinOp::NotEq => "!=",
                    BinOp::Lt => "<",
                    BinOp::Gt => ">",
                    BinOp::LtEq => "<=",
                    BinOp::GtEq => ">=",
                    _ => unreachable!(),
                };
                Ok((HType::Bool, format!("({} {} {})", lcode, c, rcode)))
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                if lty != HType::Int || rty != HType::Int {
                    return Err(format!(
                        "error: arithmetic expects Integer operands, found {:?} and {:?}",
                        lty, rty
                    ));
                }
                let c = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "%",
                    _ => unreachable!(),
                };
                Ok((HType::Int, format!("({} {} {})", lcode, c, rcode)))
            }
        }
    }

    pub fn gen_main(&mut self, program: &Program) -> CResult<String> {
        let mut fctx = FnCtx::new(None);
        let mut out = String::new();
        self.gen_block(&mut fctx, &mut out, &program.main_stmts, &HType::Void)?;
        Ok(format!("int main(void) {{\n{}    return 0;\n}}\n", out))
    }
}

fn arg_val(a: &Arg) -> &Expr {
    match a {
        Arg::Positional(e) => e,
        Arg::Named(_, e) => e,
    }
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::resolve::Resolver;

    #[test]
    fn hapsburg_display_matches_source_syntax() {
        assert_eq!(HType::Int.hapsburg_display(), "Integer");
        assert_eq!(HType::Str.hapsburg_display(), "String");
        assert_eq!(HType::ListInt.hapsburg_display(), "List<Integer>");
        assert_eq!(HType::ListListInt.hapsburg_display(), "List<List<Integer>>");
        assert_eq!(
            HType::Accumulator.hapsburg_display(),
            "Habsburg::Accumulator"
        );
    }

    #[test]
    fn marry_same_type_is_identity() {
        let (ty, conv) = marry_conversion(&HType::Int, "Integer").unwrap();
        assert_eq!(ty, HType::Int);
        assert!(conv.is_none());
    }

    #[test]
    fn marry_str_to_int_and_back() {
        let (ty, conv) = marry_conversion(&HType::Str, "Integer").unwrap();
        assert_eq!(ty, HType::Int);
        assert_eq!(conv, Some("hb_integer_parse"));

        let (ty, conv) = marry_conversion(&HType::Int, "String").unwrap();
        assert_eq!(ty, HType::Str);
        assert_eq!(conv, Some("hb_integer_to_string"));
    }

    #[test]
    fn marry_bool_to_string() {
        let (ty, conv) = marry_conversion(&HType::Bool, "String").unwrap();
        assert_eq!(ty, HType::Str);
        assert_eq!(conv, Some("hb_bool_to_string"));
    }

    #[test]
    fn marry_unsupported_pair_is_an_error() {
        let err = marry_conversion(&HType::Bool, "Integer").unwrap_err();
        assert!(
            err.contains("no legitimate marriage"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn marry_unknown_target_name_is_an_error() {
        let err = marry_conversion(&HType::Int, "NotAType").unwrap_err();
        assert!(err.contains("NotAType"), "unexpected error: {}", err);
    }

    #[test]
    fn marry_target_name_extracts_bare_identifiers_only() {
        assert_eq!(
            marry_target_name(&Expr::Ident("Integer".into())),
            Some("Integer")
        );
        assert_eq!(
            marry_target_name(&Expr::PathExpr(Path(vec!["Integer".into()]))),
            Some("Integer")
        );
        assert_eq!(marry_target_name(&Expr::Int(1)), None);
    }

    #[test]
    fn mangle_replaces_double_colon() {
        assert_eq!(
            mangle("AdventOfCode::Y2017::Day1"),
            "AdventOfCode__Y2017__Day1"
        );
    }

    #[test]
    fn c_string_literal_escapes_quotes_and_backslashes() {
        assert_eq!(c_string_literal("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    /// End-to-end (through this module, not through `cc`): compiles a tiny
    /// two-class program and checks the generated C actually calls the
    /// *leaf* class's own override, not the ancestor's -- the concrete
    /// claim behind "flattening gives correct override semantics with
    /// zero vtables" in the README.
    #[test]
    fn inherited_method_dispatches_through_the_leaf_classs_own_override() {
        let prog = parser::parse(
            "dynasty A founder {
                 override tag() -> Integer { return 1 }
                 override describe() -> Integer { return self.tag() }
             }
             dynasty B descends A {
                 override tag() -> Integer { return 2 }
             }",
        )
        .unwrap();
        let mut resolver = Resolver::new(&prog).unwrap();
        resolver.check_all().unwrap();
        let mut cg = Codegen::new(&resolver);
        cg.gen_class("B").unwrap();

        assert!(
            cg.func_impls.contains("hb__B__describe"),
            "expected a monomorphized describe() for B, got:\n{}",
            cg.func_impls
        );
        // The whole point: describe() is textually A's body, but B's
        // generated copy must call B's own tag(), not A's.
        let describe_b = extract_function_body(&cg.func_impls, "hb__B__describe");
        assert!(
            describe_b.contains("hb__B__tag(self)"),
            "B's describe() should dispatch to B's own tag(), got:\n{}",
            describe_b
        );
        assert!(
            !describe_b.contains("hb__A__tag"),
            "B's describe() must not call A's tag(), got:\n{}",
            describe_b
        );
    }

    fn extract_function_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
        let start = src
            .find(fn_name)
            .unwrap_or_else(|| panic!("{} not found in generated C", fn_name));
        let end = src[start..]
            .find("\n}\n")
            .map(|i| start + i)
            .unwrap_or(src.len());
        &src[start..end]
    }
}
