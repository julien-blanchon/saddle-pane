//! Fuzzy icon name suggestions using FST (finite state transducer) with
//! Levenshtein automaton. The collection's icon names are fetched once from
//! the Iconify API, built into an FST, and cached as a compact binary file
//! (~30-50 KB for 7000 icons). Subsequent compilations load the FST directly
//! — no JSON parsing, no sorting, just a memory map.

use std::collections::HashSet;
use std::fs;

use fst::automaton::Levenshtein;
use fst::{IntoStreamer, Set};
use serde::Deserialize;

/// Partial response from the Iconify `/collection` endpoint.
#[derive(Deserialize)]
struct CollectionResponse {
    #[serde(default)]
    uncategorized: Vec<String>,
    #[serde(default)]
    categories: std::collections::HashMap<String, Vec<String>>,
    #[serde(default)]
    aliases: std::collections::HashMap<String, String>,
    #[serde(default)]
    hidden: Vec<String>,
}

/// Try to load a cached FST for the given prefix.
fn load_cached_fst(prefix: &str) -> Option<Set<Vec<u8>>> {
    let path = crate::cache::collections_dir().join(format!("{prefix}.fst"));
    let bytes = fs::read(&path).ok()?;
    Set::new(bytes).ok()
}

/// Build an FST from a sorted list of icon names and cache it to disk.
fn build_and_cache_fst(prefix: &str, icons: &[String]) -> Option<Set<Vec<u8>>> {
    let bytes = {
        let mut builder = fst::SetBuilder::memory();
        for icon in icons {
            builder.insert(icon).ok()?;
        }
        builder.into_inner().ok()?
    };

    // Cache the FST binary (much smaller and faster to load than JSON)
    let cache_dir = crate::cache::collections_dir();
    if let Some(parent) = cache_dir.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::create_dir_all(&cache_dir);
    let _ = fs::write(cache_dir.join(format!("{prefix}.fst")), &bytes);

    Set::new(bytes).ok()
}

/// Fetch all icon names for a prefix from the Iconify API, build an FST,
/// and cache it. Returns the FST set for fuzzy queries.
fn get_or_build_fst(prefix: &str, api_url: &str) -> Option<Set<Vec<u8>>> {
    // Fast path: load cached FST (binary, ~30-50 KB, instant)
    if let Some(set) = load_cached_fst(prefix) {
        return Some(set);
    }

    // Slow path: fetch collection from API, build FST, cache it
    let url = format!("{api_url}/collection?prefix={prefix}");
    let response = ureq::get(&url).call().ok()?;
    let text = response.into_string().ok()?;
    let collection: CollectionResponse = serde_json::from_str(&text).ok()?;

    // Extract all visible icon names
    let hidden: HashSet<&str> = collection.hidden.iter().map(|s| s.as_str()).collect();
    let mut icons = HashSet::new();

    for name in &collection.uncategorized {
        if !hidden.contains(name.as_str()) {
            icons.insert(name.clone());
        }
    }
    for names in collection.categories.values() {
        for name in names {
            if !hidden.contains(name.as_str()) {
                icons.insert(name.clone());
            }
        }
    }
    for alias in collection.aliases.keys() {
        if !hidden.contains(alias.as_str()) {
            icons.insert(alias.clone());
        }
    }

    // FST requires sorted, deduplicated input
    let mut sorted: Vec<String> = icons.into_iter().collect();
    sorted.sort();

    build_and_cache_fst(prefix, &sorted)
}

/// Use the Levenshtein automaton to find fuzzy matches within `max_distance` edits.
fn fst_fuzzy_search(set: &Set<Vec<u8>>, query: &str, max_distance: u32) -> Vec<String> {
    match Levenshtein::new(query, max_distance) {
        Ok(lev) => {
            let stream = set.search(&lev).into_stream();
            stream.into_strs().unwrap_or_default()
        }
        Err(_) => vec![], // automaton too large (very long query + high distance)
    }
}

/// Build an error message with fuzzy suggestions and a CLI tip.
pub(crate) fn suggest_error_message(pack: &str, name: &str, url: &str, api_url: &str) -> String {
    let mut msg = format!("icon not found: {url}");

    if let Some(set) = get_or_build_fst(pack, api_url) {
        // Try distance 1 first (fast, catches single-char typos)
        let mut matches = fst_fuzzy_search(&set, name, 1);

        // If no results at distance 1, try distance 2
        if matches.is_empty() {
            matches = fst_fuzzy_search(&set, name, 2);
        }

        // Also find substring matches (FST doesn't do this, so iterate)
        let all_keys: Vec<String> = {
            let stream = set.into_stream();
            stream.into_strs().unwrap_or_default()
        };
        let subs: Vec<&String> = all_keys
            .iter()
            .filter(|icon| icon.contains(name) || name.contains(icon.as_str()))
            .take(5)
            .collect();

        // Deduplicate and show up to 5
        if !matches.is_empty() || !subs.is_empty() {
            msg.push_str("\n\nDid you mean:");
            let mut shown = HashSet::new();
            for icon in matches.iter().chain(subs.iter().copied()).take(5) {
                if shown.insert(icon.clone()) {
                    msg.push_str(&format!("\n  {pack}:{icon}"));
                }
            }
        }

        msg.push_str(&format!(
            "\n\nTip: use `iconify_cli search {name}` to browse available icons.\
             \n     use `iconify_cli collection {pack}` to list all icons in the `{pack}` set."
        ));
    } else {
        msg.push_str(&format!(
            "\n\nCould not fetch icon list for `{pack}` to suggest alternatives.\
             \nTip: use `iconify_cli search {name}` to find icons."
        ));
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fst_fuzzy_finds_typo() {
        let mut icons: Vec<String> = vec!["arrow-left", "arrow-right", "home", "sword", "shield"]
            .into_iter()
            .map(String::from)
            .collect();
        icons.sort();

        let set = Set::from_iter(icons).unwrap();
        let results = fst_fuzzy_search(&set, "swrod", 2);
        assert!(results.contains(&"sword".to_string()));
    }

    #[test]
    fn fst_fuzzy_distance_1() {
        let icons: Vec<String> = vec!["home", "hone", "hope", "zone"]
            .into_iter()
            .map(String::from)
            .collect();
        let set = Set::from_iter(icons).unwrap();

        let results = fst_fuzzy_search(&set, "home", 1);
        assert!(results.contains(&"home".to_string())); // exact
        assert!(results.contains(&"hone".to_string())); // 1 edit
        assert!(!results.contains(&"zone".to_string())); // 2 edits
    }

    #[test]
    fn fst_empty_on_no_match() {
        let icons: Vec<String> = vec!["abc", "def"].into_iter().map(String::from).collect();
        let set = Set::from_iter(icons).unwrap();

        let results = fst_fuzzy_search(&set, "xyz", 1);
        assert!(results.is_empty());
    }

    #[test]
    fn fst_roundtrip_serialization() {
        let icons: Vec<String> = vec!["arrow", "home", "star", "sword"]
            .into_iter()
            .map(String::from)
            .collect();
        let set = Set::from_iter(icons).unwrap();

        // Serialize
        let bytes = set.as_fst().as_bytes().to_vec();

        // Deserialize
        let loaded = Set::new(bytes).unwrap();
        assert!(loaded.contains("sword"));
        assert!(!loaded.contains("shield"));
    }
}
