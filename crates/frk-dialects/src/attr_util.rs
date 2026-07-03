//! Shared attribute/type decoding helpers for kernel dialect modules.

use melior::Context;
use melior::ir::attribute::AttributeLike;
use melior::ir::{Attribute, Type};

/// Walks a builtin ArrayAttr through the C API. melior 0.27.2's
/// `ArrayAttribute::try_from` is miswired to `is_dense_i64_array`
/// (melior src/ir/attribute/array.rs:54) and so rejects every genuine
/// array — pinned in docs/LANDSCAPE.md. Delete this shim when the fix
/// lands upstream.
pub(crate) fn array_elements<'c>(
    attribute: Attribute<'c>,
) -> Result<Vec<Attribute<'c>>, Attribute<'c>> {
    if !attribute.is_array() {
        return Err(attribute);
    }
    let raw = attribute.to_raw();
    let count = unsafe { mlir_sys::mlirArrayAttrGetNumElements(raw) };
    Ok((0..count)
        .map(|index| unsafe {
            Attribute::from_raw(mlir_sys::mlirArrayAttrGetElement(raw, index))
        })
        .collect())
}

/// Slices a dynamic type's parameter text out of its printed form and
/// re-parses it as an attribute: dynamic types expose no parameter
/// accessor through the C API, so the printer's canonical output is the
/// stable surface (D-031 world). `wrap` adds enclosing brackets for
/// multi-parameter types (whose params print comma-separated).
pub(crate) fn type_params<'c>(
    context: &'c Context,
    r#type: Type<'c>,
    prefix: &str,
    wrap: bool,
) -> Result<Attribute<'c>, String> {
    let printed = r#type.to_string();
    let inner = printed
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix('>'))
        .ok_or_else(|| format!("expected {prefix}...>, got {printed}"))?;
    let source = if wrap { format!("[{inner}]") } else { inner.to_string() };
    Attribute::parse(context, &source)
        .ok_or_else(|| format!("unparsable type parameters: {inner}"))
}

/// Reads a string attribute's bytes via its PRINTED form. melior
/// 0.27.2's StringAttribute::value() calls slice::from_raw_parts on
/// the raw StringRef, which is NULL for the empty string — UB caught
/// by the runtime check (LANDSCAPE pin). The printed form
/// (`"..."` with \HH escapes for quotes/backslashes/non-printables)
/// is always safe to read.
pub(crate) fn string_attr_bytes(attribute: melior::ir::Attribute<'_>) -> Result<Vec<u8>, String> {
    let printed = attribute.to_string();
    let inner = printed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| format!("not a string attribute: {printed}"))?;
    let bytes = inner.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            let a = bytes.get(index + 1).copied().ok_or("dangling escape")?;
            // MLIR prints \\ for backslash and \HH hex otherwise.
            if a == b'\\' {
                out.push(b'\\');
                index += 2;
            } else {
                let b = bytes.get(index + 2).copied().ok_or("short hex escape")?;
                let high = (a as char).to_digit(16).ok_or("bad hex escape")?;
                let low = (b as char).to_digit(16).ok_or("bad hex escape")?;
                out.push((high * 16 + low) as u8);
                index += 3;
            }
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    Ok(out)
}
