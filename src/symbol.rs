use std::{cmp::Ordering, collections::HashMap};

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::*;

#[derive(Debug, Default)]
pub struct SymbolTable {
    /// invariant:
    /// - the symbols in the same are sorted by line number and then by column number
    /// - any two ranges cannot overlap
    pub inner: HashMap<Url, Vec<Symbol>>,
}

impl SymbolTable {
    pub fn merge_replace(&mut self, other: Self) {
        for (url, symbols) in other.inner {
            self.inner.entry(url).insert_entry(symbols);
        }
    }

    pub fn query(&self, url: &Url, position: Position) -> Option<Symbol> {
        let (Ok(idx) | Err(idx)) = self.inner.get(url)?.binary_search(&Symbol {
            name: String::new(),
            ty: String::new(),
            range: Range {
                start: position,
                end: position,
            },
        });
        let symbol = self.inner.get(url)?.get(idx)?;
        (position.line == symbol.range.start.line
            && position.character >= symbol.range.start.character
            && position.character < symbol.range.end.character)
            .then(|| symbol.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: String,
    pub range: Range,
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> Ordering {
        if let res @ (Ordering::Less | Ordering::Greater) =
            self.range.start.line.cmp(&other.range.start.line)
        {
            return res;
        }
        if let Ordering::Less = self.range.end.character.cmp(&other.range.start.character) {
            return Ordering::Less;
        }
        if let Ordering::Greater = self.range.start.character.cmp(&other.range.end.character) {
            return Ordering::Greater;
        }
        Ordering::Equal
    }
}
