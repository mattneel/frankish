//! femto_lua frontend (M11 bar 3): Lua 5.1 v0.1 subset per the
//! ratified MANIFEST (D-052), fences per D-054, kernel mapping per
//! D-056. lua5.1 is the oracle; output protocol is the print stream.

pub mod ast;
pub mod emit;
pub mod lex;

use melior::Context;
use melior::ir::Module;

pub fn compile_lua<'c>(
    context: &'c Context,
    file: &str,
    source: &str,
) -> Result<Module<'c>, String> {
    let chunk = ast::parse(source).map_err(|e| e.to_string())?;
    emit::emit(context, file, source, &chunk)
}

/// The D-084 forced-transform gate: the resumable-frame transform ON
/// for a module that never mentions coroutines.
pub fn compile_lua_forced<'c>(
    context: &'c Context,
    file: &str,
    source: &str,
) -> Result<Module<'c>, String> {
    let chunk = ast::parse(source).map_err(|e| e.to_string())?;
    emit::emit_with_license(context, file, source, &chunk, true)
}
