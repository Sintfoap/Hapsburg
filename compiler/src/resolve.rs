use crate::ast::*;
use std::collections::{HashMap, HashSet};

/// A fully-resolved (flattened) view of one dynasty: its C3 linearization,
/// and the winning definition of every trait and method reachable from it.
pub struct Resolved<'a> {
    pub linearization: Vec<String>, // most-derived first, includes self
    pub traits: Vec<(String, &'a Trait)>, // name -> winning trait def, insertion order
    pub methods: HashMap<String, (String, &'a Method)>, // name -> (owning class, method)
}

pub struct Resolver<'a> {
    pub classes: HashMap<String, &'a Dynasty>,
    lin_cache: HashMap<String, Vec<String>>,
    warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ResolveError(pub String);

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

type RResult<T> = Result<T, ResolveError>;

impl<'a> Resolver<'a> {
    pub fn new(program: &'a Program) -> RResult<Self> {
        let mut classes = HashMap::new();
        for d in &program.dynasties {
            let key = d.name.joined();
            if classes.insert(key.clone(), d).is_some() {
                return Err(ResolveError(format!(
                    "error: dynasty '{}' declared more than once",
                    key
                )));
            }
        }
        Ok(Resolver { classes, lin_cache: HashMap::new(), warnings: Vec::new() })
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Validate founder/parent-count rules and existence of ancestors, then
    /// compute every class's linearization (forcing InbreedingError / cycle
    /// detection eagerly so problems surface before codegen).
    pub fn check_all(&mut self) -> RResult<()> {
        let names: Vec<String> = self.classes.keys().cloned().collect();
        for name in &names {
            let d = self.classes[name];
            if d.founder && !d.parents.is_empty() {
                return Err(ResolveError(format!(
                    "error: line {}: '{}' is declared 'founder' but lists ancestors ({}) — a founder claims no ancestry",
                    d.line, name, d.parents.iter().map(|p| p.joined()).collect::<Vec<_>>().join(", ")
                )));
            }
            if !d.founder && d.parents.is_empty() {
                return Err(ResolveError(format!(
                    "error: line {}: '{}' has no ancestors — descend from something with 'descends', or mark it 'founder'",
                    d.line, name
                )));
            }
            for p in &d.parents {
                if !self.classes.contains_key(&p.joined()) {
                    return Err(ResolveError(format!(
                        "error: line {}: '{}' claims descent from unknown dynasty '{}'",
                        d.line, name, p.joined()
                    )));
                }
            }
            if !d.founder && d.parents.len() == 1 {
                self.warnings.push(format!(
                    "warning: '{}' has a single line of descent — consider marrying in a second parent (line of succession weakened)",
                    name
                ));
            }
        }

        for name in &names {
            self.linearize(name, &mut Vec::new())?;
        }

        self.check_genetic_diversity(&names)?;
        Ok(())
    }

    /// C3 linearization, memoized. `stack` tracks the in-progress chain for
    /// cycle detection.
    pub fn linearize(&mut self, name: &str, stack: &mut Vec<String>) -> RResult<Vec<String>> {
        if let Some(l) = self.lin_cache.get(name) {
            return Ok(l.clone());
        }
        if stack.contains(&name.to_string()) {
            stack.push(name.to_string());
            return Err(ResolveError(format!(
                "error: cyclic line of succession: {}",
                stack.join(" -> ")
            )));
        }
        stack.push(name.to_string());

        let d = *self.classes.get(name).ok_or_else(|| {
            ResolveError(format!("error: unknown dynasty '{}'", name))
        })?;

        let mut parent_lins: Vec<Vec<String>> = Vec::new();
        for p in &d.parents {
            parent_lins.push(self.linearize(&p.joined(), stack)?);
        }
        let parent_order: Vec<String> = d.parents.iter().map(|p| p.joined()).collect();

        stack.pop();

        let lin = c3_merge(name, parent_lins, parent_order).map_err(|conflict| {
            ResolveError(format!(
                "InbreedingError: line {}: cannot reconcile the ancestry of '{}' — conflicting lines of succession among {}. Pedigree so far: [{}]",
                d.line, name, conflict.join(", "), self.lin_cache.get(name).cloned().unwrap_or_default().join(" -> ")
            ))
        })?;

        self.lin_cache.insert(name.to_string(), lin.clone());
        Ok(lin)
    }

    /// The pitch's joke check: if the whole program's inheritance graph is
    /// one fully-connected clique (every class related to every other),
    /// ferdinand refuses to compile — "no genetic diversity".
    ///
    /// A plain single-inheritance chain is *always* a total order (hence
    /// always "fully connected" under the ancestor relation), so we only
    /// run this check when the program actually exercises multiple
    /// inheritance somewhere — otherwise every ordinary linear hierarchy
    /// would be rejected, which isn't the joke's intent.
    fn check_genetic_diversity(&mut self, names: &[String]) -> RResult<()> {
        if names.len() < 2 {
            return Ok(());
        }
        let uses_multiple_inheritance = names.iter().any(|n| self.classes[n].parents.len() >= 2);
        if !uses_multiple_inheritance {
            return Ok(());
        }
        let mut ancestors: HashMap<&str, HashSet<&str>> = HashMap::new();
        for n in names {
            let lin = &self.lin_cache[n];
            ancestors.insert(n.as_str(), lin.iter().map(|s| s.as_str()).collect());
        }
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                let a = names[i].as_str();
                let b = names[j].as_str();
                let related = ancestors[a].contains(b) || ancestors[b].contains(a);
                if !related {
                    return Ok(()); // found genetic diversity; all clear
                }
            }
        }
        Err(ResolveError(
            "error: no genetic diversity, refusing to compile — every dynasty in this program descends from every other. see Alfonso XII.".to_string(),
        ))
    }

    /// Build the flattened trait/method table for a class from its
    /// linearization: most-derived definition wins for each name.
    pub fn resolve(&self, name: &str) -> RResult<Resolved<'a>> {
        let lin = self.lin_cache.get(name).cloned().ok_or_else(|| {
            ResolveError(format!("error: '{}' was never linearized", name))
        })?;

        let mut traits: Vec<(String, &'a Trait)> = Vec::new();
        let mut seen_traits: HashSet<String> = HashSet::new();
        let mut methods: HashMap<String, (String, &'a Method)> = HashMap::new();
        let mut seen_methods: HashSet<String> = HashSet::new();

        for cname in &lin {
            let d = self.classes[cname];
            for t in &d.traits {
                if seen_traits.insert(t.name.clone()) {
                    traits.push((t.name.clone(), t));
                }
            }
            for m in &d.methods {
                if seen_methods.insert(m.name.clone()) {
                    methods.insert(m.name.clone(), (cname.clone(), m));
                }
            }
        }

        Ok(Resolved { linearization: lin, traits, methods })
    }
}

/// Standard C3 merge. `parent_lins[i]` is the already-computed linearization
/// of `d.parents[i]`; `parent_order` is the parents themselves in
/// declaration order (the final list merged in).
/// On failure, returns the heads of the remaining sequences as the conflict.
fn c3_merge(
    class: &str,
    parent_lins: Vec<Vec<String>>,
    parent_order: Vec<String>,
) -> Result<Vec<String>, Vec<String>> {
    let mut sequences: Vec<Vec<String>> = parent_lins;
    sequences.push(parent_order);
    let mut result = vec![class.to_string()];

    loop {
        sequences.retain(|s| !s.is_empty());
        if sequences.is_empty() {
            break;
        }
        let mut candidate: Option<String> = None;
        for seq in &sequences {
            let head = &seq[0];
            let in_tail = sequences.iter().any(|s2| s2[1..].contains(head));
            if !in_tail {
                candidate = Some(head.clone());
                break;
            }
        }
        match candidate {
            Some(c) => {
                result.push(c.clone());
                for seq in sequences.iter_mut() {
                    seq.retain(|x| x != &c);
                }
            }
            None => {
                let heads: Vec<String> = sequences.iter().map(|s| s[0].clone()).collect();
                return Err(heads);
            }
        }
    }
    Ok(result)
}
