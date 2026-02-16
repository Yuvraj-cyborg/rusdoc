use rustdoc_types::ItemEnum;

use crate::doc::{DocIndex, DocItem};
use crate::error::{Result, RusdocError};

/// Result of resolving a query against indexed docs.
pub enum ResolveResult<'a> {
    /// Exactly one match.
    Found(DocItem<'a>),
    /// Multiple matches — caller should disambiguate.
    Multiple(Vec<DocItem<'a>>),
}

/// Resolve a documentation query against indexed docs.
pub fn resolve<'a>(doc: &'a DocIndex, query: &str) -> Result<ResolveResult<'a>> {
    let normalized = query.replace('.', "::");

    let results = doc.search(&normalized, Some(50));
    let exact: Vec<DocItem<'a>> = results
        .into_iter()
        .filter(|item| item.path.join("::") == normalized)
        .collect();

    if exact.len() == 1 {
        return Ok(ResolveResult::Found(exact.into_iter().next().unwrap()));
    }
    if exact.len() > 1 {
        return Ok(ResolveResult::Multiple(exact));
    }

    let results = doc.search(&normalized, Some(100));
    let suffix: Vec<DocItem<'a>> = results
        .into_iter()
        .filter(|item| item.path.join("::").ends_with(&normalized))
        .collect();

    if suffix.len() == 1 {
        return Ok(ResolveResult::Found(suffix.into_iter().next().unwrap()));
    }
    if suffix.len() > 1 {
        return Ok(ResolveResult::Multiple(suffix));
    }
    let results = doc.search(&normalized, Some(20));
    if results.len() == 1 {
        return Ok(ResolveResult::Found(results.into_iter().next().unwrap()));
    }
    if !results.is_empty() {
        return Ok(ResolveResult::Multiple(results));
    }

    Err(RusdocError::NotFound {
        query: query.to_string(),
    })
}

/// Format a list of items as disambiguation choices for the user.
pub fn format_disambiguation(items: &[DocItem<'_>]) -> String {
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        let path = item.path.join("::");
        let kind = item_kind_label(item.inner);
        out.push_str(&format!("  {: >3}. {} ({})\n", i + 1, path, kind));
    }
    out
}

pub fn item_kind_label(inner: &ItemEnum) -> &'static str {
    match inner {
        ItemEnum::Module(_) => "mod",
        ItemEnum::ExternCrate { .. } => "extern crate",
        ItemEnum::Use(_) => "use",
        ItemEnum::Struct(_) => "struct",
        ItemEnum::StructField(_) => "field",
        ItemEnum::Union(_) => "union",
        ItemEnum::Enum(_) => "enum",
        ItemEnum::Variant(_) => "variant",
        ItemEnum::Function(_) => "fn",
        ItemEnum::Trait(_) => "trait",
        ItemEnum::TraitAlias(_) => "trait alias",
        ItemEnum::Impl(_) => "impl",
        ItemEnum::TypeAlias(_) => "type",
        ItemEnum::Constant { .. } => "const",
        ItemEnum::Static(_) => "static",
        ItemEnum::Macro(_) => "macro",
        ItemEnum::ProcMacro(_) => "proc macro",
        ItemEnum::Primitive(_) => "primitive",
        ItemEnum::AssocConst { .. } => "assoc const",
        ItemEnum::AssocType { .. } => "assoc type",
        _ => "item",
    }
}
