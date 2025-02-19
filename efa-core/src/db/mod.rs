use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use rusqlite::{Connection, OpenFlags};

#[derive(Debug)]
struct Database {
    path: Option<PathBuf>,
    conn: Connection,
}

impl Database {
    /// Create a new database.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        if path.as_ref().exists() {
            bail!("cannot create new database: already exists");
        }

        let db = Self {
            path: Some(path.as_ref().to_path_buf()),
            conn: Connection::open(path)?,
        };

        // Create name table
        db.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS names (
                id INTEGER PRIMARY KEY,
                name VARCHAR(255) UNIQUE,
                hash BLOB,
                time DATETIME
            );
        "#,
            [],
        )?;
        db.conn
            .execute("CREATE INDEX IF NOT EXISTS name_idx ON names (name);", [])?;

        // Create code object table
        db.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS code_objs (
                id INTEGER PRIMARY KEY,
                hash BLOB UNIQUE,
                code_obj BLOB UNIQUE,
                time DATETIME
            );
        "#,
            [],
        )?;
        db.conn.execute(
            "CREATE INDEX IF NOT EXISTS hash_idx ON code_objs (hash);",
            [],
        )?;

        // TODO: Create type table

        Ok(db)
    }

    /// Open an existing database.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Self {
            path: Some(path.as_ref().to_path_buf()),
            conn: Connection::open_with_flags(
                path,
                OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )?,
        })
    }

    /// Create an in-memory database.
    pub fn temp() -> Result<Self> {
        Ok(Self {
            path: None,
            conn: Connection::open_in_memory()?,
        })
    }

    /// Delete a database
    pub fn delete(self) -> Result<()> {
        if let Some(path) = self.path {
            fs::remove_file(path)?;
        }
        // Now self gets dropped, closing self.conn
        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    // Just put a test.db on the disk for out-of-band inspection
    fn put_on_disk() {
        let path = Path::new("/tmp/test.db");
        if path.exists() {
            fs::remove_file(path).unwrap();
        }
        Database::new(&path).unwrap();
    }

    #[test]
    fn test_new_open() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        Database::new(&path).unwrap();
        let db = Database::open(&path).unwrap();
        db.delete().unwrap();
    }
}
