//! Which searched runs a dataset may be made of (`--search-advantage`).

#[cfg(feature = "onnx")]
use std::path::{Path, PathBuf};

#[cfg(feature = "onnx")]
use anyhow::ensure;
#[cfg(feature = "onnx")]
use baylee_train_ops::search_advantage::AFTER_PER_ROOT;
use serde_json::Value;
#[cfg(feature = "onnx")]
use serde_json::json;

/// The worlds of a fair search, as a run's `search.decks` names them
/// (`netplay4::search`'s metadata): every card the seat cannot see is dealt
/// from what it may suppose. Any other search deals from lists the seat does
/// not know, and what it finds is nothing a seat could have found.
pub(super) const FAIR_DECKS: &str =
    "public_prior_from_the_seats_view_and_own_list_never_another_players_list";

/// Whether a search played fair worlds.
pub(super) fn fair(search: &Value) -> bool {
    search["decks"] == FAIR_DECKS
}

/// The searched runs of a search-advantage dataset.
#[cfg(feature = "onnx")]
pub(super) struct SearchSources {
    /// Whether the dataset gets its roots' afterstates.
    afterstates: bool,
    /// The search every run of the dataset was played with, and the first run
    /// that named it.
    search: Option<(PathBuf, Value)>,
    sources: Vec<Value>,
}

#[cfg(feature = "onnx")]
impl SearchSources {
    pub(super) const fn new(afterstates: bool) -> Self {
        Self {
            afterstates,
            search: None,
            sources: Vec::new(),
        }
    }

    /// Admits one run, `decks` the number of train decks it can seat: a
    /// searched one that is data, whose worlds are fair, played with the
    /// search of every other run, and whose roots an afterstate frame's
    /// cursor has room for where the dataset gets them.
    ///
    /// # Errors
    /// A run that is not.
    pub(super) fn admit(
        &mut self,
        run_dir: &Path,
        run: &Value,
        decks: usize,
    ) -> anyhow::Result<()> {
        let search = &run["search"];
        ensure!(
            search.is_object(),
            "--search-advantage reads searched runs only"
        );
        ensure!(
            run["numbers_only"] != true,
            "--search-advantage refuses a run for numbers only"
        );
        ensure!(
            fair(search),
            "--search-advantage reads fair searches only (eval4 --search-fair): this run's worlds \
             were dealt from {}",
            search["decks"]
        );
        ensure!(
            !self.afterstates
                || search["candidates"]
                    .as_u64()
                    .is_none_or(|k| k <= u64::from(AFTER_PER_ROOT)),
            "--search-afterstates has room for {AFTER_PER_ROOT} candidates a root; this run \
             searched {}",
            search["candidates"]
        );
        match &self.search {
            None => self.search = Some((run_dir.to_path_buf(), search.clone())),
            Some((first, shared)) => ensure!(
                shared == search,
                "one search-advantage dataset holds one search; this run's differs from {}'s in {}",
                first.display(),
                differing(shared, search).join(", ")
            ),
        }
        self.sources
            .push(json!({"run": run_dir, "search": search["contract"],
            "search_settings": search, "collection_build": run["evaluator_build"],
            "collection_revision": run["evaluator_revision"],
            "collection_dirty": run["evaluator_dirty"], "decks": decks}));
        Ok(())
    }

    /// The runs, as the dataset names them.
    pub(super) fn metadata(&self) -> &[Value] {
        &self.sources
    }
}

/// The settings two searches differ in.
#[cfg(feature = "onnx")]
fn differing(a: &Value, b: &Value) -> Vec<String> {
    let names = |value: &Value| {
        value
            .as_object()
            .into_iter()
            .flat_map(|object| object.keys().cloned())
            .collect::<std::collections::BTreeSet<_>>()
    };
    names(a)
        .union(&names(b))
        .filter(|name| a[name.as_str()] != b[name.as_str()])
        .cloned()
        .collect()
}

#[cfg(all(test, feature = "onnx"))]
mod tests {
    use super::*;

    fn run(search: &Value) -> Value {
        json!({"numbers_only": false, "evaluator_build": "b", "search": search})
    }

    fn search(candidates: u64) -> Value {
        json!({"contract": "belief_lookahead_v5", "candidates": candidates, "particles": 4,
            "decks": FAIR_DECKS})
    }

    fn admit(afterstates: bool, run: &Value) -> anyhow::Result<()> {
        SearchSources::new(afterstates).admit(Path::new("runs/r"), run, 2)
    }

    #[test]
    fn a_search_advantage_run_is_fair_searched_data_whose_roots_fit() {
        admit(false, &run(&search(32))).unwrap();
        let mut unsaid = run(&search(8));
        unsaid["numbers_only"] = Value::Null;
        admit(true, &unsaid).unwrap();
        let mut numbers = run(&search(4));
        numbers["numbers_only"] = json!(true);
        assert!(admit(false, &numbers).is_err());
        assert!(admit(true, &run(&search(9))).is_err());
        assert!(admit(false, &json!({"numbers_only": false})).is_err());
        // Worlds from a list the seat does not know, and a run that does not
        // say where its worlds are from.
        let mut unfair = search(4);
        unfair["decks"] = json!("known_to_the_harness");
        let error = admit(false, &run(&unfair)).unwrap_err().to_string();
        assert!(error.contains("known_to_the_harness"), "{error}");
        unfair.as_object_mut().unwrap().remove("decks");
        assert!(admit(false, &run(&unfair)).is_err());
    }

    /// The word a fair search's own metadata has for its worlds.
    #[test]
    fn the_fair_worlds_are_named_as_the_search_names_them() {
        use baylee_train::netplay4::search::SearchConfig4;
        let config = SearchConfig4::new(4, 4, 8).unwrap();
        assert!(!fair(&config.metadata()));
        assert!(fair(&config.with_fair(true).metadata()));
    }

    #[test]
    fn a_dataset_holds_one_search() {
        let mut sources = SearchSources::new(false);
        sources
            .admit(Path::new("runs/a"), &run(&search(4)), 2)
            .unwrap();
        sources
            .admit(Path::new("runs/b"), &run(&search(4)), 3)
            .unwrap();
        let mut other = search(4);
        other["particles"] = json!(8);
        other["turns"] = json!(2);
        let error = sources
            .admit(Path::new("runs/c"), &run(&other), 2)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("runs/a") && error.contains("particles, turns"),
            "{error}"
        );
        assert_eq!(sources.sources.len(), 2);
        assert_eq!(sources.sources[1]["search_settings"], search(4));
    }
}
