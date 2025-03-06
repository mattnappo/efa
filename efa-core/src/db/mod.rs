use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::build_hash;
use crate::{is_valid_name, vm::CodeObject, Hash, HASH_SIZE};

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OpenFlags};

#[derive(Debug)]
pub struct Database {
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

        Database::build_schema(&db.conn)?;

        Ok(db)
    }

    fn build_schema(conn: &Connection) -> Result<()> {
        // Create name table
        conn.execute(
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

        conn.execute("CREATE INDEX IF NOT EXISTS name_idx ON names (name);", [])?;

        // Create code object table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS code_objs (
                id INTEGER PRIMARY KEY,
                hash BLOB UNIQUE,
                code_obj BLOB UNIQUE,
                is_main INTEGER DEFAULT (0),
                time DATETIME
            );
        "#,
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS hash_idx ON code_objs (hash);",
            [],
        )?;

        // TODO: Create type table

        Ok(())
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
        let db = Self {
            path: None,
            conn: Connection::open_in_memory().unwrap(),
        };
        Self::build_schema(&db.conn)?;
        Ok(db)
    }

    /// Delete a database
    pub fn delete(self) -> Result<()> {
        if let Some(path) = self.path {
            fs::remove_file(path)?;
        }
        // Now self gets dropped, closing self.conn
        Ok(())
    }

    fn insert_code_object(&self, code_obj: &CodeObject, is_main: bool) -> Result<Hash> {
        let obj = rmp_serde::to_vec(code_obj)?;
        let hash = code_obj.hash()?;

        self.conn.execute(
            "INSERT INTO code_objs (hash, code_obj, is_main, time) VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP);",
            params![hash, obj, is_main as u8],
        )?;

        Ok(hash)
    }

    pub fn insert_code_object_with_name(&self, code_obj: &CodeObject, name: &str) -> Result<Hash> {
        if !is_valid_name(name) {
            bail!("cannot insert code object with invalid name '{name}'");
        }

        let hash = self.insert_code_object(code_obj, name == "main")?;

        self.conn.execute(
            "INSERT INTO names (name, hash, time) VALUES (?1, ?2, CURRENT_TIMESTAMP);",
            params![name, hash],
        )?;

        Ok(hash)
    }

    /// Allow multiple names to point to the same hash.
    pub fn create_alias(&self, name: &str, hash: &Hash) -> Result<()> {
        // Check that the hash is in the thing
        let obj = self.get_code_object(hash)?;
        if obj.hash()? != *hash {
            bail!(
                "cannot create alias to unknown code object 0x'{}'",
                hex::encode(hash)
            );
        }

        self.conn.execute(
            "INSERT INTO names (name, hash, time) VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            params![name, hash],
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

        let obj = query_result
            .into_iter()
            .flatten()
            .flatten()
            .next()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "query failed: no code object with hash 0x{}",
                    hex::encode(hash)
                )
            });
        obj
    }

    pub fn get_main_object(&self) -> Result<(Hash, CodeObject)> {
        let mut stmt = self
            .conn
            .prepare("SELECT hash, code_obj FROM code_objs WHERE is_main = TRUE;")?;

        let query_result = stmt.query_map([], |row| {
            let hash: Vec<u8> = row.get(0)?;
            let code_obj_blob: Vec<u8> = row.get(1)?;
            Ok((hash, rmp_serde::from_slice::<CodeObject>(&code_obj_blob)))
        })?;

        let (hash, obj) = query_result
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| anyhow::anyhow!("query failed: no main object found"))?;

        Ok((build_hash(hash)?, obj?))
    }

    pub fn get_code_object_by_name(&self, name: &str) -> Result<(Hash, CodeObject)> {
        let mut stmt = self
            .conn
            .prepare("SELECT hash FROM names WHERE name = ?1;")?;

        let query_result = stmt.query_map([name], |row| {
            let hash: Vec<u8> = row.get(0)?;
            Ok(hash)
        })?;

        let hash = match query_result.into_iter().next() {
            Some(h) => h?,
            None => bail!("query failed: no code object with name '{name}'"),
        };

        let hash: Hash = hash[0..HASH_SIZE]
            .try_into()
            .map_err(|_| anyhow::anyhow!("failed to hash CodeObject"))?;

        Ok((hash, self.get_code_object(&hash)?))
    }

    pub fn get_name_of_hash(&self, hash: &Hash) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM names WHERE hash = ?1;")?;

        let query_result = stmt.query_map([hash], |row| {
            let name = row.get(0)?;
            Ok(name)
        })?;

        let res = query_result.into_iter().next().transpose();
        Ok(res?)
    }

    pub fn get_functions(&self) -> Result<Vec<(String, Hash)>> {
        let mut stmt = self.conn.prepare("SELECT name, hash FROM names;")?;

        let query_result = stmt.query_map([], |row| {
            let name = row.get(0)?;
            let hash = row.get(1)?;
            Ok((name, hash))
        })?;
        let res = query_result.collect::<rusqlite::Result<_>>()?;
        Ok(res)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::bytecode::Instr;
    use crate::vm::tests::{init_code_obj, init_nondet_code_obj};

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
        let obj = init_code_obj(bytecode![Instr::Nop]);

        db.insert_code_object(&obj, false).unwrap();
    }

    #[test]
    fn test_get_codeobj() {
        let db = Database::open("/tmp/test.db").unwrap();
        let obj = init_code_obj(bytecode![Instr::Nop]);

        let res = db.get_code_object(&obj.hash().unwrap()).unwrap();
        assert_eq!(res.hash().unwrap(), obj.hash().unwrap());
    }

    #[test]
    fn test_insert_codeobj_name() {
        let db = Database::open("/tmp/test.db").unwrap();
        let obj1 = init_code_obj(bytecode![]);
        let obj2 = init_nondet_code_obj(bytecode![]);

        db.insert_code_object_with_name(&obj1, "random_obj")
            .unwrap();

        assert!(db
            .insert_code_object_with_name(&obj2, "random obj2")
            .is_err());

        // same name case
        // same code object case
        // invalid name case
    }

    #[test]
    fn test_get_codeobj_name() {
        let db = Database::open("/tmp/test.db").unwrap();
        let obj = init_code_obj(bytecode![]);
        let (hash, _) = db.get_code_object_by_name("random_obj").unwrap();
        assert_eq!(obj.hash().unwrap(), hash);
    }

    #[test]
    fn test_create_alias() {
        let db = Database::open("/tmp/test.db").unwrap();
        let hash = init_code_obj(bytecode![]).hash().unwrap();

        db.create_alias("name_2", &hash).unwrap();
    }

    #[test]
    fn test_name_of_hash() {
        let db = Database::temp().unwrap();
        let obj = init_code_obj(bytecode![Instr::Return]);

        let hash = db.insert_code_object_with_name(&obj, "func_name").unwrap();

        let name = db.get_name_of_hash(&hash).unwrap();
        assert_eq!(name, Some("func_name".to_string()));
    }
}
