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

impl<'a> DepGraph<'_> {
    pub fn new(db: &'a Database) -> DepGraph<'a> {
        DepGraph {
            graph: HashMap::new(),
            db,
        }
    }

    pub fn solve(&mut self) -> Result<()> {
        let main = self.db.get_main_object()?;
        dbg!(main);
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Instr;
    use crate::db::Database;
    use crate::vm::tests::init_code_obj;

    fn mock_db() -> Result<Database> {
        let db = Database::temp()?;

        let obj = init_code_obj(bytecode![Instr::Nop, Instr::Return]);
        db.insert_code_object_with_name(&obj, "main")?;

        Ok(db)
    }

    #[test]
    fn test_() {
        let db = mock_db().unwrap();
        let mut g = DepGraph::new(&db);

        g.solve().unwrap();
    }
}
