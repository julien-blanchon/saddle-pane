//! Benchmark: FST Levenshtein fuzzy search vs naive Levenshtein on a realistic dataset.
//!
//! This test fetches the `mdi` collection (~7400 icons) and compares:
//! - FST build time
//! - FST file size
//! - FST fuzzy search time (distance 1 and 2)
//! - Naive linear scan with Levenshtein distance
//!
//! Run: `cargo test -p bevy_iconify --test bench_suggest -- --nocapture`

use std::time::Instant;

/// Naive Levenshtein for comparison.
fn levenshtein(a: &str, b: &str) -> usize {
    let (a_len, b_len) = (a.len(), b.len());
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }
    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

fn fetch_mdi_icons() -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct CollectionResponse {
        #[serde(default)]
        uncategorized: Vec<String>,
        #[serde(default)]
        categories: std::collections::HashMap<String, Vec<String>>,
    }

    let url = "https://api.iconify.design/collection?prefix=mdi";
    let resp = ureq::get(url).call().expect("API reachable");
    let text = resp.into_string().unwrap();
    let col: CollectionResponse = serde_json::from_str(&text).unwrap();

    let mut icons = std::collections::HashSet::new();
    for name in &col.uncategorized {
        icons.insert(name.clone());
    }
    for names in col.categories.values() {
        for name in names {
            icons.insert(name.clone());
        }
    }
    let mut sorted: Vec<String> = icons.into_iter().collect();
    sorted.sort();
    sorted
}

#[test]
fn benchmark_fst_vs_naive() {
    use fst::automaton::Levenshtein as FstLevenshtein;
    use fst::{IntoStreamer, Set};

    let icons = fetch_mdi_icons();
    let n = icons.len();
    println!("\n=== Benchmark: {n} icons from mdi collection ===\n");

    // --- FST build ---
    let t = Instant::now();
    let set = Set::from_iter(icons.iter()).unwrap();
    let fst_build_us = t.elapsed().as_micros();
    let fst_bytes = set.as_fst().as_bytes().len();
    let raw_bytes: usize = icons.iter().map(|s| s.len()).sum();

    println!("FST build:        {fst_build_us} us");
    println!(
        "FST size:         {fst_bytes} bytes ({:.1} KB)",
        fst_bytes as f64 / 1024.0
    );
    println!(
        "Raw strings:      {raw_bytes} bytes ({:.1} KB)",
        raw_bytes as f64 / 1024.0
    );
    println!(
        "Compression:      {:.1}%",
        fst_bytes as f64 / raw_bytes as f64 * 100.0
    );

    // --- FST serialization round-trip ---
    let t = Instant::now();
    let serialized = set.as_fst().as_bytes().to_vec();
    let serialize_us = t.elapsed().as_micros();

    let t = Instant::now();
    let _loaded = Set::new(serialized).unwrap();
    let deserialize_us = t.elapsed().as_micros();

    println!("FST serialize:    {serialize_us} us");
    println!("FST deserialize:  {deserialize_us} us");

    // --- FST fuzzy search ---
    let queries = ["swrod", "hme", "arw-left", "chvron-right", "acount-circle"];

    println!("\n--- Fuzzy search (5 queries) ---\n");

    for query in &queries {
        // FST distance 1
        let t = Instant::now();
        let lev = FstLevenshtein::new(query, 1).unwrap();
        let results: Vec<String> = set.search(&lev).into_stream().into_strs().unwrap();
        let fst_d1_us = t.elapsed().as_micros();

        // FST distance 2
        let t = Instant::now();
        let lev = FstLevenshtein::new(query, 2).unwrap();
        let results_d2: Vec<String> = set.search(&lev).into_stream().into_strs().unwrap();
        let fst_d2_us = t.elapsed().as_micros();

        // Naive: compute distance for all icons, sort, take top 5
        let t = Instant::now();
        let mut scored: Vec<(usize, &str)> = icons
            .iter()
            .map(|icon| (levenshtein(query, icon), icon.as_str()))
            .collect();
        scored.sort_by_key(|(d, _)| *d);
        let naive_top5: Vec<&str> = scored.iter().take(5).map(|(_, s)| *s).collect();
        let naive_us = t.elapsed().as_micros();

        println!("  query: \"{query}\"");
        println!("    FST d=1: {fst_d1_us:>5} us  -> {results:?}");
        println!(
            "    FST d=2: {fst_d2_us:>5} us  -> {:?}",
            &results_d2[..results_d2.len().min(5)]
        );
        println!("    Naive:   {naive_us:>5} us  -> {naive_top5:?}");
        println!();
    }

    // --- Assertions ---
    // FST should be much smaller than raw data
    assert!(fst_bytes < raw_bytes, "FST should compress icon names");
    // FST should build in under 10ms for 7K keys
    assert!(
        fst_build_us < 10_000,
        "FST build took {fst_build_us} us, expected < 10,000 us"
    );
}
