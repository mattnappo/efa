use std::collections::HashSet;
use std::hash::Hash as StdHash;

use anyhow::Result;
use derivative::Derivative;

use crate::db::Database;
use crate::vm::CodeObject;
use crate::Hash;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Node {
    pub(crate) hash: Hash,
    pub(crate) name: String,
}

pub trait NodeStore: Clone + StdHash + PartialEq + Eq {
    fn get_code_object(&self, hash: &Hash) -> Result<CodeObject>;
    fn get_name_of_hash(&self, hash: &Hash) -> Result<Option<String>>;
    fn get_code_object_by_name(&self, name: &str) -> Result<(Hash, CodeObject)>;
    fn nodes(&self) -> Result<HashSet<Node>>;
}

/// A node whose code object resides in a database.
#[derive(Derivative)]
#[derivative(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DatabaseNodeStore<'a> {
    #[derivative(PartialEq = "ignore", Hash = "ignore")]
    db: &'a Database,
}

impl<'a> DatabaseNodeStore<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }
}

/// A node from a file currently being analyzed, whose code object is stored in a `Parse`
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct FreeNode {
    hash: Hash,
    name: String,
}

impl NodeStore for DatabaseNodeStore<'_> {
    fn get_code_object(&self, hash: &Hash) -> Result<CodeObject> {
        self.db.get_code_object(&hash)
    }

    fn nodes(&self) -> Result<HashSet<Node>> {
        let nodes = self
            .db
            .get_functions()?
            .into_iter()
            .map(|(name, hash)| Node { name, hash })
            .collect::<HashSet<_>>();
        Ok(nodes)
    }

    fn get_name_of_hash(&self, hash: &Hash) -> Result<Option<String>> {
        self.db.get_name_of_hash(hash)
    }

    fn get_code_object_by_name(&self, name: &str) -> Result<(Hash, CodeObject)> {
        self.db.get_code_object_by_name(name)
    }
}
