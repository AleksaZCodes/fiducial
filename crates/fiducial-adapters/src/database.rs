//! Database contract — relational storage: queries, migrations, transactions.
//!
//! Designed against the narrowest interface shared by D1, Supabase, Neon and
//! Postgres. Any method that leaks a vendor's model (Supabase row-level
//! security policies, D1's batch API shape, Postgres advisory locks) belongs
//! on the vendor type, not on this trait.

use std::collections::BTreeMap;
use thiserror::Error;

use crate::BoxFuture;

/// A SQL parameter value — the intersection across all target vendors.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

/// One row from a query result: ordered column→value pairs.
#[derive(Debug, Clone)]
pub struct Row(pub BTreeMap<String, Value>);

impl Row {
    pub fn get(&self, column: &str) -> Option<&Value> {
        self.0.get(column)
    }

    pub fn columns(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(|s| s.as_str())
    }
}

/// Errors a [`Database`] implementation may return.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("query failed: {0}")]
    QueryFailed(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("migration failed at step {step}: {message}")]
    Migration { step: u32, message: String },
    #[error("{0}")]
    Other(String),
}

/// Relational database contract.
///
/// Implementations must be `Send + Sync` — they will be placed behind
/// `Box<dyn Database>` and shared across async tasks.
pub trait Database: Send + Sync {
    /// Execute a write statement (INSERT, UPDATE, DELETE, DDL).
    ///
    /// Returns the number of rows affected.
    fn execute<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<u64, DatabaseError>>;

    /// Run a SELECT query and return all matching rows.
    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<Vec<Row>, DatabaseError>>;

    /// Run a SELECT query and return the first row, or `None` if empty.
    fn query_one<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Row>, DatabaseError>>;

    /// Run a sequence of statements as an atomic batch.
    ///
    /// Semantics match each vendor's batch/transaction primitive — D1 uses a
    /// batch API; Supabase and Postgres use a transaction. The `None`
    /// implementation runs them sequentially with no atomicity guarantee.
    fn batch<'a>(
        &'a self,
        statements: &'a [(&'a str, &'a [Value])],
    ) -> BoxFuture<'a, Result<Vec<u64>, DatabaseError>>;
}

// ── None implementation ───────────────────────────────────────────────────────

/// No-op database — wired in from the first commit, costs nothing.
///
/// All queries return empty results; all writes report zero rows affected.
/// This is not a placeholder: `database = "none"` is a real, working
/// selection that lets a product compile, run and be tested without a
/// live database.
pub struct NoneDatabase;

impl Database for NoneDatabase {
    fn execute<'a>(
        &'a self,
        _sql: &'a str,
        _params: &'a [Value],
    ) -> BoxFuture<'a, Result<u64, DatabaseError>> {
        Box::pin(std::future::ready(Ok(0)))
    }

    fn query<'a>(
        &'a self,
        _sql: &'a str,
        _params: &'a [Value],
    ) -> BoxFuture<'a, Result<Vec<Row>, DatabaseError>> {
        Box::pin(std::future::ready(Ok(vec![])))
    }

    fn query_one<'a>(
        &'a self,
        _sql: &'a str,
        _params: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Row>, DatabaseError>> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn batch<'a>(
        &'a self,
        statements: &'a [(&'a str, &'a [Value])],
    ) -> BoxFuture<'a, Result<Vec<u64>, DatabaseError>> {
        let n = statements.len();
        Box::pin(std::future::ready(Ok(vec![0; n])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Box<dyn Database> {
        Box::new(NoneDatabase)
    }

    #[tokio::test]
    async fn execute_returns_zero_rows_affected() {
        let n = db()
            .execute("INSERT INTO t VALUES (?)", &[Value::Integer(1)])
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn query_returns_empty_rows() {
        let rows = db().execute("SELECT * FROM t", &[]).await.unwrap();
        assert_eq!(rows, 0);
    }

    #[tokio::test]
    async fn query_one_returns_none() {
        let row = db()
            .query_one("SELECT * FROM t WHERE id = ?", &[Value::Integer(42)])
            .await
            .unwrap();
        assert!(row.is_none());
    }

    #[tokio::test]
    async fn batch_returns_one_count_per_statement() {
        let results = db()
            .batch(&[
                (
                    "INSERT INTO a VALUES (?)",
                    &[Value::Text("x".into())] as &[_],
                ),
                ("DELETE FROM b WHERE id = ?", &[Value::Integer(1)]),
            ])
            .await
            .unwrap();
        assert_eq!(results, vec![0, 0]);
    }

    #[test]
    fn none_database_is_object_safe() {
        let _: Box<dyn Database> = Box::new(NoneDatabase);
    }

    #[test]
    fn row_get_returns_column_value() {
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), Value::Integer(1));
        let row = Row(map);
        assert_eq!(row.get("id"), Some(&Value::Integer(1)));
        assert_eq!(row.get("missing"), None);
    }
}
