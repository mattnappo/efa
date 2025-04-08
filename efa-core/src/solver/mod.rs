//! The solver is responsible for determining the dependence graph of a project
//! Nodes are functions, directed edges are calls, and the root node is a main function.

use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::bytecode::Instr;

mod node;
pub mod resolve_dyn;
mod toposort;

use node::{Node, NodeStore};

#[derive(Debug)]
pub struct DepGraph<'s, S: NodeStore> {
    graph: HashMap<Node, HashSet<Node>>,
    node_store: &'s S,
}

impl<'s, S> DepGraph<'_, S>
where
    S: NodeStore,
{
    pub fn new(store: &'s S) -> DepGraph<'s, S> {
        DepGraph {
            graph: HashMap::new(),
            node_store: store,
        }
    }

    pub fn solve_static(&mut self) -> Result<()> {
        let nodes = self.node_store.nodes()?;

        // Seen nodes
        let mut solved = HashSet::<Node>::new();

        // TODO: remove these clones
        nodes.into_iter().try_for_each(|node| {
            if !solved.contains(&node) {
                let deps = self.solve_node(&node)?;
                solved.insert(node.clone());
                self.graph.insert(node.clone(), deps);
            }
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(())
    }

    /// Return the dependences of the given node
    fn solve_node(&self, node: &Node) -> Result<HashSet<Node>> {
        let obj = self.node_store.get_code_object(&node.hash)?;
        let code = obj
            .code
            .iter()
            .filter(|instr| {
                matches!(
                    instr,
                    Instr::Call
                        | Instr::CallSelf
                        | Instr::LoadFunc(_)
                        | Instr::LoadDyn(_)
                )
            })
            .collect::<Vec<&Instr>>();

        // Check that each Instr::Call is preceded by a LoadFunc/LoadDyn
        let mut deps = code[..]
            .windows(2)
            .filter_map(|pair| match (pair[0], pair[1]) {
                // Want to return dependences (name, hash)
                (Instr::LoadFunc(hash), Instr::Call) => {
                    // Result<Option<String>>
                    let name = self.node_store.get_name_of_hash(hash);
                    Some((name, Ok(*hash)))
                }
                (Instr::LoadDyn(name), Instr::Call) => {
                    let hash = self
                        .node_store
                        .get_code_object_by_name(name)
                        .map(|(x, _)| x);
                    Some((Ok(Some(name.to_string())), hash))
                }
                _ => None,
            })
            .map(|(name, hash)| {
                let h = hash?;
                let n = name?.ok_or_else(|| {
                    anyhow::anyhow!("hash 0x{} has no name", hex::encode(h))
                })?;
                Ok(Node { name: n, hash: h })
            })
            .collect::<Result<HashSet<_>>>()?;

        if code.contains(&&Instr::CallSelf) {
            deps.insert(node.clone());
        }

        Ok(deps)
    }

    // fn linearize(&self) ->
}

impl<'a, T> std::fmt::Display for DepGraph<'a, T>
where
    T: NodeStore,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names = self
            .graph
            .iter()
            .map(|(node, deps)| {
                (
                    &node.name,
                    deps.iter().map(|dep| &dep.name).collect::<HashSet<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();
        write!(f, "{names:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::node::DatabaseNodeStore;
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
    fn test_solver() {
        let db = mock_db().unwrap();
        let store = DatabaseNodeStore::new(&db);
        let mut g = DepGraph::new(&store);

        g.solve_static().unwrap();

        println!("{g}");
    }
}
