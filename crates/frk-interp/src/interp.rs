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
        self.slots.get(&slot_key(value)).copied().ok_or_else(|| {
            EvalError::Malformed(format!("use of unbound SSA value: {value}"))
        })
    }

    pub fn set(&mut self, target: IrValue, value: Value) {
        self.slots.insert(slot_key(target), value);
    }
}

pub struct Interp<'c, 'a> {
    registry: HashMap<&'static str, Box<dyn Eval>>,
    functions: HashMap<String, OperationRef<'c, 'a>>,
    depth: Cell<usize>,
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
            depth: Cell::new(0),
        })
    }

    /// Calls `name(args)` and returns its results. This is both the public
    /// entry and the path `func.call` re-enters.
    pub fn eval_function(&self, name: &str, args: &[Value]) -> Result<Vec<Value>, EvalError> {
        let function = *self
            .functions
            .get(name)
            .ok_or_else(|| EvalError::CalleeNotFound(name.to_string()))?;

        if self.depth.get() >= MAX_CALL_DEPTH {
            return Err(EvalError::Trap(format!(
                "call depth exceeded {MAX_CALL_DEPTH} frames (D-029)"
            )));
        }
        self.depth.set(self.depth.get() + 1);
        let result = self.run_body(function, args);
        self.depth.set(self.depth.get() - 1);
        result
    }

    fn run_body(
        &self,
        function: OperationRef<'c, 'a>,
        args: &[Value],
    ) -> Result<Vec<Value>, EvalError> {
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
    ) -> Result<Vec<Value>, EvalError> {
        let mut block = entry;
        loop {
            bind_block_args(frame, block, &incoming)?;
            match self.exec_block(frame, block)? {
                Step::Return(values) => return Ok(values),
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
        frame.set(argument.into(), *value);
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
