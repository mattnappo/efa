//! The solver is responsible for determining the dependence graph of a project
//! Nodes are functions, directed edges are calls, and the root node is a main function.

use std::collections::HashMap;

use anyhow::Result;

use crate::bytecode::Instr;
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

    pub fn solve_static(&mut self) -> Result<()> {
        let (main_hash, main_obj) = self.db.get_main_object()?;
        let code = main_obj
            .code
            .iter()
            .filter(|instr| match instr {
                Instr::Call | Instr::CallSelf | Instr::LoadFunc(_) | Instr::LoadDyn(_) => true,
                _ => false,
            })
            .collect::<Vec<&Instr>>();

        let mut calls_self = false;

        // Check that each Instr::Call is preceded by a LoadFunc/LoadDyn
        let deps = &code[..]
            .windows(2)
            .filter_map(|pair| match (pair[0], pair[1]) {
                // Want to return dependences (name, hash)
                (Instr::LoadFunc(hash), Instr::Call) => {
                    // Result<Option<String>>
                    let name = self.db.get_name_of_hash(hash);
                    Some((name, *hash))
                }
                (Instr::LoadDyn(name), Instr::Call) => {
                    let (hash, _) = self.db.get_code_object_by_name(name).unwrap();
                    Some((Ok(Some(name.to_string())), hash))
                }
                (_, Instr::Call) => {
                    calls_self = true;
                    None
                }
                _ => None,
            })
            .map(|(name, hash)| {
                let n = name?
                    .ok_or_else(|| anyhow::anyhow!("hash 0x{} has no name", hex::encode(hash)))?;
                Ok((n, hash))
            })
            .collect::<Result<Vec<_>>>()?;

        dbg!(deps);

        Ok(())
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

        let foo = init_code_obj(bytecode![Instr::CallSelf, Instr::Return]);

        let hash_foo = db.insert_code_object_with_name(&foo, "foo")?;

        let main = init_code_obj(bytecode![
            Instr::LoadFunc(hash_foo),
            Instr::Call,
            Instr::CallSelf,
            Instr::Return
        ]);
        db.insert_code_object_with_name(&main, "main")?;

        Ok(db)
    }

    #[test]
    fn test_() {
        let db = mock_db().unwrap();
        let mut g = DepGraph::new(&db);

        g.solve_static().unwrap();
    }
}
