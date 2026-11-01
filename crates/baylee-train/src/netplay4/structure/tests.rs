//! Binding an exported catalog to this build's, on small real catalogs.
use super::*;
use crate::cardstructure::catalog;

fn index(name: &str) -> u32 {
    baylee_cards::all()
        .find(|c| c.name() == name)
        .unwrap_or_else(|| panic!("{name} is a card"))
        .index
        .get()
}

/// Three cards by ascending index: the first is the one a test adds, so
/// every node this build compiles for the other two moves.
fn three() -> [u32; 3] {
    let mut cards = [index("Mana Leak"), index("Negate"), index("Remove Soul")];
    cards.sort_unstable();
    cards
}

/// A catalog as an exported model reads it back: serialized and parsed.
fn exported(cards: &[u32]) -> Forest {
    let forest = catalog(cards).unwrap();
    serde_json::from_slice(&serde_json::to_vec(&forest).unwrap()).unwrap()
}

/// `exported(cards)` with every key of card `from` renamed to card `to`:
/// the model learned `to` with `from`'s rules, as if `to` changed since.
fn exported_with_changed(cards: &[u32], from: u32, to: u32) -> Forest {
    let mut forest = exported(cards);
    forest.roots = std::mem::take(&mut forest.roots)
        .into_iter()
        .map(|(key, node)| {
            let mut parts: Vec<String> = key.split('/').map(str::to_owned).collect();
            if compat::owner(&key) == format!("card/{from}") {
                parts[1] = to.to_string();
            }
            (parts.join("/"), node)
        })
        .collect();
    forest
}

fn refs(keys: &[&str]) -> Vec<RuleRef> {
    keys.iter()
        .map(|key| RuleRef {
            key: (*key).to_owned(),
            role: 1,
            slot: None,
            ability: None,
        })
        .collect()
}

fn bound(model: Forest, current: &Forest, policy: CatalogPolicy4) -> anyhow::Result<RuleRoots> {
    let expected = digest(&model).unwrap();
    bind(model, current, &expected, policy)
}

#[test]
fn the_same_catalog_binds_under_every_policy_and_leaves_nothing_out() {
    let cards = [index("Mana Leak"), index("Negate")];
    for policy in [
        CatalogPolicy4::Exact,
        CatalogPolicy4::Superset,
        CatalogPolicy4::Drift,
    ] {
        let roots = bound(exported(&cards), &catalog(&cards).unwrap(), policy).unwrap();
        assert!(roots.binding.identical);
        assert!(roots.unknown.is_empty() && roots.binding.withdrawn.is_empty());
    }
}

#[test]
fn an_added_card_is_left_out_and_known_cards_keep_the_models_nodes() {
    let [negate, leak, soul] = three();
    let model = exported(&[leak, soul]);
    let model_roots = model.roots.clone();
    let current = catalog(&[leak, negate, soul]).unwrap();
    assert!(
        bound(exported(&[leak, soul]), &current, CatalogPolicy4::Exact).is_err(),
        "a different catalog needs an explicit policy"
    );
    let roots = bound(model, &current, CatalogPolicy4::Superset).unwrap();
    assert!(!roots.binding.identical);
    assert!(roots.binding.new_roots > 0 && roots.binding.withdrawn.is_empty());
    assert_eq!(
        (
            roots.binding.appended_cards,
            roots.binding.appended_tokens,
            roots.binding.changed_roots
        ),
        (1, 0, 0)
    );

    let soul_key = format!("c/{soul}/0");
    let rows = vec![
        refs(&[&soul_key, &format!("i/{soul}/0")]),
        refs(&[&format!("c/{negate}/0"), &format!("i/{negate}/0")]),
        refs(&[&format!("c/{leak}/0"), &format!("c/{negate}/0")]),
    ];
    let (width, values, left_out) = pack(&rows, 4, &roots).unwrap();
    assert_eq!((width, left_out), (2, 3));
    // A known key reads the model's own node, wherever this build put it.
    assert_eq!(values[0], i64::from(model_roots[&soul_key]));
    assert_ne!(current.roots[&soul_key], model_roots[&soul_key]);
    // The added card's row is empty, like an unknown card's.
    assert!(values[width * 2..width * 4].iter().all(|&v| v == 0));
    assert_eq!(
        values[width * 4],
        i64::from(model_roots[&format!("c/{leak}/0")])
    );
    assert_eq!(
        values[width * 4 + 2],
        0,
        "the added reference leaves no gap"
    );

    let stray = vec![refs(&["c/4000000000/0"])];
    assert!(
        pack(&stray, 1, &roots).is_err(),
        "a key neither catalog has is still an error"
    );
}

#[test]
fn a_changed_card_is_refused_or_withdrawn_whole() {
    let [negate, leak, soul] = three();
    // The model learned Negate with Remove Soul's rules.
    let model = || exported_with_changed(&[leak, soul], soul, negate);
    let current = catalog(&[leak, negate]).unwrap();
    let error = bound(model(), &current, CatalogPolicy4::Superset).unwrap_err();
    assert!(error.to_string().contains("changed"), "{error}");

    let roots = bound(model(), &current, CatalogPolicy4::Drift).unwrap();
    let withdrawn = &roots.binding.withdrawn;
    assert_eq!(withdrawn.len(), 1);
    assert_eq!(withdrawn[0].owner, format!("card/{negate}"));
    let name = baylee_cards::by_index(baylee_core::ids::CardIndex::new(negate))
        .map(baylee_cards_dsl::CardDef::name);
    assert_eq!(withdrawn[0].name.as_deref(), name);
    assert!(name.is_some());
    assert!(roots.binding.changed_roots > 0);
    let rows = vec![
        refs(&[&format!("c/{negate}/0"), &format!("i/{negate}/0")]),
        refs(&[&format!("c/{leak}/0")]),
    ];
    let (_, values, left_out) = pack(&rows, 2, &roots).unwrap();
    assert_eq!(
        left_out, 2,
        "every key of the changed card, identity included"
    );
    assert!(values[..2].iter().all(|&v| v == 0));
}

#[test]
fn the_comparison_matches_trees_not_node_numbers() {
    let [negate, leak, soul] = three();
    let report = compat::superset(
        &exported(&[leak, soul]),
        &catalog(&[negate, soul, leak]).unwrap(),
    );
    assert!(report.holds(), "{report:?}");
    assert!(
        report
            .new_roots
            .iter()
            .all(|key| compat::owner(key) == format!("card/{negate}"))
    );
    let report = compat::superset(
        &exported_with_changed(&[leak, soul], soul, negate),
        &catalog(&[leak, negate]).unwrap(),
    );
    assert!(!report.holds());
    assert!(
        report
            .changed
            .iter()
            .all(|key| compat::owner(key) == format!("card/{negate}"))
    );
}
