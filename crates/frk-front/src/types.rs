//! The type kit's type language (SPEC §6.4). Unification rides ena
//! (D-005 bill of materials); `TyVid` keys the table, unresolved
//! variables carry `None`.

use ena::unify::{EqUnifyValue, UnifyKey};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TyVid(pub u32);

impl UnifyKey for TyVid {
    type Value = Option<Ty>;
    fn index(&self) -> u32 {
        self.0
    }
    fn from_index(index: u32) -> Self {
        Self(index)
    }
    fn tag() -> &'static str {
        "TyVid"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    Unit,
    Bool,
    Int,
    Tuple(Vec<Ty>),
    /// Nominal reference to a declared (non-recursive, v0.1) ADT.
    Adt(String),
    Fun(Box<Ty>, Box<Ty>),
    Var(TyVid),
}

impl EqUnifyValue for Ty {}

impl std::fmt::Display for Ty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Bool => write!(f, "bool"),
            Self::Int => write!(f, "int"),
            Self::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", parts.join(" * "))
            }
            Self::Adt(name) => write!(f, "{name}"),
            Self::Fun(a, b) => write!(f, "({a} -> {b})"),
            Self::Var(vid) => write!(f, "'t{}", vid.0),
        }
    }
}
