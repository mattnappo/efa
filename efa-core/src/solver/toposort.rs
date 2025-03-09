use std::cell::RefCell;
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

        let mut marks = graph
            .keys()
            .map(|k| (k, Mark::Unmarked))
            .collect::<HashMap<_, _>>();

        while let Some(node) = Self::unmarked(&marks) {
            marks.insert(node, Mark::Temp);
            (marks, sorted) = Self::visit(node, &marks, &sorted, graph)?;
        }

        Ok(())
    }

    fn unmarked(marks: &'a HashMap<&'a T, Mark>) -> Option<&T> {
        marks.iter().find_map(|(node, mark)| match mark {
            Mark::Unmarked | Mark::Temp => Some(*node),
            Mark::Final => None,
        })
    }

    fn visit(
        node: &'a T,
        marks: &HashMap<&'a T, Mark>,
        sorted: &Vec<&'a T>,
        graph: &'a Graph<T>,
    ) -> Result<(HashMap<&'a T, Mark>, Vec<&'a T>)> {
        if marks.get(node).unwrap() == &Mark::Final {
            return Ok((HashMap::new(), sorted.clone()));
        }

        if marks.get(node).unwrap() == &Mark::Temp {
            bail!("toposort: error: graph has cycle");
        }

        let mut marks = marks.clone();

        marks.insert(node, Mark::Temp);

        for in_node in graph.get(node).unwrap() {
            Self::visit(in_node, &marks, sorted, graph)?;
        }

        marks.insert(node, Mark::Final);
        let mut new_sorted = sorted.clone();

        new_sorted.insert(0, node);
        Ok((marks, new_sorted))
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
