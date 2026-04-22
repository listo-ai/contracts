//! Artifact store contract — the trait every backend (S3, local FS,
//! Azure Blob, GCS) implements and every caller (block installer,
//! backup uploader, template distributor) depends on.
//!
//! See `agent/docs/design/ARTIFACTS.md`. This module owns the *shape*
//! only: types and the trait. The backend crates
//! (`data-artifacts-s3`, `data-artifacts-local`, …) each provide a
//! concrete `ArtifactStore` behind a Cargo feature.
//!
//! STATUS: scaffolding — method bodies to be filled in follow-up PRs.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::auth::TenantId;

/// Stable key under a tenant bucket. Constructed via the `keys` module
/// helpers — callers must not hand-format strings.
///
/// Shape: `<prefix>/<...>` where prefix is one of `blocks`,
/// `snapshots`, `templates`, `firmware`, `docs`. See ARTIFACTS.md § 3.
pub type ArtifactKey = String;

/// Content-hash + size asserted by the publisher. Verified by the
/// consumer against the hash carried in the authenticated control
/// message, not blindly trusted from the store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Integrity {
    pub sha256: [u8; 32],
    pub size: u64,
}

/// Direction of a presigned URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresignDirection {
    /// Client will upload to this URL (PUT / POST multipart).
    Put,
    /// Client will download from this URL (GET with ranges).
    Get,
}

/// Result of a presign call. `url` is opaque to the caller; the caller
/// hands it to whichever party needs to move the bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrl {
    pub url: String,
    pub expires_at_ms: u64,
    pub direction: PresignDirection,
}

/// Structured artifact-store failure. Serialisable so cross-process
/// callers see the same shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ArtifactError {
    /// Subsystem compiled out or `artifacts: null` in config.
    #[error("artifact store disabled")]
    Disabled,
    /// Key not present in the store.
    #[error("not found: {key}")]
    NotFound { key: String },
    /// Tenant / scope check failed before touching the store.
    #[error("forbidden: {reason}")]
    Forbidden { reason: String },
    /// Presign URL expired before consumer used it. Not an error in
    /// Flow A: edges re-request a fresh URL via `presign-download`
    /// rather than treating this as a failure.
    #[error("expired")]
    Expired,
    /// Hash mismatch on download. Hard reject — the manifest signature
    /// covers the hash, so mismatch is semantically a signature
    /// failure. No retry.
    #[error("integrity mismatch on {key}")]
    IntegrityMismatch { key: String },
    /// Local cache ran out of room and eviction couldn't free enough.
    #[error("cache full")]
    CacheFull,
    /// Backend reported the error verbatim. Opaque; log-and-fail.
    #[error("backend: {0}")]
    Backend(String),
}

/// Byte-stream type alias; concrete stream is backend-specific.
///
/// Kept as an alias so the trait stays object-safe. Will likely become
/// a wrapper around `futures::Stream<Item = Result<Bytes, _>>` once
/// real implementations land.
pub type ByteStream = std::pin::Pin<
    Box<dyn futures_core::Stream<Item = Result<bytes::Bytes, ArtifactError>> + Send + 'static>,
>;

/// The contract. Object-safe so `AppState` can hold
/// `Arc<dyn ArtifactStore>`.
///
/// Backends must be safe to call concurrently — the agent's domain
/// code will issue parallel fetches during block-install sweeps and
/// parallel puts during snapshot multipart uploads.
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    /// Stream bytes into storage. Multipart-aware; backends handle
    /// chunking internally.
    async fn put(&self, key: &ArtifactKey, bytes: ByteStream) -> Result<(), ArtifactError>;

    /// Stream bytes out. Caller verifies integrity against the hash in
    /// the authenticated control message — the store is not a trusted
    /// integrity oracle.
    async fn get(&self, key: &ArtifactKey) -> Result<ByteStream, ArtifactError>;

    /// Cheap existence check — HEAD request, no body.
    async fn head(&self, key: &ArtifactKey) -> Result<Option<Integrity>, ArtifactError>;

    /// Mint a time-limited URL scoped to one object. TTL is advisory;
    /// backends clamp to their own maximum.
    async fn presign(
        &self,
        key: &ArtifactKey,
        direction: PresignDirection,
        ttl: Duration,
    ) -> Result<PresignedUrl, ArtifactError>;

    /// Stable backend id — surfaces in capabilities as
    /// `artifacts.<id>.v1`.
    fn id(&self) -> &'static str;
}

/// Zero-config impl for deployments with the subsystem disabled at
/// runtime (`artifacts: null`) or compiled out entirely.
///
/// Every method returns [`ArtifactError::Disabled`]. `AppState` holds
/// one of these by default; enabling a real backend is a config +
/// feature-flag change.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullArtifactStore;

#[async_trait]
impl ArtifactStore for NullArtifactStore {
    async fn put(&self, _key: &ArtifactKey, _bytes: ByteStream) -> Result<(), ArtifactError> {
        Err(ArtifactError::Disabled)
    }

    async fn get(&self, _key: &ArtifactKey) -> Result<ByteStream, ArtifactError> {
        Err(ArtifactError::Disabled)
    }

    async fn head(&self, _key: &ArtifactKey) -> Result<Option<Integrity>, ArtifactError> {
        Err(ArtifactError::Disabled)
    }

    async fn presign(
        &self,
        _key: &ArtifactKey,
        _direction: PresignDirection,
        _ttl: Duration,
    ) -> Result<PresignedUrl, ArtifactError> {
        Err(ArtifactError::Disabled)
    }

    fn id(&self) -> &'static str {
        "null"
    }
}

/// Typed key constructors. One source of truth for the layout in
/// ARTIFACTS.md § 3 — callers never hand-format prefix strings.
pub mod keys {
    use super::ArtifactKey;
    use crate::auth::TenantId;

    /// `blocks/<block_id>/<version>/bundle.tar.zst`
    pub fn block_bundle(_tenant: &TenantId, _block_id: &str, _version: &str) -> ArtifactKey {
        todo!("scaffolding — fill in once contract is ratified")
    }

    /// `blocks/<block_id>/<version>/manifest.json`
    pub fn block_manifest(_tenant: &TenantId, _block_id: &str, _version: &str) -> ArtifactKey {
        todo!()
    }

    /// `snapshots/<agent_id>/<ts>.listo-snapshot`
    pub fn snapshot(_tenant: &TenantId, _agent_id: &str, _ts_ms: u64) -> ArtifactKey {
        todo!()
    }

    /// `templates/<template_id>/<version>.listo-template`
    pub fn template(_tenant: &TenantId, _template_id: &str, _version: &str) -> ArtifactKey {
        todo!()
    }

    /// `firmware/<channel>/<version>/listod.listo`
    pub fn firmware(_tenant: &TenantId, _channel: &str, _version: &str) -> ArtifactKey {
        todo!()
    }

    /// `docs/<user_id>/<doc_id>/<rev>`
    pub fn doc(
        _tenant: &TenantId,
        _user_id: &str,
        _doc_id: &str,
        _rev: u64,
    ) -> ArtifactKey {
        todo!()
    }
}

/// Bucket name derivation from tenant. `listo-<tenant>`.
///
/// Isolated as a free fn so the presigner, provisioner, and clients
/// agree byte-for-byte.
pub fn bucket_for(_tenant: &TenantId) -> String {
    todo!("scaffolding — fill in once contract is ratified")
}
