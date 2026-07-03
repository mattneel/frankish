//! frk.adt — sums, products, tuples as parametric `!frk_adt` types with
//! pure value ops (SPEC §4.1 as amended by D-031). The dialect namespace
//! is `frk_adt`; SPEC prose writes "frk.adt" for the same thing.
//!
//! Ops:
//! - `make_sum(fields...) {variant} -> !frk_adt.sum<...>`
//! - `tag_of(sum) -> i64`
//! - `extract(sum) {variant, field} -> <field type>`
//! - `make_product(fields...) -> !frk_adt.product<...>`
//! - `get(product) {field} -> <field type>`
//!
//! Type parameter encoding: a sum's single parameter is an array of
//! variants, each an array of field types — `Option<i64>` is
//! `!frk_adt.sum<[[], [i64]]>`; a product's parameter is its field-type
//! array — `!frk_adt.product<[i64, i64]>`. Tuples ARE products.
//!
//! There is deliberately no `match` op (D-031): dispatch compiles to
//! `tag_of` + upstream `cf.switch` + per-arm `extract`, produced by the
//! decision-tree pass from the frontend's pattern matrix.
//!
//! IRDL enforces shape here — operand/result base types (sum vs product
//! vs anything), attribute kinds, arity, i64 tags. What IRDL cannot say
//! (variant/field indices in range, extract's result = the named field's
//! type, make_sum operand types = the variant's shape) is the frk
//! verification pass's job (K1 second half; runs before execution or
//! lowering).
//!
//! IRDL landmines, learned against mlir-opt 22.1.8:
//! - A reused constraint variable unifies *values*: one `%idx` shared by
//!   `variant` and `field` would demand variant == field. Every
//!   independently-valued attribute gets its own variable.
//! - `irdl.is i64` means "the attribute equals the type i64";
//!   `irdl.base "#builtin.integer"` means "any integer attribute".
//! - `irdl.parametric`/`irdl.base` symbol refs must be fully nested
//!   (`@frk_adt::@sum`).

use melior::Context;
use melior::ir::attribute::{AttributeLike, IntegerAttribute, TypeAttribute};
use melior::ir::operation::OperationLike;
use melior::ir::{Attribute, OperationRef, Type, ValueLike};

/// The dialect definition, loaded at registration time (D-031) by
/// [`crate::register`]. (Raw string uses ##: the IRDL text itself
/// contains `"#` in `irdl.base "#builtin.integer"`.)
pub const IRDL: &str = r##"
irdl.dialect @frk_adt {
  irdl.type @sum {
    %variants = irdl.any
    irdl.parameters(variants: %variants)
  }
  irdl.type @product {
    %fields = irdl.any
    irdl.parameters(fields: %fields)
  }
  irdl.operation @make_sum {
    %any = irdl.any
    %sum = irdl.base @frk_adt::@sum
    %idx = irdl.base "#builtin.integer"
    irdl.operands(fields: variadic %any)
    irdl.results(sum: %sum)
    irdl.attributes { "variant" = %idx }
  }
  irdl.operation @tag_of {
    %sum = irdl.base @frk_adt::@sum
    %tag = irdl.is i64
    irdl.operands(sum: %sum)
    irdl.results(tag: %tag)
  }
  irdl.operation @extract {
    %sum = irdl.base @frk_adt::@sum
    %any = irdl.any
    %vidx = irdl.base "#builtin.integer"
    %fidx = irdl.base "#builtin.integer"
    irdl.operands(sum: %sum)
    irdl.results(value: %any)
    irdl.attributes { "variant" = %vidx, "field" = %fidx }
  }
  irdl.operation @make_product {
    %any = irdl.any
    %prod = irdl.base @frk_adt::@product
    irdl.operands(fields: variadic %any)
    irdl.results(product: %prod)
  }
  irdl.operation @get {
    %prod = irdl.base @frk_adt::@product
    %any = irdl.any
    %idx = irdl.base "#builtin.integer"
    irdl.operands(product: %prod)
    irdl.results(value: %any)
    irdl.attributes { "field" = %idx }
  }
}
"##;

// ---- semantic verification (K1 second half; driven by crate::verify) ----

/// Checks one `frk_adt.<name>` op's semantic invariants — everything the
/// IRDL constraints above cannot say. One message per offending op.
pub(crate) fn verify_op<'c>(
    context: &'c Context,
    name: &str,
    op: OperationRef<'c, '_>,
) -> Result<(), String> {
    match name {
        "make_sum" => {
            let variants = decode_sum(context, result_type(op)?)?;
            let variant = index_attr(op, "variant")?;
            let fields = variants.get(variant).ok_or_else(|| {
                format!(
                    "variant {variant} out of range: the sum has {} variant(s)",
                    variants.len()
                )
            })?;
            expect_field_types(op, fields)
        }
        "extract" => {
            let variants = decode_sum(context, operand_type(op, 0)?)?;
            let variant = index_attr(op, "variant")?;
            let field = index_attr(op, "field")?;
            let fields = variants.get(variant).ok_or_else(|| {
                format!(
                    "variant {variant} out of range: the sum has {} variant(s)",
                    variants.len()
                )
            })?;
            let field_type = fields.get(field).ok_or_else(|| {
                format!(
                    "field {field} out of range: variant {variant} has {} field(s)",
                    fields.len()
                )
            })?;
            let result = result_type(op)?;
            if result == *field_type {
                Ok(())
            } else {
                Err(format!(
                    "extract result type {result} != variant {variant} field {field} type {field_type}"
                ))
            }
        }
        "make_product" => {
            let fields = decode_product(context, result_type(op)?)?;
            expect_field_types(op, &fields)
        }
        "get" => {
            let fields = decode_product(context, operand_type(op, 0)?)?;
            let field = index_attr(op, "field")?;
            let field_type = fields.get(field).ok_or_else(|| {
                format!(
                    "field {field} out of range: the product has {} field(s)",
                    fields.len()
                )
            })?;
            let result = result_type(op)?;
            if result == *field_type {
                Ok(())
            } else {
                Err(format!(
                    "get result type {result} != field {field} type {field_type}"
                ))
            }
        }
        // Fully covered by the IRDL constraints.
        "tag_of" => Ok(()),
        other => Err(format!("no semantic verifier for frk_adt.{other}")),
    }
}

/// Decodes `!frk_adt.sum<[[t...], ...]>` into per-variant field types.
/// Dynamic types expose no parameter accessor through the C API, so this
/// goes print → slice the angle brackets → Attribute::parse; the
/// printer's canonical output is the stable surface here.
pub(crate) fn decode_sum<'c>(
    context: &'c Context,
    r#type: Type<'c>,
) -> Result<Vec<Vec<Type<'c>>>, String> {
    let params = type_params(context, r#type, "!frk_adt.sum<")?;
    let variants = array_elements(params).map_err(|attribute| {
        format!("sum parameter must be an array of variants, got {attribute}")
    })?;
    variants
        .iter()
        .enumerate()
        .map(|(index, fields)| decode_field_list(*fields, &format!("variant {index}")))
        .collect()
}

/// Decodes `!frk_adt.product<[t...]>` into its field types.
pub(crate) fn decode_product<'c>(
    context: &'c Context,
    r#type: Type<'c>,
) -> Result<Vec<Type<'c>>, String> {
    let fields = type_params(context, r#type, "!frk_adt.product<")?;
    decode_field_list(fields, "product")
}

fn decode_field_list<'c>(
    fields: Attribute<'c>,
    what: &str,
) -> Result<Vec<Type<'c>>, String> {
    let fields = array_elements(fields).map_err(|attribute| {
        format!("{what} must be an array of field types, got {attribute}")
    })?;
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            TypeAttribute::try_from(*field)
                .map(|attribute| attribute.value())
                .map_err(|_| format!("{what} field {index} must be a type, got {field}"))
        })
        .collect()
}

/// Walks a builtin ArrayAttr through the C API. melior 0.27.2's
/// `ArrayAttribute::try_from` is miswired to `is_dense_i64_array`
/// (melior src/ir/attribute/array.rs:54) and so rejects every genuine
/// array — pinned in docs/LANDSCAPE.md. Delete this shim when the fix
/// lands upstream.
fn array_elements<'c>(attribute: Attribute<'c>) -> Result<Vec<Attribute<'c>>, Attribute<'c>> {
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

fn type_params<'c>(
    context: &'c Context,
    r#type: Type<'c>,
    prefix: &str,
) -> Result<Attribute<'c>, String> {
    let printed = r#type.to_string();
    let inner = printed
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_suffix('>'))
        .ok_or_else(|| format!("expected {prefix}...>, got {printed}"))?;
    Attribute::parse(context, inner)
        .ok_or_else(|| format!("unparsable type parameters: {inner}"))
}

pub(crate) fn index_attr(op: OperationRef<'_, '_>, name: &str) -> Result<usize, String> {
    let value = op
        .attribute(name)
        .ok()
        .and_then(|attribute| IntegerAttribute::try_from(attribute).ok())
        .ok_or_else(|| format!("missing integer attribute {name:?}"))?
        .value();
    usize::try_from(value).map_err(|_| format!("attribute {name:?} is negative: {value}"))
}

fn result_type<'c>(op: OperationRef<'c, '_>) -> Result<Type<'c>, String> {
    Ok(op
        .result(0)
        .map_err(|_| "op has no result".to_string())?
        .r#type())
}

fn operand_type<'c>(op: OperationRef<'c, '_>, index: usize) -> Result<Type<'c>, String> {
    Ok(op
        .operand(index)
        .map_err(|_| format!("op has no operand {index}"))?
        .r#type())
}

fn expect_field_types(op: OperationRef<'_, '_>, fields: &[Type<'_>]) -> Result<(), String> {
    if op.operand_count() != fields.len() {
        return Err(format!(
            "{} operand(s) for {} field(s)",
            op.operand_count(),
            fields.len()
        ));
    }
    for (index, field_type) in fields.iter().enumerate() {
        let actual = operand_type(op, index)?;
        if actual != *field_type {
            return Err(format!(
                "operand {index} has type {actual}, field expects {field_type}"
            ));
        }
    }
    Ok(())
}
