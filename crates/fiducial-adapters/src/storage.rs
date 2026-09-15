//! Storage contract — object storage: put, get, delete, list, signed URLs.
//!
//! Designed against the narrowest interface shared by R2, S3 and Supabase
//! Storage. Vendor-specific features (R2 multipart uploads, S3 versioning,
//! Supabase access policies) belong on the vendor type, not here.

use thiserror::Error;

use crate::BoxFuture;

/// Errors a [`Storage`] implementation may return.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("object not found: {key}")]
    NotFound { key: String },
    #[error("permission denied for key: {key}")]
    Forbidden { key: String },
    #[error("I/O error: {0}")]
    Io(String),
    #[error("{0}")]
    Other(String),
}

/// Object storage contract.
pub trait Storage: Send + Sync {
    /// Write bytes at `key`, replacing any existing object.
    fn put<'a>(
        &'a self,
        key: &'a str,
        bytes: &'a [u8],
        content_type: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>>;

    /// Read the object at `key`. Returns `None` when the key does not exist.
    fn get<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<Option<Vec<u8>>, StorageError>>;

    /// Delete the object at `key`. Succeeds even if the key does not exist.
    fn delete<'a>(&'a self, key: &'a str) -> BoxFuture<'a, Result<(), StorageError>>;

    /// List all keys with the given `prefix`. An empty prefix lists everything.
    fn list<'a>(&'a self, prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>>;

    /// Return a time-limited signed URL for the given key and `ttl_seconds`.
    ///
    /// The `None` implementation always returns an empty string — callers that
    /// need real signed URLs must select a vendor that implements them.
    fn signed_url<'a>(
        &'a self,
        key: &'a str,
        ttl_seconds: u64,
    ) -> BoxFuture<'a, Result<String, StorageError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op object storage — all writes succeed silently; all reads return `None`.
pub struct NoneStorage;

impl Storage for NoneStorage {
    fn put<'a>(
        &'a self,
        _key: &'a str,
        _bytes: &'a [u8],
        _content_type: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn get<'a>(&'a self, _key: &'a str) -> BoxFuture<'a, Result<Option<Vec<u8>>, StorageError>> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn delete<'a>(&'a self, _key: &'a str) -> BoxFuture<'a, Result<(), StorageError>> {
        Box::pin(std::future::ready(Ok(())))
    }

    fn list<'a>(&'a self, _prefix: &'a str) -> BoxFuture<'a, Result<Vec<String>, StorageError>> {
        Box::pin(std::future::ready(Ok(vec![])))
    }

    fn signed_url<'a>(
        &'a self,
        _key: &'a str,
        _ttl_seconds: u64,
    ) -> BoxFuture<'a, Result<String, StorageError>> {
        Box::pin(std::future::ready(Ok(String::new())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s() -> Box<dyn Storage> {
        Box::new(NoneStorage)
    }

    #[tokio::test]
    async fn put_succeeds_silently() {
        s().put("a/b.txt", b"hello", Some("text/plain"))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn get_returns_none() {
        let v = s().get("missing").await.unwrap();
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn delete_succeeds_on_missing_key() {
        s().delete("ghost").await.unwrap();
    }

    #[tokio::test]
    async fn list_returns_empty() {
        let keys = s().list("prefix/").await.unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn none_storage_is_object_safe() {
        let _: Box<dyn Storage> = Box::new(NoneStorage);
    }
}
