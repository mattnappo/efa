use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::asm::parser::Parse;
use crate::bytecode::{Bytecode, Instr};
use crate::vm::CodeObject;
use crate::Hash;

use super::toposort::toposort;

#[derive(Debug)]
pub struct DynCallResolver {
    objs: HashMap<String, CodeObject>,
    deps: HashMap<String, HashSet<String>>,

    hash_order: Vec<String>,
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
            hash_order: vec![],
        };

        s.deps = s.solve()?;
        s.hash_order = toposort(&s.deps)?;
        Ok(s)
    }

    /// Compute the hashes of the code objects, replacing `LoadDyn` instructions with
    /// `LoadHash` when possible. Takes ownership since the modified code objects are
    /// returned back.
    pub fn resolve_dyn_calls(self) -> Result<HashMap<String, CodeObject>> {
        /*
           already_hashed = Map<Name, Hash>
           for name in hash_order.rev()
               dyns = hash[name].code.filter(LoadDyn)
               for dyn in dyns:
                   if dyn.name in already_hashed:
                       dyn.name = already_hashed[dyn.name]
               hash = objs[name].hash()
               already_hashed[name] = hash
               (name, new_obj)
           collect into map
        */

        // Keep track of the code objects we've already hashed
        let mut hashed = HashMap::<String, Hash>::new();

        let new_objs = self
            .hash_order
            .into_iter()
            .rev()
            .map(|name| {
                let obj = self
                    .objs
                    .get(&name)
                    .ok_or_else(|| anyhow!("object '{name}' not present"))?;

                let new_instrs: Vec<Instr> = obj
                    .code
                    .iter()
                    .map(|instr| match instr {
                        Instr::LoadDyn(dyn_name) => {
                            let hash = hashed.get(dyn_name.as_str())
                                .ok_or_else(|| anyhow!("dyn_name '{name}' should have already been hashed"))?;

                            Ok(Instr::LoadFunc(*hash))
                        }
                        e => Ok(e.clone()),
                    })
                    .collect::<Result<_>>()?;

                let new_obj = {
                    let mut c = obj.clone();
                    c.code = Bytecode::new(new_instrs);
                    c
                };

                hashed.insert(name.clone(), new_obj.hash()?);

                Ok((name, new_obj))
            })
            .collect::<Result<_>>()?;

        Ok(new_objs)
    }

    fn solve(&self) -> Result<HashMap<String, HashSet<String>>> {
        let mut solved = HashSet::<&str>::new();

        let graph = self
            .objs
            .keys()
            .filter_map(|node| {
                if !solved.contains(node.as_str()) {
                    let deps = self.solve_node(node).map(|s| {
                        solved.insert(node);
                        s
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
        dbg!(&resolver);
        let resolved = resolver.resolve_dyn_calls().unwrap();
        dbg!(resolved);
    }
}
