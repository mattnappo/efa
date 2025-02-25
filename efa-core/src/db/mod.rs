use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{vm::CodeObject, Hash};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OpenFlags};

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

    pub fn insert_code_object(&self, code_obj: &CodeObject) -> Result<()> {
        let obj = rmp_serde::to_vec(code_obj)?;
        let hash = code_obj.hash()?;

        self.conn.execute(
            "INSERT INTO code_objs (hash, code_obj, time) VALUES (?1, ?2, CURRENT_TIMESTAMP);",
            params![hash, obj],
        )?;

        Ok(())
    }

    pub fn get_code_object(&self, hash: &Hash) -> Result<CodeObject> {
        let mut stmt = self
            .conn
            .prepare("SELECT code_obj FROM code_objs WHERE hash = (?1);")?;

        let query_result = stmt.query_map([hash], |row| {
            let code_obj_blob: Vec<u8> = row.get(0)?;
            Ok(rmp_serde::from_slice::<CodeObject>(&code_obj_blob))
        })?;

        let obj = match query_result.into_iter().next() {
            Some(obj) => Ok(obj??),
            None => bail!(
                "query failed: no code object with hash 0x{}",
                hex::encode(hash)
            ),
        };

        obj
    }

    // TODO: Now must write functions for:
    // - insert new code object into table
    // -- optionally give it a name (something for names to point to)
    // - lookup code object by hash (SELECT on only second table)
    // - lookup code object by name (SELECT on JOIN both tables)
    // -
    // -
    // -
}

#[cfg(test)]
pub mod tests {
    use crate::bytecode::{Bytecode, Instr};
    use crate::vm::tests::init_code_obj;

    use super::*;

    #[ignore]
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

    #[test]
    fn test_insert_codeobj() {
        let db = Database::open("/tmp/test.db").unwrap();
        let obj = init_code_obj(Bytecode::new(vec![Instr::Nop]));

        db.insert_code_object(&obj).unwrap();
    }

    #[test]
    fn test_get_codeobj() {
        let db = Database::open("/tmp/test.db").unwrap();
        let obj = init_code_obj(Bytecode::new(vec![Instr::Nop]));

        let res = db.get_code_object(&obj.hash().unwrap()).unwrap();
        assert_eq!(res.hash().unwrap(), obj.hash().unwrap());
    }
}
