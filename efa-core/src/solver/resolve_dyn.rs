use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::asm::parser::Parse;
use crate::bytecode::Instr;
use crate::vm::CodeObject;

#[derive(Debug)]
struct DynCallResolver {
    objs: HashMap<String, CodeObject>,

    deps: HashMap<String, HashSet<String>>,
}

impl DynCallResolver {
    pub fn new(nodes: Vec<Parse>) -> Result<Self> {
        let objs = nodes
            .into_iter()
            .map(|p| (p.func_name, p.code_obj))
            .collect();
        let mut s = Self {
            objs,
            deps: HashMap::new(),
        };

        s.deps = s.solve()?;
        Ok(s)
    }

    fn solve(&self) -> Result<HashMap<String, HashSet<String>>> {
        let mut solved = HashSet::<&str>::new();

        let graph = self
            .objs
            .keys()
            .filter_map(|node| {
                if !solved.contains(node.as_str()) {
                    let deps = self.solve_node(node).and_then(|s| {
                        solved.insert(node);
                        Ok(s)
                    });
                    Some((node.to_owned(), deps))
                } else {
                    None
                }
            })
            .map(|(k, v)| v.map(|b| (k, b)))
            .collect::<Result<HashMap<String, HashSet<String>>>>()?;

        Ok(graph)
    }

    fn solve_node(&self, node: &str) -> Result<HashSet<String>> {
        self.objs
            .get(node)
            .ok_or_else(|| anyhow::anyhow!("node '{node}' not present"))
            .map(|obj| {
                obj.code
                    .iter()
                    .filter_map(|instr| match instr {
                        Instr::LoadDyn(name) => Some(name.to_string()),
                        _ => None,
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::asm::parser::Parser;

    #[test]
    fn test_resolver() {
        let parse = Parser::parse_file("./examples/call.asm").unwrap();
        let resolver = DynCallResolver::new(parse).unwrap();
        dbg!(resolver);
    }
}
