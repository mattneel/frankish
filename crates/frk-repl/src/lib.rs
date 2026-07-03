//! frk-repl — the frankish shell's engine (SPEC §9, M8; semantics
//! D-043). Library-first so the transcript-golden runner drives the
//! EXACT code the interactive binary runs.
//!
//! Model (D-043): the session is an accumulated ml_core declaration
//! prefix, re-elaborated whole on every line — no incremental typing
//! state to corrupt; the reference interpreter (D-008) evaluates. A
//! decl line typechecks `prefix + line` and, on success, commits it
//! and prints `val name : τ` per binding (types only — values would
//! force evaluation of possibly-polymorphic/dead bindings). An
//! expression line compiles `prefix + let main () = ( line )` under
//! the REPL policy (main may return any concrete τ), interprets, and
//! prints `- : τ = value`. A failing line leaves the session
//! unchanged. Redefinition is ml shadowing, verbatim.
//!
//! Commands: `:type E`, `:load FILE` (decls, resolved against `cwd`),
//! `:emit` (kernel module of the session under the current profile's
//! strategy — requires the session to define main), `:profile
//! [arena|rc]`, `:help`, `:q`.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use frk_front::infer::TypedProgram;
use frk_front::types::Ty;
use frk_interp::{Interp, Value};

pub struct Engine {
    /// Accumulated, committed declaration source.
    prefix: String,
    /// Emission strategy for `:emit` (evaluation is always interp).
    strategy: frk_dialects::Strategy,
    /// Base directory for `:load` (the transcript case dir, or cwd).
    pub load_base: PathBuf,
    pub done: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            strategy: frk_dialects::Strategy::Arena,
            load_base: PathBuf::from("."),
            done: false,
        }
    }

    pub fn prompt(&self) -> String {
        let profile = match self.strategy {
            frk_dialects::Strategy::Arena => "arena",
            frk_dialects::Strategy::Rc => "rc",
        };
        format!("frk[{profile}]> ")
    }

    /// Feeds one line; returns the response text (may be empty or
    /// multi-line, no trailing newline guarantee — callers println).
    pub fn feed(&mut self, line: &str) -> String {
        let line = line.trim();
        if line.is_empty() {
            return String::new();
        }
        if let Some(command) = line.strip_prefix(':') {
            return self.command(command);
        }
        // Classification by the real parser (D-043): a line that
        // parses as a program is a decl; otherwise try it as an
        // expression; report the DECL error when both fail and the
        // line looks declarative.
        let as_decl = frk_front::check_ml(&format!("{}\n{}", self.prefix, line));
        match as_decl {
            Ok(typed) => self.commit_decl(line, &typed),
            Err(decl_error) => {
                let wrapped = self.wrap_expr(line);
                match self.eval_expr(&wrapped) {
                    Ok(rendered) => rendered,
                    // Every shell error echoes the offending line —
                    // the M8 exit amendment (D-044): a shell whose
                    // trap messages point at nothing ships §6.5's
                    // bug-by-law.
                    Err(expr_error) => {
                        let message = if line.starts_with("let ") || line.starts_with("type ") {
                            decl_error.to_string()
                        } else {
                            expr_error
                        };
                        format!("error: {message}\n  at: {line}")
                    }
                }
            }
        }
    }

    fn wrap_expr(&self, line: &str) -> String {
        format!("{}\nlet main () = ( {} )", self.prefix, line)
    }

    fn commit_decl(&mut self, line: &str, typed: &TypedProgram) -> String {
        // Names introduced by THIS line: parse the line alone for its
        // binding names, then read their types off the whole-program
        // elaboration (last shadowing occurrence wins).
        let mut out = String::new();
        let new_names = decl_names(line);
        if new_names.is_empty() {
            // A type definition.
            self.prefix.push_str(line);
            self.prefix.push('\n');
            return "type defined".to_string();
        }
        let mut seen = HashSet::new();
        for (_, bindings) in typed.decls.iter().rev() {
            for binding in bindings.iter().rev() {
                if new_names.contains(&binding.name) && seen.insert(binding.name.clone()) {
                    let _ = writeln!(out, "val {} : {}", binding.name, pretty_ty(&binding.expr.ty));
                }
            }
            if seen.len() == new_names.len() {
                break;
            }
        }
        self.prefix.push_str(line);
        self.prefix.push('\n');
        out.trim_end().to_string()
    }

    fn eval_expr(&self, source: &str) -> Result<String, String> {
        // Typecheck first: a polymorphic result (only functions can be,
        // value restriction) renders as <fun> without emission — there
        // is no concrete type to emit at (D-043).
        let typed = frk_front::check_ml(source).map_err(|e| e.to_string())?;
        let ty = typed
            .main_result
            .clone()
            .ok_or_else(|| "no expression to evaluate".to_string())?;
        if has_vars(&ty) {
            if matches!(ty, Ty::Fun(..)) {
                return Ok(format!("- : {} = <fun>", pretty_ty(&ty)));
            }
            return Err(format!("ambiguous type {}", pretty_ty(&ty)));
        }
        let context = frk_core::context();
        frk_dialects::register(&context).map_err(|e| e.to_string())?;
        let (module, ty) =
            frk_front::compile_ml_any(&context, source).map_err(|e| e.to_string())?;
        frk_dialects::verify(&context, &module).map_err(|e| e.to_string())?;
        let mut interp = Interp::new(&module).map_err(|e| e.to_string())?;
        frk_dialects::register_eval(&mut interp);
        let values = interp.eval_function("main", &[]).map_err(|e| e.to_string())?;
        Ok(format!(
            "- : {} = {}",
            pretty_ty(&ty),
            render(&typed, &ty, &values[0])
        ))
    }

    fn command(&mut self, command: &str) -> String {
        let (name, rest) = match command.split_once(char::is_whitespace) {
            Some((name, rest)) => (name, rest.trim()),
            None => (command, ""),
        };
        match name {
            "q" | "quit" => {
                self.done = true;
                String::new()
            }
            "help" => "commands: :type EXPR | :load FILE | :emit | :profile [arena|rc] | :q"
                .to_string(),
            "type" => {
                if rest.is_empty() {
                    return "usage: :type EXPR".to_string();
                }
                match frk_front::check_ml(&self.wrap_expr(rest)) {
                    Ok(typed) => match typed.main_result {
                        Some(ty) => format!("- : {}", pretty_ty(&ty)),
                        None => "error: no type".to_string(),
                    },
                    Err(error) => format!("error: {error}"),
                }
            }
            "load" => {
                if rest.is_empty() {
                    return "usage: :load FILE".to_string();
                }
                let path = self.load_base.join(rest);
                let contents = match std::fs::read_to_string(&path) {
                    Ok(contents) => contents,
                    // Name only what was asked for — the resolved path
                    // depends on the process cwd (portability).
                    Err(error) => return format!("error: {rest}: {error}"),
                };
                let candidate = format!("{}\n{}", self.prefix, contents);
                match frk_front::check_ml(&candidate) {
                    Ok(_) => {
                        self.prefix = candidate;
                        format!("loaded {rest}")
                    }
                    Err(error) => format!("error: {error}"),
                }
            }
            "emit" => {
                let context = frk_core::context();
                if let Err(error) = frk_dialects::register(&context) {
                    return format!("error: {error}");
                }
                match frk_front::compile_ml(&context, &self.prefix) {
                    Ok(module) => {
                        // The session's kernel module, pre-lowering —
                        // strategy applies at lowering; show which.
                        format!(
                            "// profile: {:?} (strategy applies at lowering)\n{}",
                            self.strategy,
                            module.as_operation()
                        )
                    }
                    Err(error) => format!("error: {error} (:emit needs the session to define main)"),
                }
            }
            "profile" => match rest {
                "" => format!("profile: {:?}; evaluation: interp (reference, D-008)", self.strategy),
                "arena" => {
                    self.strategy = frk_dialects::Strategy::Arena;
                    "profile: Arena".to_string()
                }
                "rc" => {
                    self.strategy = frk_dialects::Strategy::Rc;
                    "profile: Rc".to_string()
                }
                other => format!("error: unknown profile {other:?} (arena|rc)"),
            },
            other => format!("error: unknown command :{other} (:help lists them)"),
        }
    }
}

/// Binding names a single decl line introduces (empty for typedefs).
fn decl_names(line: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let Ok(program) = frk_front::ast::parse(line) else {
        return names;
    };
    for (_, bindings) in &program.decls {
        for binding in bindings {
            names.insert(binding.name.clone());
        }
    }
    names
}

/// Pretty-prints a type with leftover unification variables shown as
/// scheme variables ('a, 'b, ...) in first-appearance order (D-043).
fn pretty_ty(ty: &Ty) -> String {
    fn walk(ty: &Ty, names: &mut Vec<u32>, out: &mut String) {
        match ty {
            Ty::Var(vid) => {
                let index = match names.iter().position(|seen| *seen == vid.0) {
                    Some(index) => index,
                    None => {
                        names.push(vid.0);
                        names.len() - 1
                    }
                };
                out.push('\'');
                out.push((b'a' + (index % 26) as u8) as char);
            }
            Ty::Tuple(items) => {
                out.push('(');
                for (position, item) in items.iter().enumerate() {
                    if position > 0 {
                        out.push_str(" * ");
                    }
                    walk(item, names, out);
                }
                out.push(')');
            }
            Ty::Fun(a, b) => {
                out.push('(');
                walk(a, names, out);
                out.push_str(" -> ");
                walk(b, names, out);
                out.push(')');
            }
            other => {
                let _ = write!(out, "{other}");
            }
        }
    }
    let mut names = Vec::new();
    let mut out = String::new();
    walk(ty, &mut names, &mut out);
    out
}

fn has_vars(ty: &Ty) -> bool {
    match ty {
        Ty::Var(_) => true,
        Ty::Tuple(items) => items.iter().any(has_vars),
        Ty::Fun(a, b) => has_vars(a) || has_vars(b),
        _ => false,
    }
}

/// Renders an interpreter value against its zonked ml type (D-043).
fn render(program: &TypedProgram, ty: &Ty, value: &Value) -> String {
    match ty {
        Ty::Unit => "()".to_string(),
        Ty::Int => value
            .as_signed()
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "<corrupt int>".into()),
        Ty::Bool => match value.as_bool() {
            Ok(true) => "true".into(),
            Ok(false) => "false".into(),
            Err(_) => "<corrupt bool>".into(),
        },
        Ty::Tuple(items) => match value.as_adt() {
            Ok((_, fields)) if fields.len() == items.len() => {
                let parts: Vec<String> = items
                    .iter()
                    .zip(fields)
                    .map(|(item_ty, field)| render(program, item_ty, field))
                    .collect();
                format!("({})", parts.join(", "))
            }
            _ => "<corrupt tuple>".into(),
        },
        Ty::Adt(name) => {
            let Ok((tag, fields)) = value.as_adt() else {
                return "<corrupt adt>".into();
            };
            let Some(info) = program.adts.get(name) else {
                return format!("<{name}?>");
            };
            let Some((ctor, payload)) = info.ctors.get(tag as usize) else {
                return format!("<{name}#{tag}?>");
            };
            if payload.is_empty() {
                ctor.clone()
            } else if payload.len() == 1 {
                format!("{ctor} {}", render(program, &payload[0], &fields[0]))
            } else {
                let parts: Vec<String> = payload
                    .iter()
                    .zip(fields)
                    .map(|(field_ty, field)| render(program, field_ty, field))
                    .collect();
                format!("{ctor} ({})", parts.join(", "))
            }
        }
        Ty::Fun(..) => "<fun>".to_string(),
        Ty::Var(_) => "<unresolved>".to_string(),
    }
}

/// Runs a whole scripted transcript: echoes `PROMPT line` for every
/// input, then the response — the transcript-golden format (D-043).
pub fn run_transcript(input: &str, load_base: PathBuf) -> String {
    let mut engine = Engine::new();
    engine.load_base = load_base;
    let mut out = String::new();
    for line in input.lines() {
        if engine.done {
            break;
        }
        let _ = writeln!(out, "{}{}", engine.prompt(), line);
        let response = engine.feed(line);
        if !response.is_empty() {
            let _ = writeln!(out, "{response}");
        }
    }
    out
}
