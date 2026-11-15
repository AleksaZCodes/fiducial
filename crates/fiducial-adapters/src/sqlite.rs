//! `database = "sqlite"` — the [`Database`] contract on a local SQLite file.
//!
//! The self-hosted target's database: one file on the host, WAL mode, no server
//! and no account. It uses `sqlx` (`docs/specs/2026-11-01-a-self-hosted-target.md`),
//! and maps to the same SQL dialect `d1` already uses, so a migration written
//! for one runs on the other.
//!
//! This is the first Rust-side consumer the contract has had. `d1` is
//! unreachable from Rust (a Workers binding exists only inside a Worker), which
//! is why this crate had no real `Database` until the self-hosted server did.

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow};
use sqlx::{Column, Row as _, SqlitePool, TypeInfo, ValueRef};

use crate::database::{Database, DatabaseError, Row, Value};
use crate::BoxFuture;

/// A SQLite database behind a connection pool.
pub struct SqliteDatabase {
    pool: SqlitePool,
}

fn conn(e: sqlx::Error) -> DatabaseError {
    DatabaseError::Connection(e.to_string())
}

fn failed(e: sqlx::Error) -> DatabaseError {
    DatabaseError::QueryFailed(e.to_string())
}

impl SqliteDatabase {
    /// Open (creating if absent) the database at `path`, in WAL mode with
    /// foreign keys enforced and a busy timeout, so a second writer waits
    /// rather than failing at once.
    pub async fn open(path: &str) -> Result<Self, DatabaseError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await
            .map_err(conn)?;
        Ok(Self { pool })
    }

    /// A private in-memory database. One connection: each in-memory connection
    /// is its own database, so a pool of more would see different ones.
    pub async fn in_memory() -> Result<Self, DatabaseError> {
        let options = SqliteConnectOptions::new()
            .in_memory(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(conn)?;
        Ok(Self { pool })
    }
}

fn bind<'q>(
    q: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    v: &'q Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match v {
        Value::Null => q.bind(None::<i64>),
        Value::Bool(b) => q.bind(*b),
        Value::Integer(i) => q.bind(*i),
        Value::Float(f) => q.bind(*f),
        Value::Text(t) => q.bind(t.as_str()),
        Value::Blob(b) => q.bind(b.as_slice()),
    }
}

fn to_row(row: &SqliteRow) -> Result<Row, DatabaseError> {
    let mut map = std::collections::BTreeMap::new();
    for (i, col) in row.columns().iter().enumerate() {
        let raw = row.try_get_raw(i).map_err(failed)?;
        let value = if raw.is_null() {
            Value::Null
        } else {
            // SQLite is dynamically typed: the type is the stored value's, not
            // the column's declaration.
            match raw.type_info().name() {
                "INTEGER" | "INT" | "BIGINT" | "BOOLEAN" => {
                    Value::Integer(row.try_get::<i64, _>(i).map_err(failed)?)
                }
                "REAL" | "FLOAT" | "DOUBLE" => {
                    Value::Float(row.try_get::<f64, _>(i).map_err(failed)?)
                }
                "BLOB" => Value::Blob(row.try_get::<Vec<u8>, _>(i).map_err(failed)?),
                _ => Value::Text(row.try_get::<String, _>(i).map_err(failed)?),
            }
        };
        map.insert(col.name().to_string(), value);
    }
    Ok(Row(map))
}

impl Database for SqliteDatabase {
    fn execute<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<u64, DatabaseError>> {
        Box::pin(async move {
            let mut q = sqlx::query(sql);
            for p in params {
                q = bind(q, p);
            }
            let done = q.execute(&self.pool).await.map_err(failed)?;
            Ok(done.rows_affected())
        })
    }

    fn query<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<Vec<Row>, DatabaseError>> {
        Box::pin(async move {
            let mut q = sqlx::query(sql);
            for p in params {
                q = bind(q, p);
            }
            let rows = q.fetch_all(&self.pool).await.map_err(failed)?;
            rows.iter().map(to_row).collect()
        })
    }

    fn query_one<'a>(
        &'a self,
        sql: &'a str,
        params: &'a [Value],
    ) -> BoxFuture<'a, Result<Option<Row>, DatabaseError>> {
        Box::pin(async move {
            let mut q = sqlx::query(sql);
            for p in params {
                q = bind(q, p);
            }
            match q.fetch_optional(&self.pool).await.map_err(failed)? {
                Some(r) => Ok(Some(to_row(&r)?)),
                None => Ok(None),
            }
        })
    }

    fn batch<'a>(
        &'a self,
        statements: &'a [(&'a str, &'a [Value])],
    ) -> BoxFuture<'a, Result<Vec<u64>, DatabaseError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(conn)?;
            let mut counts = Vec::with_capacity(statements.len());
            for (step, (sql, params)) in statements.iter().enumerate() {
                let mut q = sqlx::query(sql);
                for p in *params {
                    q = bind(q, p);
                }
                // Dropping `tx` on an early return rolls the whole batch back.
                let done = q
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| DatabaseError::Migration {
                        step: step as u32,
                        message: e.to_string(),
                    })?;
                counts.push(done.rows_affected());
            }
            tx.commit().await.map_err(conn)?;
            Ok(counts)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn db() -> SqliteDatabase {
        let db = SqliteDatabase::in_memory().await.unwrap();
        db.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT, score REAL, data BLOB)",
            &[],
        )
        .await
        .unwrap();
        db
    }

    #[tokio::test]
    async fn a_value_of_every_kind_round_trips() {
        let db = db().await;
        let n = db
            .execute(
                "INSERT INTO t (id, name, score, data) VALUES (?, ?, ?, ?)",
                &[
                    Value::Integer(1),
                    Value::Text("a".into()),
                    Value::Float(1.5),
                    Value::Blob(vec![1, 2, 3]),
                ],
            )
            .await
            .unwrap();
        assert_eq!(n, 1);
        let row = db
            .query_one("SELECT * FROM t WHERE id = ?", &[Value::Integer(1)])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.get("id"), Some(&Value::Integer(1)));
        assert_eq!(row.get("name"), Some(&Value::Text("a".into())));
        assert_eq!(row.get("score"), Some(&Value::Float(1.5)));
        assert_eq!(row.get("data"), Some(&Value::Blob(vec![1, 2, 3])));
    }

    #[tokio::test]
    async fn null_and_missing_rows_are_distinct() {
        let db = db().await;
        db.execute("INSERT INTO t (id) VALUES (?)", &[Value::Integer(2)])
            .await
            .unwrap();
        let row = db
            .query_one("SELECT name FROM t WHERE id = 2", &[])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.get("name"), Some(&Value::Null));
        assert!(db
            .query_one("SELECT * FROM t WHERE id = 99", &[])
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn a_failed_batch_changes_nothing() {
        let db = db().await;
        let ok: &[Value] = &[Value::Integer(1), Value::Text("x".into())];
        let dup: &[Value] = &[Value::Integer(1), Value::Text("y".into())];
        let r = db
            .batch(&[
                ("INSERT INTO t (id, name) VALUES (?, ?)", ok),
                ("INSERT INTO t (id, name) VALUES (?, ?)", dup),
            ])
            .await;
        assert!(
            matches!(r, Err(DatabaseError::Migration { step: 1, .. })),
            "{r:?}"
        );
        let rows = db.query("SELECT * FROM t", &[]).await.unwrap();
        assert!(rows.is_empty(), "the first insert must have rolled back");
    }

    #[tokio::test]
    async fn a_file_database_is_in_wal_mode() {
        let dir = std::env::temp_dir().join(format!("fid-sqlite-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("app.db");
        let db = SqliteDatabase::open(path.to_str().unwrap()).await.unwrap();
        let row = db
            .query_one("PRAGMA journal_mode", &[])
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.get("journal_mode"), Some(&Value::Text("wal".into())));
        drop(db);
        std::fs::remove_dir_all(&dir).ok();
    }
}
