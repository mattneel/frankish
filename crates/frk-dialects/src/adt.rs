//! frk.adt — sums, products, tuples as parametric `!frk_adt` types with
//! pure value ops (SPEC §4.1 as amended by D-031). The dialect namespace
//! is `frk_adt`; SPEC prose writes "frk.adt" for the same thing.
//!
//! Ops (packed surface, D-036 — no variadics: IRDL unifies every
//! element of a variadic group to one type, so heterogeneous payloads
//! flow through explicit product chains):
//! - `product_new() -> !frk_adt.product<[]>`
//! - `product_snoc(product, value) -> product-with-one-more-field`
//! - `make_sum(payload product) {variant} -> !frk_adt.sum<...>`
//! - `tag_of(sum) -> i64`
//! - `extract(sum) {variant, field} -> <field type>`
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
use melior::ir::attribute::{IntegerAttribute, TypeAttribute};
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
  irdl.operation @product_new {
    %prod = irdl.base @frk_adt::@product
    irdl.results(product: %prod)
  }
  irdl.operation @product_snoc {
    %prod_in = irdl.base @frk_adt::@product
    %any = irdl.any
    %prod_out = irdl.base @frk_adt::@product
    irdl.operands(product: %prod_in, value: %any)
    irdl.results(grown: %prod_out)
  }
  irdl.operation @make_sum {
    %payload = irdl.base @frk_adt::@product
    %sum = irdl.base @frk_adt::@sum
    %idx = irdl.base "#builtin.integer"
    irdl.operands(payload: %payload)
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
        "product_new" => {
            let fields = decode_product(context, result_type(op)?)?;
            if fields.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "product_new must yield an empty product, result declares {} field(s)",
                    fields.len()
                ))
            }
        }
        "product_snoc" => {
            let old_fields = decode_product(context, operand_type(op, 0)?)?;
            let appended = operand_type(op, 1)?;
            let new_fields = decode_product(context, result_type(op)?)?;
            if new_fields.len() != old_fields.len() + 1 {
                return Err(format!(
                    "snoc result declares {} field(s); operand has {} + 1 appended",
                    new_fields.len(),
                    old_fields.len()
                ));
            }
            for (index, old) in old_fields.iter().enumerate() {
                if new_fields[index] != *old {
                    return Err(format!(
                        "snoc result field {index} is {}, operand field {index} is {old}",
                        new_fields[index]
                    ));
                }
            }
            let last = new_fields[old_fields.len()];
            if last != appended {
                return Err(format!(
                    "snoc appends a {appended}, result declares {last}"
                ));
            }
            Ok(())
        }
        "make_sum" => {
            let variants = decode_sum(context, result_type(op)?)?;
            let variant = index_attr(op, "variant")?;
            let fields = variants.get(variant).ok_or_else(|| {
                format!(
                    "variant {variant} out of range: the sum has {} variant(s)",
                    variants.len()
                )
            })?;
            let payload = decode_product(context, operand_type(op, 0)?)?;
            if payload.len() != fields.len() {
                return Err(format!(
                    "payload has {} field(s), variant {variant} needs {}",
                    payload.len(),
                    fields.len()
                ));
            }
            for (index, field) in fields.iter().enumerate() {
                if payload[index] != *field {
                    return Err(format!(
                        "payload field {index} is {}, variant {variant} field {index} is {field}",
                        payload[index]
                    ));
                }
            }
            Ok(())
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
    let params = crate::attr_util::type_params(context, r#type, "!frk_adt.sum<", false)?;
    let variants = crate::attr_util::array_elements(params).map_err(|attribute| {
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
    let fields = crate::attr_util::type_params(context, r#type, "!frk_adt.product<", false)?;
    decode_field_list(fields, "product")
}

pub(crate) fn decode_field_list<'c>(
    fields: Attribute<'c>,
    what: &str,
) -> Result<Vec<Type<'c>>, String> {
    let fields = crate::attr_util::array_elements(fields).map_err(|attribute| {
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

