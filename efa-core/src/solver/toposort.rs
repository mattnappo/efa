use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use anyhow::{bail, Result};

type Graph<T: Hash + Eq + PartialEq + Clone> = HashMap<T, HashSet<T>>;

type StrGraph = Graph<String>;

#[derive(Clone, Hash, Eq, PartialEq)]
enum Mark {
    Unmarked,
    Temp,
    Final,
}

#[derive(Clone)]
struct Toposort<'a, T>
where
    T: Hash + Eq + PartialEq + Clone,
{
    sorted: Vec<&'a T>,
    marks: HashMap<&'a T, Mark>,
    graph: &'a Graph<T>,
}

impl<'a, T> Toposort<'a, T>
where
    T: Hash + Eq + PartialEq + Clone,
{
    pub fn new(graph: &'a Graph<T>) -> Result<()> {
        let sorted = vec![];

        let marks = graph
            .keys()
            .map(|k| (k, Mark::Unmarked))
            .collect::<HashMap<_, _>>();

        let mut topo = Self {
            sorted,
            marks,
            graph,
        };

        while let Some(node) = topo.unmarked() {
            topo.marks.insert(node, Mark::Temp);
            topo.visit(node)?;
        }

        Ok(())
    }

    fn unmarked(&self) -> Option<&T> {
        self.marks.iter().find_map(|(node, mark)| match mark {
            Mark::Unmarked | Mark::Temp => Some(*node),
            Mark::Final => None,
        })
    }

    fn visit(&mut self, node: &'a T) -> Result<()> {
        if self.marks.get(node).unwrap() == &Mark::Final {
            return Ok(());
        }

        if self.marks.get(node).unwrap() == &Mark::Temp {
            bail!("toposort: error: graph has cycle");
        }

        self.marks.insert(node, Mark::Temp);

        for in_node in self.graph.get(node).unwrap() {
            self.visit(in_node)?;
        }

        self.marks.insert(node, Mark::Final);
        self.sorted.insert(0, node);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toposort() {
        // toposort();
    }
}
