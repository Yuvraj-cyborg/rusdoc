use std::collections::HashMap;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rustdoc_types::{Crate, Id, ItemEnum};

/// A parsed and indexed documentation set ready for searching.
pub struct DocIndex {
    krate: Crate,
    /// search_key → item ID
    index: Vec<(String, Id)>,
    matcher: SkimMatcherV2,
}

/// A lightweight view of a documentation item for display.
pub struct DocItem<'a> {
    pub path: Vec<String>,
    pub name: String,
    pub docs: Option<&'a str>,
    pub deprecation: Option<&'a rustdoc_types::Deprecation>,
    pub inner: &'a ItemEnum,
}

impl DocIndex {
    /// Build an index from a parsed rustdoc Crate.
    pub fn from_crate(krate: Crate) -> Self {
        let mut index = Vec::new();

        // Build path map from crate.paths
        let path_map: HashMap<&Id, String> = krate
            .paths
            .iter()
            .map(|(id, summary)| (id, summary.path.join("::")))
            .collect();

        // Index every item in the crate
        for (id, item) in &krate.index {
            // Skip items without names (e.g. impl blocks)
            let name = match &item.name {
                Some(n) => n.clone(),
                None => continue,
            };

            // Build the search key: prefer the full path from paths map
            let key = if let Some(full_path) = path_map.get(id) {
                full_path.clone()
            } else {
                name.clone()
            };

            index.push((key, id.clone()));
        }

        index.sort_by(|a, b| a.0.cmp(&b.0));

        DocIndex {
            krate,
            index,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Fuzzy search, returning up to `limit` results sorted by relevance.
    pub fn search(&self, query: &str, limit: Option<usize>) -> Vec<DocItem<'_>> {
        let query_lower = query.to_lowercase();

        let mut scored: Vec<(i64, &Id, &str)> = self
            .index
            .iter()
            .filter_map(|(key, id)| {
                self.matcher
                    .fuzzy_match(&key.to_lowercase(), &query_lower)
                    .map(|score| (score, id, key.as_str()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));

        if let Some(limit) = limit {
            scored.truncate(limit);
        }

        scored
            .into_iter()
            .filter_map(|(_, id, key)| {
                let item = self.krate.index.get(id)?;
                let path: Vec<String> = key.split("::").map(|s| s.to_string()).collect();
                let name = item.name.clone().unwrap_or_default();

                Some(DocItem {
                    path,
                    name,
                    docs: item.docs.as_deref(),
                    deprecation: item.deprecation.as_ref(),
                    inner: &item.inner,
                })
            })
            .collect()
    }

    /// Get item count for diagnostics.
    pub fn item_count(&self) -> usize {
        self.index.len()
    }
}
