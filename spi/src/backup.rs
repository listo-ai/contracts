//! Backup & restore contract types — bundle envelope, manifest, and
//! portability classification.
//!
//! These types define the wire shape of `.listo-snapshot` and
//! `.listo-template` bundles. They are pure data — no I/O, no
//! orchestration. See `agent/docs/design/BACKUP.md` for the full
//! design.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Portability — per-slot classification
// ---------------------------------------------------------------------------

/// Controls whether a slot value travels across devices (templates),
/// stays device-local (snapshots only), or requires sealed encryption.
///
/// Declared per slot in `KindManifest` so a kind author sets
/// portability in the same place as the slot's type and role — one
/// source of truth. See BACKUP.md § 2.
///
/// Default is `Portable` — you opt *out* of travelling. If a kind
/// author forgets to classify, the slot goes into templates and breaks
/// in a loud, visible way (e.g. a credential shows up as plaintext in
/// review). The opposite default (secret by default) would silently
/// strip config and produce working-but-empty imports nobody notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Portability {
    /// Logical configuration. Exported to templates. Round-trips
    /// cleanly. Default for config slots (kind config, node wiring,
    /// static options).
    #[default]
    Portable,

    /// Local to this device. Excluded from templates. Included in
    /// snapshots. Stripped values are left null on template import;
    /// the kind's init hook regenerates them on first tick.
    Device,

    /// Credentials. Excluded from templates. Included in snapshots
    /// but only in the sealed section (age/X25519 encrypted at export
    /// time). Never in plaintext on disk outside a live process.
    Secret,

    /// Derived from other slots at runtime. Excluded from both
    /// templates and snapshots. Regenerated on first tick after
    /// restore.
    Derived,
}

impl Portability {
    /// Whether this slot is included in template exports.
    pub fn included_in_template(self) -> bool {
        matches!(self, Self::Portable)
    }

    /// Whether this slot is included in snapshot exports.
    pub fn included_in_snapshot(self) -> bool {
        !matches!(self, Self::Derived)
    }

    /// Whether this slot requires sealed-section encryption in the
    /// bundle.
    pub fn requires_sealing(self) -> bool {
        matches!(self, Self::Secret)
    }
}

// ---------------------------------------------------------------------------
// BundleKind — snapshot vs template
// ---------------------------------------------------------------------------

/// Discriminator for the two bundle types. Encoded in the manifest's
/// `bundle_kind` field. See BACKUP.md § 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleKind {
    /// Disaster recovery for *this* device. Everything needed to
    /// resurrect the exact agent state.
    Snapshot,
    /// Deploy or share logical configuration across *any* device.
    /// Portable fields only.
    Template,
}

impl BundleKind {
    /// File extension without the leading dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Snapshot => "listo-snapshot",
            Self::Template => "listo-template",
        }
    }
}

impl std::fmt::Display for BundleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Snapshot => f.write_str("snapshot"),
            Self::Template => f.write_str("template"),
        }
    }
}

// ---------------------------------------------------------------------------
// Bundle manifest — the small, signed JSON at the top of every bundle
// ---------------------------------------------------------------------------

/// Identity of the entity that produced the bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCreator {
    /// `device_id` of the producing agent. See BACKUP.md § 4.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    /// Human operator or automation identity, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// `<binary>@<version>` of the producing tool.
    pub tool: String,
}

/// Schema versions embedded in a snapshot manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersions {
    pub sqlite: u32,
    pub postgres: u32,
}

/// Pointers to DB dump files inside the payload tarball.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DumpPaths {
    pub sqlite: String,
    pub postgres: String,
}

/// Optional encryption metadata for sealed sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionMeta {
    /// e.g. `"age-x25519"`.
    pub scheme: String,
    /// Public keys of intended recipients.
    pub recipients: Vec<String>,
}

/// The small JSON file at the top of every `.listo-snapshot` or
/// `.listo-template` bundle. Covered by the ed25519 signature.
/// Diagnostic tools inspect it without unpacking; restore decisions
/// (version gate, kind availability, device match) are made against
/// it alone. See BACKUP.md § 1.3.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub bundle_kind: BundleKind,

    /// Envelope format version. Bump on structural changes to this
    /// struct.
    #[serde(default = "default_bundle_version")]
    pub bundle_version: u32,

    /// Distinguishes backup bundles from OTA bundles that share the
    /// same envelope. Always `"agent-state"` for backup.
    #[serde(default = "default_subject")]
    pub subject: String,

    /// UTC milliseconds since epoch.
    pub created_ms: u64,
    pub created_by: BundleCreator,

    // -- Snapshot-only fields -----------------------------------------------

    /// Required for snapshots — the producing device's identity.
    /// Restore checks `target.device_id == source_device_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_device_id: Option<String>,

    /// Advisory hostname. Never trusted for identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hostname: Option<String>,

    /// Exact agent version that produced the dump.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,

    /// DB schema versions at dump time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<SchemaVersions>,

    /// Paths to dump files inside `payload.tar.zst`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dumps: Option<DumpPaths>,

    // -- Template-only fields -----------------------------------------------

    /// `spi` major version — compatibility gate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spi_major: Option<u32>,

    /// Kinds the template requires on the target agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds_required: Vec<String>,

    /// Version of each required kind at export time.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub kind_versions: std::collections::BTreeMap<String, String>,

    /// Subtree root that was exported. `None` = whole graph.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,

    /// Number of nodes in the template payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_count: Option<u32>,

    /// Whether the template includes snippet definitions.
    #[serde(default)]
    pub contains_snippets: bool,

    // -- Shared fields -------------------------------------------------------

    /// SHA-256 of `payload.tar.zst`, hex-encoded.
    pub payload_sha256: String,

    /// Optional encryption for sealed sections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionMeta>,
}

fn default_bundle_version() -> u32 {
    1
}

fn default_subject() -> String {
    "agent-state".to_owned()
}

impl BundleManifest {
    /// Create a minimal snapshot manifest. Caller fills remaining
    /// fields via `with_*` builders.
    pub fn new_snapshot(
        device_id: String,
        agent_version: String,
        payload_sha256: String,
        created_ms: u64,
        tool: String,
    ) -> Self {
        Self {
            bundle_kind: BundleKind::Snapshot,
            bundle_version: 1,
            subject: "agent-state".to_owned(),
            created_ms,
            created_by: BundleCreator {
                device_id: Some(device_id.clone()),
                user: None,
                tool,
            },
            source_device_id: Some(device_id),
            source_hostname: None,
            agent_version: Some(agent_version),
            schema: None,
            dumps: None,
            spi_major: None,
            kinds_required: Vec::new(),
            kind_versions: std::collections::BTreeMap::new(),
            root_path: None,
            node_count: None,
            contains_snippets: false,
            payload_sha256,
            encryption: None,
        }
    }

    /// Create a minimal template manifest.
    pub fn new_template(
        spi_major: u32,
        payload_sha256: String,
        created_ms: u64,
        tool: String,
    ) -> Self {
        Self {
            bundle_kind: BundleKind::Template,
            bundle_version: 1,
            subject: "agent-state".to_owned(),
            created_ms,
            created_by: BundleCreator {
                device_id: None,
                user: None,
                tool,
            },
            source_device_id: None,
            source_hostname: None,
            agent_version: None,
            schema: None,
            dumps: None,
            spi_major: Some(spi_major),
            kinds_required: Vec::new(),
            kind_versions: std::collections::BTreeMap::new(),
            root_path: None,
            node_count: None,
            contains_snippets: false,
            payload_sha256,
            encryption: None,
        }
    }

    pub fn with_schema(mut self, schema: SchemaVersions) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_dumps(mut self, dumps: DumpPaths) -> Self {
        self.dumps = Some(dumps);
        self
    }

    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        self.source_hostname = Some(hostname.into());
        self
    }

    pub fn with_root_path(mut self, path: impl Into<String>) -> Self {
        self.root_path = Some(path.into());
        self
    }

    pub fn with_kinds(
        mut self,
        required: Vec<String>,
        versions: std::collections::BTreeMap<String, String>,
    ) -> Self {
        self.kinds_required = required;
        self.kind_versions = versions;
        self
    }

    pub fn with_node_count(mut self, count: u32) -> Self {
        self.node_count = Some(count);
        self
    }

    pub fn with_encryption(mut self, meta: EncryptionMeta) -> Self {
        self.encryption = Some(meta);
        self
    }

    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.created_by.user = Some(user.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portability_defaults_to_portable() {
        let p: Portability = serde_json::from_str("null").unwrap_or_default();
        assert_eq!(p, Portability::Portable);
    }

    #[test]
    fn portability_serde_roundtrip() {
        for variant in [
            Portability::Portable,
            Portability::Device,
            Portability::Secret,
            Portability::Derived,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: Portability = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn bundle_kind_extension() {
        assert_eq!(BundleKind::Snapshot.extension(), "listo-snapshot");
        assert_eq!(BundleKind::Template.extension(), "listo-template");
    }

    #[test]
    fn snapshot_manifest_serde_roundtrip() {
        let m = BundleManifest::new_snapshot(
            "dev_abc".into(),
            "0.42.1".into(),
            "deadbeef".into(),
            1_735_689_600_000,
            "agent@0.42.1".into(),
        )
        .with_hostname("edge-03");

        let json = serde_json::to_string_pretty(&m).unwrap();
        let back: BundleManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
        assert_eq!(back.bundle_kind, BundleKind::Snapshot);
        assert_eq!(back.source_device_id.as_deref(), Some("dev_abc"));
    }

    #[test]
    fn template_manifest_serde_roundtrip() {
        let m = BundleManifest::new_template(
            1,
            "cafef00d".into(),
            1_735_689_600_000,
            "agent@0.42.1".into(),
        )
        .with_root_path("flows/boiler-1")
        .with_node_count(312);

        let json = serde_json::to_string_pretty(&m).unwrap();
        let back: BundleManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
        assert_eq!(back.bundle_kind, BundleKind::Template);
        assert_eq!(back.spi_major, Some(1));
    }

    #[test]
    fn portability_inclusion_rules() {
        // Portable: both
        assert!(Portability::Portable.included_in_template());
        assert!(Portability::Portable.included_in_snapshot());
        // Device: snapshot only
        assert!(!Portability::Device.included_in_template());
        assert!(Portability::Device.included_in_snapshot());
        // Secret: snapshot only (sealed)
        assert!(!Portability::Secret.included_in_template());
        assert!(Portability::Secret.included_in_snapshot());
        assert!(Portability::Secret.requires_sealing());
        // Derived: neither
        assert!(!Portability::Derived.included_in_template());
        assert!(!Portability::Derived.included_in_snapshot());
    }
}
