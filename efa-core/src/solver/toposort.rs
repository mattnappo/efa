use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use anyhow::{anyhow, bail, Result};

type Graph<T> = HashMap<T, HashSet<T>>;

pub fn toposort<T>(graph: &Graph<T>) -> Result<Vec<T>>
where
    T: Hash + Eq + PartialEq + Clone + Debug,
{
    let soln = graph.iter().try_fold(vec![], |acc, (node, _)| {
        visit_node(graph, node, vec![], acc.clone())
    })?;
    Ok(soln)
}

fn visit_node<T>(
    graph: &Graph<T>,
    node: &T,
    path: Vec<T>,
    visited: Vec<T>,
) -> Result<Vec<T>>
where
    T: Hash + Eq + PartialEq + Clone + Debug,
{
    if path.contains(&node) {
        bail!("toposort: cycle found");
    } else if visited.contains(node) {
        Ok(visited)
    } else {
        let edges = graph
            .get(&node)
            .ok_or_else(|| anyhow!("toposort: node '{node:?}' not present in graph"))?;

        let mut new_path = path.clone();
        new_path.insert(0, node.clone());

        let mut new_visited =
            edges.into_iter().try_fold(visited.clone(), |acc, edge| {
                visit_node(graph, edge, new_path.clone(), acc.clone())
            })?;

        new_visited.insert(0, node.clone());
        Ok(new_visited)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toposort() {
        assert_eq!(
            toposort(&HashMap::from([
                ("a", HashSet::from(["b", "c"])),
                ("b", HashSet::from(["c", "d"])),
                ("c", HashSet::from(["d"])),
                ("d", HashSet::new()),
            ]))
            .unwrap(),
            vec!["a", "b", "c", "d"]
        );
    }
}
