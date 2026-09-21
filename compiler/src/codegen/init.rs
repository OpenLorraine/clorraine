//! Module initialization order.
//!
//! The generated entry stub runs each module's `cl_init_*` function before the
//! user's `main`. A module must be initialized after every module it imports or
//! names in a global initializer, transitively; independent modules break ties
//! on the dotted module name compared byte-wise, so the order is deterministic.
//! Inside one module the globals follow the same read relation read from
//! [`crate::init_graph`], with declaration order as the tie break. A dependency
//! cycle is rejected before codegen, but the walks stay total and fall back to
//! source order rather than looping.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Expr, Item};

use super::Emitter;

impl<'a> Emitter<'a> {
    /// The user modules in initialization order.
    ///
    /// A module is initialized after every module it imports or names in a
    /// global initializer, transitively; independent modules break ties on the
    /// dotted name compared byte-wise, so the order is deterministic.
    pub(super) fn init_order(&self) -> Vec<String> {
        let modules: Vec<String> = self.loaded.modules.keys().cloned().collect();
        let prereqs = crate::init_graph::module_dependencies(self.loaded);

        let mut order: Vec<String> = Vec::new();
        let mut done: BTreeSet<String> = BTreeSet::new();
        while order.len() < modules.len() {
            let ready = modules
                .iter()
                .filter(|name| !done.contains(*name))
                .filter(|name| {
                    prereqs
                        .get(*name)
                        .is_none_or(|deps| deps.iter().all(|dep| done.contains(dep)))
                })
                .min()
                .cloned();
            match ready {
                Some(name) => {
                    done.insert(name.clone());
                    order.push(name);
                }
                None => {
                    // A dependency cycle is E11100 before codegen; keep the
                    // emitter total by appending the rest in name order.
                    for name in &modules {
                        if !done.contains(name) {
                            done.insert(name.clone());
                            order.push(name.clone());
                        }
                    }
                }
            }
        }
        order
    }

    /// One module's globals in initialization order.
    ///
    /// The globals are declared in source order, but a global that reads a
    /// later-declared global must still be initialized after it, so the order
    /// follows the reads between the module's bindings. Declaration order
    /// breaks ties, which keeps an independent program in its source order. A
    /// `const`-only cycle is legal, so a blocked walk falls back to the
    /// remaining declaration order instead of looping.
    pub(super) fn global_init_order(&self, module_name: &str) -> Vec<(String, &'a Expr)> {
        let Some(module) = self.loaded.module(module_name) else {
            return Vec::new();
        };
        let entries: Vec<(String, &'a Expr)> = module
            .items
            .iter()
            .filter_map(|item| match item {
                Item::GlobalLet(decl) => Some((decl.name.clone(), &decl.value)),
                Item::GlobalConst(decl) => Some((decl.name.clone(), &decl.value)),
                _ => None,
            })
            .collect();
        if entries.len() < 2 {
            return entries;
        }

        // `reads[i]` holds the indices of the globals entry `i` reads.
        let index: BTreeMap<&str, usize> = entries
            .iter()
            .enumerate()
            .map(|(position, (name, _))| (name.as_str(), position))
            .collect();
        let mut reads: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); entries.len()];
        for (position, (_, value)) in entries.iter().enumerate() {
            for (read_module, name) in
                crate::init_graph::global_reads(self.loaded, module, value)
            {
                if read_module != module_name {
                    continue;
                }
                if let Some(&source) = index.get(name.as_str()) {
                    if source != position {
                        reads[position].insert(source);
                    }
                }
            }
        }

        let mut done = vec![false; entries.len()];
        let mut order = Vec::with_capacity(entries.len());
        for _ in 0..entries.len() {
            let pick = (0..entries.len())
                .filter(|position| !done[*position])
                .find(|position| reads[*position].iter().all(|source| done[*source]))
                .or_else(|| (0..entries.len()).find(|position| !done[*position]))
                .expect("an unfinished entry is either ready or part of a cycle");
            done[pick] = true;
            order.push(entries[pick].clone());
        }
        order
    }
}
