//! r7rs_core frontend (M15, D-060): the R7RS-small core sublanguage
//! per the ratified MANIFEST. chibi-scheme is the oracle; the specimen
//! exists to force frk.ctl (call/ec + error → prompt/abort) and to
//! make proper tail calls (M14) load-bearing corpus-wide.
//!
//! Layers: [`reader`] (source → s-expression `Datum`s, design-
//! independent) → ast (core forms) → emit (kernel dialects). The
//! reader lands first and standalone.

pub mod ast;
pub mod emit;
pub mod reader;

use melior::Context;
use melior::ir::Module;

pub fn compile_scheme<'c>(
    context: &'c Context,
    file: &str,
    source: &str,
) -> Result<Module<'c>, String> {
    let program = ast::parse(source)?;
    emit::emit(context, file, source, &program)
}
