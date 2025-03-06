//! The solver is responsible for determining the dependence graph of a project
//! Nodes are functions, directed edges are calls, and the root node is a main function.

use std::collections::HashMap;

use anyhow::Result;

use crate::db::Database;
use crate::Hash;

struct Node {
    hash: Hash,
    name: String,
}

pub struct DepGraph<'a> {
    graph: HashMap<Node, Node>,

    db: &'a Database,
}

impl<'a> DepGraph {
    pub fn new(db: &'a Database) -> DepGraph<'a> {
        DepGraph {
            graph: HashMap::new(),
            db,
        }
    }

    pub fn solve(&self) -> Result<()> {
        todo!()
    }
}
