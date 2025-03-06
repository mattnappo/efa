//! The solver is responsible for determining the dependence graph of a project
//! Nodes are functions, directed edges are calls, and the root node is a main function.

use std::collections::HashMap;
use std::collections::HashSet;

use anyhow::Result;
use derivative::Derivative;

use crate::bytecode::Instr;
use crate::db::Database;
use crate::vm::CodeObject;
use crate::Hash;

#[derive(Derivative)]
#[derivative(Hash, PartialEq, Eq)]
struct Node {
    hash: Hash,
    name: String,
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    code_object: Box<CodeObject>,
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
        let (hash, obj) = self.db.get_main_object()?;

        let main_node = Node {
            name: "main".to_string(),
            hash,
            code_object: Box::new(obj),
        };

        // make a set of solved nodes

        let deps = self.solve_node(&main_node)?;
        /*
        for dep in deps {
            if dep not already solved (dep not in set) {
                solve(dep)
            }
        }
        */

        // Aggregate into self.deps
        Ok(())
    }

    /// Return the dependences of the given node
    fn solve_node(&self, node: &Node) -> Result<HashSet<Node>> {
        let obj = self.db.get_code_object(&node.hash)?;
        let code = obj
            .code
            .iter()
            .filter(|instr| match instr {
                Instr::Call | Instr::CallSelf | Instr::LoadFunc(_) | Instr::LoadDyn(_) => true,
                _ => false,
            })
            .collect::<Vec<&Instr>>();

        // Check that each Instr::Call is preceded by a LoadFunc/LoadDyn
        let deps = code[..]
            .windows(2)
            .filter_map(|pair| match (pair[0], pair[1]) {
                // Want to return dependences (name, hash)
                (Instr::LoadFunc(hash), Instr::Call) => {
                    // Result<Option<String>>
                    let name = self.db.get_name_of_hash(hash);
                    Some((name, *hash))
                }
                (Instr::LoadDyn(name), Instr::Call) => {
                    let (hash, obj) = self.db.get_code_object_by_name(name).unwrap();
                    Some((Ok(Some(name.to_string())), hash))
                }
                (_, Instr::Call) => Some((Ok(Some(node.name)), node.hash)),
                _ => None,
            })
            .map(|(name, hash)| {
                let n = name?
                    .ok_or_else(|| anyhow::anyhow!("hash 0x{} has no name", hex::encode(hash)))?;
                let code_object = Box::new(self.db.get_code_object(&hash)?);
                Ok(Node {
                    name: n,
                    hash,
                    code_object,
                })
            })
            .collect::<Result<HashSet<_>>>()?;

        Ok(deps)
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
