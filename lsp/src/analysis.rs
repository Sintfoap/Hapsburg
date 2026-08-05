use ferdinand::ast::{self, MethodBody};
use ferdinand::codegen::Codegen;
use ferdinand::parser;
use ferdinand::resolve::Resolver;
use std::collections::{HashMap, HashSet};
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

#[derive(Clone)]
pub struct TraitInfo {
    pub name: String,
    pub ty_display: String,
    pub line: u32, // 0-indexed, LSP convention
}

#[derive(Clone)]
pub struct MethodInfo {
    pub name: String,
    pub sig_display: String,
    pub owner: String,
    pub line: u32,
    pub is_abstract: bool,
}

#[derive(Clone)]
pub struct ClassInfo {
    pub name: String,
    pub line: u32,
    pub founder: bool,
    pub parents: Vec<String>,
    /// C3 linearization, most-derived first. Empty if this class (or an
    /// ancestor) failed to linearize — see the module doc comment on why
    /// that doesn't block the rest of the document's info.
    pub pedigree: Vec<String>,
    pub traits: Vec<TraitInfo>,
    pub methods: Vec<MethodInfo>,
}

#[derive(Clone, Default)]
pub struct DocAnalysis {
    pub diagnostics: Vec<Diagnostic>,
    pub classes: HashMap<String, ClassInfo>,
    /// (start line, class name), sorted by line — used to approximate
    /// "which dynasty/method contains this position" for local-variable
    /// hover, since the AST doesn't track end positions.
    pub class_order: Vec<(u32, String)>,
    /// (declaration line, variable name, Hapsburg-syntax type).
    pub var_hints: Vec<(u32, String, String)>,
}

fn line0(line: usize) -> u32 {
    line.saturating_sub(1) as u32
}

fn diagnostic(msg: &str, severity: DiagnosticSeverity) -> Diagnostic {
    let line = extract_line(msg);
    let range = Range::new(Position::new(line, 0), Position::new(line, u32::MAX));
    Diagnostic {
        range,
        severity: Some(severity),
        source: Some("ferdinand".to_string()),
        message: msg.to_string(),
        ..Diagnostic::default()
    }
}

/// Best-effort "line N" extraction from ferdinand's plain-string errors —
/// see lsp/README.md for which error classes carry real line numbers and
/// which don't yet.
fn extract_line(msg: &str) -> u32 {
    if let Some(idx) = msg.find("line ") {
        let rest = &msg[idx + 5..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = digits.parse::<usize>() {
            if n > 0 {
                return line0(n);
            }
        }
    }
    0
}

fn method_sig(m: &ast::Method) -> String {
    let params = m
        .params
        .iter()
        .map(|p| format!("{}: {}", p.name, p.ty))
        .collect::<Vec<_>>()
        .join(", ");
    match &m.ret {
        Some(t) => format!("override {}({}) -> {}", m.name, params, t),
        None => format!("override {}({})", m.name, params),
    }
}

/// Runs the real compiler pipeline (parse -> resolve -> dry-run codegen,
/// no C emitted, no `cc` invoked) and extracts everything the LSP's
/// hover/symbols/completion/definition handlers need. Unlike the CLI,
/// this doesn't stop at the very first error: a resolver failure still
/// stops the whole analysis (nothing downstream has anything to work
/// with), but once dynasties are declared we try to resolve *every* one
/// independently and keep whatever succeeds, and codegen keeps going
/// class-by-class rather than bailing out after the first one — so a
/// program that's mid-edit and half-broken still gets hover/symbols for
/// the parts that are fine.
pub fn analyze(text: &str) -> DocAnalysis {
    let mut result = DocAnalysis::default();

    let program = match parser::parse(text) {
        Ok(p) => p,
        Err(e) => {
            result.diagnostics.push(diagnostic(&e, DiagnosticSeverity::ERROR));
            return result;
        }
    };

    let mut resolver = match Resolver::new(&program) {
        Ok(r) => r,
        Err(e) => {
            result.diagnostics.push(diagnostic(&e.0, DiagnosticSeverity::ERROR));
            return result;
        }
    };

    if let Err(e) = resolver.check_all() {
        result.diagnostics.push(diagnostic(&e.0, DiagnosticSeverity::ERROR));
    }
    for w in resolver.warnings() {
        result.diagnostics.push(diagnostic(w, DiagnosticSeverity::WARNING));
    }

    let names: Vec<String> = resolver.classes.keys().cloned().collect();
    for name in &names {
        let d = resolver.classes[name];
        result.class_order.push((line0(d.line), name.clone()));

        let mut info = ClassInfo {
            name: name.clone(),
            line: line0(d.line),
            founder: d.founder,
            parents: d.parents.iter().map(|p| p.joined()).collect(),
            pedigree: Vec::new(),
            traits: Vec::new(),
            methods: Vec::new(),
        };

        // linearize()/resolve() are memoized and independent of whether
        // check_all() ran to completion, so this recovers full info for
        // every class that's individually well-formed even when an
        // earlier class in the file has a structural error.
        if resolver.linearize(name, &mut Vec::new()).is_ok() {
            if let Ok(resolved) = resolver.resolve(name) {
                info.pedigree = resolved.linearization.clone();
                for (tname, t) in &resolved.traits {
                    info.traits.push(TraitInfo {
                        name: tname.clone(),
                        ty_display: t.ty.to_string(),
                        line: line0(t.line),
                    });
                }
                for (mname, (owner, m)) in &resolved.methods {
                    info.methods.push(MethodInfo {
                        name: mname.clone(),
                        sig_display: method_sig(m),
                        owner: owner.clone(),
                        line: line0(m.line),
                        is_abstract: matches!(m.body, MethodBody::Abstract),
                    });
                }
            }
        }

        result.classes.insert(name.clone(), info);
    }
    result.class_order.sort_by_key(|(line, _)| *line);

    let mut cg = Codegen::new(&resolver);
    let birthed = cg.discover_birthed(&program);
    for class in &birthed {
        if let Err(e) = cg.gen_class(class) {
            result.diagnostics.push(diagnostic(&e, DiagnosticSeverity::ERROR));
        }
    }
    if let Err(e) = cg.gen_main(&program) {
        result.diagnostics.push(diagnostic(&e, DiagnosticSeverity::ERROR));
    }

    let mut seen = HashSet::new();
    for (line, name, ty) in &cg.var_hints {
        let display = ty.hapsburg_display();
        let key = (*line, name.clone(), display.clone());
        if seen.insert(key) {
            result.var_hints.push((line0(*line), name.clone(), display));
        }
    }

    result
}
