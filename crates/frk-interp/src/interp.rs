//! The derived interpreter (SPEC §7.1): a generic walker over MLIR IR
//! dispatching per-op [`Eval`] implementations. From M2 on it is the
//! project's reference semantics (D-008): every other execution path must
//! byte-match it on every golden (law L3).

use std::cell::Cell;
use std::collections::HashMap;

use melior::ir::attribute::StringAttribute;
use melior::ir::operation::OperationLike;
use melior::ir::{BlockLike, BlockRef, Module, OperationRef, RegionLike, Value as IrValue, ValueLike};

use crate::error::EvalError;
use crate::value::Value;

/// Call-depth ceiling (D-029): deep recursion traps deterministically
/// instead of exhausting the host stack somewhere runner-dependent.
///
/// The interpreter recurses on the host stack (a few KiB per interpreted
/// frame), so reaching this ceiling needs roughly 8 MiB of stack. Run
/// deep interpretation on a thread sized accordingly — see
/// [`STACK_SIZE`] — as the harness runner does; 2 MiB default test
/// threads are not enough.
pub const MAX_CALL_DEPTH: usize = 1024;

/// Stack size that comfortably hosts [`MAX_CALL_DEPTH`] interpreted
/// frames. melior IR handles are not Send, so spawn the thread around
/// the whole parse+interpret unit, not mid-flight.
pub const STACK_SIZE: usize = 64 * 1024 * 1024;

/// What executing one operation does to control flow.
pub enum Step<'c, 'a> {
    /// Plain op: results bound, fall through to the next op.
    Continue,
    /// Function-level return (func.return).
    Return(Vec<Value>),
    /// CFG edge with block arguments (cf.br / cf.cond_br).
    Branch(BlockRef<'c, 'a>, Vec<Value>),
    /// Structured-region exit (scf.yield).
    Yield(Vec<Value>),
    /// A call in TAIL POSITION (its sole result feeds the immediately
    /// following func.return): the frame is REPLACED, not stacked —
    /// proper tail calls as law (D-029's exemption, cashed at M14).
    TailCall(String, Vec<Value>),
}

/// How a CFG region concluded (D-059): a value return, or a tail call
/// for the caller's trampoline to continue.
pub enum CfgOutcome {
    Return(Vec<Value>),
    TailCall(String, Vec<Value>),
}

/// K2 (SPEC §3): one op's executable semantics. Upstream dialects get
/// adapter impls in [`crate::upstream`]; kernel dialect ops implement
/// this as part of their contract from M3 on.
pub trait Eval {
    fn eval<'c, 'a>(
        &self,
        interp: &Interp<'c, 'a>,
        frame: &mut Frame,
        op: OperationRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError>;
}

/// SSA environment for one function activation, keyed by MLIR value
/// identity (the C-API pointer). Meaningful only against the one module
/// the interpreter walks — which is all a frame ever sees.
#[derive(Default)]
pub struct Frame {
    slots: HashMap<usize, Value>,
}

fn slot_key(value: IrValue) -> usize {
    value.to_raw().ptr as usize
}

impl Frame {
    pub fn get(&self, value: IrValue) -> Result<Value, EvalError> {
        self.slots.get(&slot_key(value)).cloned().ok_or_else(|| {
            EvalError::Malformed(format!("use of unbound SSA value: {value}"))
        })
    }

    pub fn set(&mut self, target: IrValue, value: Value) {
        self.slots.insert(slot_key(target), value);
    }
}

/// A host-provided function: called for `func.call` to a symbol with
/// no body (runtime externals — prints, M9/D-046). Receives the
/// argument values and the shared output buffer.
pub type Builtin = Box<dyn Fn(&[Value], &mut String) -> Result<Vec<Value>, EvalError>>;

pub struct Interp<'c, 'a> {
    registry: HashMap<&'static str, Box<dyn Eval>>,
    functions: HashMap<String, OperationRef<'c, 'a>>,
    builtins: HashMap<String, Builtin>,
    /// Program output (println-style builtins append; D-045: once the
    /// shell can OBSERVE effects, replay semantics must be revisited).
    output: std::cell::RefCell<String>,
    depth: Cell<usize>,
    /// Live prompt tokens, innermost last (κ_frk, D-060). Strictly
    /// LIFO — `frk_ctl.prompt` pushes on entry and pops on exit.
    ctl_prompts: std::cell::RefCell<Vec<i64>>,
    /// Monotonic prompt-token source. Never reused within a run, so a
    /// stale escape can never alias a fresh prompt (no ABA).
    ctl_next_token: Cell<i64>,
    /// The value of the single in-flight abort. Safe as one slot: an
    /// abort unwinds atomically (no user code runs between the abort
    /// raising the signal and its prompt catching it).
    ctl_aborted: std::cell::RefCell<Option<Value>>,
    /// Global cells (D-078): one shared Value::Box per sym, created
    /// lazily zeroed. The reference mirror of native zeroinitialized
    /// module globals.
    globals: std::cell::RefCell<HashMap<String, Value>>,
    /// Live effect handlers, innermost last (κ_frk v1, D-069):
    /// (label, clause closure, handle token, masked). Masked entries
    /// are skipped by dispatch — the handler-free-for-ℓ context rule
    /// during a clause call; the mask lifting is the deep reinstall.
    ctl_handlers: std::cell::RefCell<Vec<(String, Value, i64, bool)>>,
    /// One-shot resumer markers: marker id → consumed (D-069).
    ctl_markers: std::cell::RefCell<std::collections::HashMap<i64, bool>>,
}

impl<'c, 'a> Interp<'c, 'a> {
    /// Indexes the module's `func.func` symbols and arms the upstream
    /// evaluator registry.
    pub fn new(module: &'a Module<'c>) -> Result<Self, EvalError> {
        let mut functions = HashMap::new();
        let mut next = module.body().first_operation();
        while let Some(op) = next {
            if op_name(op)? == "func.func" {
                let attribute = op.attribute("sym_name").map_err(|_| {
                    EvalError::Malformed("func.func without sym_name".into())
                })?;
                let name = StringAttribute::try_from(attribute)
                    .map_err(|_| EvalError::Malformed("non-string sym_name".into()))?
                    .value()
                    .to_string();
                functions.insert(name, op);
            }
            next = op.next_in_block();
        }
        Ok(Self {
            registry: crate::upstream::register_all(),
            functions,
            builtins: HashMap::new(),
            output: std::cell::RefCell::new(String::new()),
            depth: Cell::new(0),
            ctl_prompts: std::cell::RefCell::new(Vec::new()),
            ctl_next_token: Cell::new(1),
            ctl_aborted: std::cell::RefCell::new(None),
            globals: std::cell::RefCell::new(HashMap::new()),
            ctl_handlers: std::cell::RefCell::new(Vec::new()),
            ctl_markers: std::cell::RefCell::new(std::collections::HashMap::new()),
        })
    }

    /// Installs an effect handler over the prompt `token` (D-069).
    pub fn ctl_push_handler(&self, label: &str, clause: Value, token: i64) {
        self.ctl_handlers
            .borrow_mut()
            .push((label.to_string(), clause, token, false));
    }

    /// Removes the handler for `token` (LIFO; defensive scan).
    pub fn ctl_pop_handler(&self, token: i64) {
        let mut handlers = self.ctl_handlers.borrow_mut();
        if let Some(position) = handlers.iter().rposition(|(_, _, t, _)| *t == token) {
            handlers.remove(position);
        }
    }

    /// Innermost UNMASKED handler for `label`: masks it and returns
    /// (index, clause, token). None ⇒ the unhandled-effect trap.
    pub fn ctl_find_and_mask(&self, label: &str) -> Option<(usize, Value, i64)> {
        let mut handlers = self.ctl_handlers.borrow_mut();
        for index in (0..handlers.len()).rev() {
            let (l, _, _, masked) = &handlers[index];
            if !*masked && l == label {
                handlers[index].3 = true;
                let (_, clause, token, _) = handlers[index].clone();
                return Some((index, clause, token));
            }
        }
        None
    }

    /// Lifts the dispatch mask (the deep reinstall).
    pub fn ctl_unmask(&self, index: usize) {
        if let Some(entry) = self.ctl_handlers.borrow_mut().get_mut(index) {
            entry.3 = false;
        }
    }

    /// A fresh one-shot resumer marker (same monotonic source as
    /// prompt tokens — never reused).
    pub fn ctl_new_marker(&self) -> i64 {
        let marker = self.ctl_next_token.get();
        self.ctl_next_token.set(marker + 1);
        self.ctl_markers.borrow_mut().insert(marker, false);
        marker
    }

    /// Consumes a marker; the second consumption is the κ_frk trap.
    pub fn ctl_consume_marker(&self, marker: i64) -> Result<(), EvalError> {
        let mut markers = self.ctl_markers.borrow_mut();
        match markers.get_mut(&marker) {
            Some(consumed) if !*consumed => {
                *consumed = true;
                Ok(())
            }
            Some(_) => Err(EvalError::Trap("one-shot violation (κ_frk)".into())),
            None => Err(EvalError::Malformed(
                "resume of an unknown marker (κ_frk)".into(),
            )),
        }
    }

    /// Was this marker consumed? (The perform-site decision.)
    pub fn ctl_marker_consumed(&self, marker: i64) -> bool {
        self.ctl_markers
            .borrow()
            .get(&marker)
            .copied()
            .unwrap_or(false)
    }

    /// Installs a fresh prompt and returns its token (κ_frk §2).
    pub fn ctl_push_prompt(&self) -> i64 {
        let token = self.ctl_next_token.get();
        self.ctl_next_token.set(token + 1);
        self.ctl_prompts.borrow_mut().push(token);
        token
    }

    /// Removes `token` and anything still nested above it (LIFO; the
    /// truncate is defensive — a well-typed run pops the exact top).
    pub fn ctl_pop_prompt(&self, token: i64) {
        let mut prompts = self.ctl_prompts.borrow_mut();
        if let Some(position) = prompts.iter().rposition(|&t| t == token) {
            prompts.truncate(position);
        }
    }

    /// Is `token` a live prompt? A dead token is the "escape past
    /// extent" trap's trigger.
    pub fn ctl_prompt_live(&self, token: i64) -> bool {
        self.ctl_prompts.borrow().contains(&token)
    }

    /// Parks the aborting value for the catching prompt to collect.
    /// The shared cell for a global sym (D-078), created lazily with
    /// `zero` on first touch. Every caller gets the SAME Value::Box.
    pub fn global_cell(&self, sym: &str, zero: Value) -> Value {
        self.globals
            .borrow_mut()
            .entry(sym.to_string())
            .or_insert_with(|| Value::boxed(zero))
            .clone()
    }

    pub fn ctl_set_aborted(&self, value: Value) {
        *self.ctl_aborted.borrow_mut() = Some(value);
    }

    /// Collects the parked abort value (the prompt whose token matched).
    pub fn ctl_take_aborted(&self) -> Result<Value, EvalError> {
        self.ctl_aborted.borrow_mut().take().ok_or_else(|| {
            EvalError::Malformed("caught an abort with no parked value (κ_frk)".into())
        })
    }

    /// The printed input types of a module function, for evaluators
    /// that dispatch on a callee's convention (D-063: closure_eval
    /// checks whether input 0 is the uniform envref). Generic — the
    /// interpreter itself knows no dialect types.
    pub fn function_input_types(&self, name: &str) -> Option<Vec<String>> {
        let function = self.functions.get(name)?;
        let attribute = function.attribute("function_type").ok()?;
        let function_type =
            melior::ir::r#type::FunctionType::try_from(
                melior::ir::attribute::TypeAttribute::try_from(attribute).ok()?.value(),
            )
            .ok()?;
        let mut inputs = Vec::with_capacity(function_type.input_count());
        for index in 0..function_type.input_count() {
            inputs.push(function_type.input(index).ok()?.to_string());
        }
        Some(inputs)
    }

    /// Registers a host builtin for calls to a bodyless symbol.
    pub fn register_builtin(&mut self, symbol: impl Into<String>, builtin: Builtin) {
        self.builtins.insert(symbol.into(), builtin);
    }

    /// Coverage probe (D-062): is a builtin registered for `symbol`?
    pub fn has_builtin(&self, symbol: &str) -> bool {
        self.builtins.contains_key(symbol)
    }

    /// Drains everything builtins printed so far.
    pub fn take_output(&self) -> String {
        std::mem::take(&mut self.output.borrow_mut())
    }

    pub(crate) fn call_builtin(
        &self,
        symbol: &str,
        arguments: &[Value],
    ) -> Option<Result<Vec<Value>, EvalError>> {
        let builtin = self.builtins.get(symbol)?;
        Some(builtin(arguments, &mut self.output.borrow_mut()))
    }

    /// Registers (or overrides) the evaluator for one op — the K2 hook
    /// kernel dialects use to plug their semantics in.
    pub fn register_eval(&mut self, op: &'static str, evaluator: Box<dyn Eval>) {
        self.registry.insert(op, evaluator);
    }

    /// Calls `name(args)` and returns its results. This is both the public
    /// entry and the path `func.call` re-enters.
    pub fn eval_function(&self, name: &str, args: &[Value]) -> Result<Vec<Value>, EvalError> {
        // The trampoline (D-059): tail calls REPLACE the frame — the
        // loop below runs successive tail callees at ONE depth unit.
        let mut name = name.to_string();
        let mut args = args.to_vec();
        let mut counted = false;
        let result = loop {
            // Host builtins answer only for absent/bodyless symbols.
            let bodyless = match self.functions.get(&name) {
                None => true,
                Some(function) => function
                    .region(0)
                    .map(|region| region.first_block().is_none())
                    .unwrap_or(true),
            };
            if bodyless {
                if let Some(result) = self.call_builtin(&name, &args) {
                    break result;
                }
            }
            let function = match self.functions.get(&name) {
                Some(function) => *function,
                None => break Err(EvalError::CalleeNotFound(name.clone())),
            };
            if !counted {
                if self.depth.get() >= MAX_CALL_DEPTH {
                    return Err(EvalError::Trap(format!(
                        "call depth exceeded {MAX_CALL_DEPTH} frames (D-029)"
                    )));
                }
                self.depth.set(self.depth.get() + 1);
                counted = true;
            }
            match self.run_body(function, &args) {
                Ok(CfgOutcome::Return(values)) => break Ok(values),
                Ok(CfgOutcome::TailCall(next, next_args)) => {
                    name = next;
                    args = next_args;
                }
                Err(error) => break Err(error),
            }
        };
        if counted {
            self.depth.set(self.depth.get() - 1);
        }
        result
    }

    /// Detects the tail shape at `call`: its results are EXACTLY the
    /// operands of the immediately following func.return.
    fn tail_shape(
        &self,
        frame: &Frame,
        call: OperationRef<'c, 'a>,
    ) -> Result<Option<Step<'c, 'a>>, EvalError> {
        let Some(following) = call.next_in_block() else {
            return Ok(None);
        };
        if op_name(following)? != "func.return" {
            return Ok(None);
        }
        if call.result_count() != following.operand_count() {
            return Ok(None);
        }
        for index in 0..call.result_count() {
            let result = call.result(index).map_err(|_| {
                EvalError::Malformed("call result vanished".into())
            })?;
            let operand = following.operand(index).map_err(|_| {
                EvalError::Malformed("return operand vanished".into())
            })?;
            if result.to_raw().ptr != operand.to_raw().ptr {
                return Ok(None);
            }
        }
        let callee = call
            .attribute("callee")
            .ok()
            .and_then(|attribute| {
                melior::ir::attribute::FlatSymbolRefAttribute::try_from(attribute).ok()
            })
            .ok_or_else(|| EvalError::Malformed("func.call without callee".into()))?
            .value()
            .to_string();
        let mut args = Vec::with_capacity(call.operand_count());
        for index in 0..call.operand_count() {
            let operand = call.operand(index).map_err(|_| {
                EvalError::Malformed("call operand vanished".into())
            })?;
            args.push(frame.get(operand)?);
        }
        Ok(Some(Step::TailCall(callee, args)))
    }

    fn run_body(
        &self,
        function: OperationRef<'c, 'a>,
        args: &[Value],
    ) -> Result<CfgOutcome, EvalError> {
        let region = function
            .region(0)
            .map_err(|_| EvalError::Malformed("func.func without a region".into()))?;
        let entry = region.first_block().ok_or_else(|| {
            EvalError::Unsupported("body-less (external) function".into())
        })?;
        let mut frame = Frame::default();
        self.run_cfg(&mut frame, entry, args.to_vec())
    }

    /// Executes a multi-block CFG region (a function body) to its Return.
    pub fn run_cfg(
        &self,
        frame: &mut Frame,
        entry: BlockRef<'c, 'a>,
        mut incoming: Vec<Value>,
    ) -> Result<CfgOutcome, EvalError> {
        let mut block = entry;
        loop {
            bind_block_args(frame, block, &incoming)?;
            match self.exec_block(frame, block)? {
                Step::Return(values) => return Ok(CfgOutcome::Return(values)),
                Step::TailCall(name, args) => {
                    return Ok(CfgOutcome::TailCall(name, args));
                }
                Step::Branch(next, args) => {
                    block = next;
                    incoming = args;
                }
                Step::Yield(_) => {
                    return Err(EvalError::Malformed(
                        "yield escaped its structured region".into(),
                    ));
                }
                Step::Continue => {
                    return Err(EvalError::Malformed(
                        "exec_block leaked a Continue".into(),
                    ));
                }
            }
        }
    }

    /// Executes one single-block structured region (scf bodies) to its
    /// Yield. Multi-block structured regions are out of v0 scope.
    pub fn run_structured_block(
        &self,
        frame: &mut Frame,
        block: BlockRef<'c, 'a>,
        args: Vec<Value>,
    ) -> Result<Vec<Value>, EvalError> {
        bind_block_args(frame, block, &args)?;
        match self.exec_block(frame, block)? {
            Step::Yield(values) => Ok(values),
            Step::TailCall(..) => Err(EvalError::Malformed(
                "tail call escaped a structured region".into(),
            )),
            Step::Return(_) => Err(EvalError::Malformed(
                "return inside a structured region".into(),
            )),
            Step::Branch(..) => Err(EvalError::Unsupported(
                "multi-block structured region (v0)".into(),
            )),
            Step::Continue => Err(EvalError::Malformed(
                "exec_block leaked a Continue".into(),
            )),
        }
    }

    fn exec_block(
        &self,
        frame: &mut Frame,
        block: BlockRef<'c, 'a>,
    ) -> Result<Step<'c, 'a>, EvalError> {
        let mut next = block.first_operation();
        while let Some(op) = next {
            let name = op_name(op)?;
            // Tail-shape interception (D-059): a func.call whose
            // results feed the immediately following func.return runs
            // as a frame REPLACEMENT, not a recursion.
            if name == "func.call" {
                if let Some(step) = self.tail_shape(frame, op)? {
                    return Ok(step);
                }
            }
            let evaluator = self
                .registry
                .get(name.as_str())
                .ok_or(EvalError::UnknownOp(name))?;
            match evaluator.eval(self, frame, op)? {
                Step::Continue => next = op.next_in_block(),
                step => return Ok(step),
            }
        }
        Err(EvalError::Malformed(
            "block ended without a terminator".into(),
        ))
    }
}

fn bind_block_args(
    frame: &mut Frame,
    block: BlockRef<'_, '_>,
    values: &[Value],
) -> Result<(), EvalError> {
    if block.argument_count() != values.len() {
        return Err(EvalError::Malformed(format!(
            "block expects {} argument(s), got {}",
            block.argument_count(),
            values.len()
        )));
    }
    for (index, value) in values.iter().enumerate() {
        let argument = block
            .argument(index)
            .map_err(|_| EvalError::Malformed("block argument out of range".into()))?;
        frame.set(argument.into(), value.clone());
    }
    Ok(())
}

fn op_name(op: OperationRef<'_, '_>) -> Result<String, EvalError> {
    Ok(op
        .name()
        .as_string_ref()
        .as_str()
        .map_err(|_| EvalError::Malformed("non-UTF-8 op name".into()))?
        .to_string())
}
