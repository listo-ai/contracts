//! Declarative, orthogonal classification flags on a kind.
//!
//! A single kind may carry many facets (`{isProtocol, isDriver,
//! isContainer}`). Facets drive palette grouping, placement predicates,
//! RBAC scoping, and generic RSQL queries. See
//! `docs/design/EVERYTHING-AS-NODE.md` § "Facets".

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Facet {
    IsProtocol,
    IsDriver,
    IsDevice,
    IsPoint,
    IsCompute,
    IsContainer,
    IsSystem,
    IsIdentity,
    IsEphemeral,
    IsWritable,
    /// Flow document container (`sys.core.flow`).
    IsFlow,
    /// I/O surface (webhooks, HTTP clients, queues).
    IsIO,
    /// Placement-agnostic utility node — the engine skips the parent's
    /// `may_contain` whitelist for nodes carrying this facet. Use for
    /// system/utility kinds (timers, heartbeats, annotations) that must
    /// be placeable anywhere in the graph without requiring every
    /// container manifest to explicitly opt them in.
    IsAnywhere,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FacetSet(BTreeSet<Facet>);

impl FacetSet {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn of<const N: usize>(facets: [Facet; N]) -> Self {
        Self(facets.into_iter().collect())
    }

    pub fn contains(&self, facet: Facet) -> bool {
        self.0.contains(&facet)
    }

    pub fn insert(&mut self, facet: Facet) {
        self.0.insert(facet);
    }

    pub fn iter(&self) -> impl Iterator<Item = Facet> + '_ {
        self.0.iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<const N: usize> From<[Facet; N]> for FacetSet {
    fn from(a: [Facet; N]) -> Self {
        Self::of(a)
    }
}
