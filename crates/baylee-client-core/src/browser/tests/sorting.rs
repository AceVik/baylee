//! The order the rows come out in. The pile's own order is the default and is information - it is what "the top card of your graveyard" means - a name or a mana value sort is stable and breaks its ties on that same pile order rather than on whatever the previous sort left behind, reversing reverses the key without reversing the tie-break, a new key starts the right way up, and no key may interleave two zones, because the tabs are the panel's first structure. Which rows exist at all is the filter's and the tabs' business; how one is drawn is the sheet's.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// The graveyard's own order is the default and is information: it is
/// what "the top card of your graveyard" means.
#[test]
fn the_default_order_is_the_pile_s_own_and_survives_being_reversed() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(4, 0, "Zealous Persecution", 3),
                printed(5, 0, "Ancestral Vision", 4),
                printed(6, 0, "Mox Diamond", 5),
            ],
        )
        .build();
    let mut b = Browser::new();
    assert_eq!(b.sort(), SortKey::Place);
    let names = |b: &Browser| -> Vec<String> {
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|r| r.name)
            .collect()
    };
    assert_eq!(
        names(&b),
        [
            "Zealous Persecution".to_string(),
            "Ancestral Vision".to_string(),
            "Mox Diamond".to_string()
        ],
        "the pile was re-ordered with no sort asked for"
    );

    b.reverse();
    assert_eq!(
        names(&b),
        [
            "Mox Diamond".to_string(),
            "Ancestral Vision".to_string(),
            "Zealous Persecution".to_string()
        ],
        "reversing the pile order did not reverse it"
    );
}

#[test]
fn sorting_by_name_and_by_cost_are_both_stable_and_reversible() {
    let mut cheap = printed(4, 0, "Zealous Persecution", 3);
    cheap.mana_value = 2;
    let mut dear = printed(5, 0, "Ancestral Vision", 4);
    dear.mana_value = 9;
    let mut also_cheap = printed(6, 0, "Mox Diamond", 5);
    also_cheap.mana_value = 2;
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![cheap, dear, also_cheap])
        .build();
    let mut b = Browser::new();
    let names = |b: &Browser| -> Vec<String> {
        b.rows(&view, None, Names::projected())
            .into_iter()
            .map(|r| r.name)
            .collect()
    };

    b.sort_by(SortKey::Name);
    assert_eq!(
        names(&b),
        [
            "Ancestral Vision".to_string(),
            "Mox Diamond".to_string(),
            "Zealous Persecution".to_string()
        ]
    );

    b.sort_by(SortKey::ManaValue);
    assert_eq!(
        names(&b),
        [
            // Two twos, and the tie is broken by the pile's own order —
            // never by whatever the previous sort happened to leave.
            "Zealous Persecution".to_string(),
            "Mox Diamond".to_string(),
            "Ancestral Vision".to_string()
        ]
    );
    b.reverse();
    assert_eq!(
        names(&b),
        [
            "Ancestral Vision".to_string(),
            "Zealous Persecution".to_string(),
            "Mox Diamond".to_string()
        ],
        "descending reversed the tie-break as well as the key"
    );
}

/// Whatever the key, the tabs are the panel's first structure.
#[test]
fn a_sort_never_interleaves_two_zones() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Mox Diamond", 3)])
        .with_exile(0, vec![printed(5, 0, "Ancestral Vision", 4)])
        .build();
    let mut b = Browser::new();
    b.sort_by(SortKey::Name);
    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].zone,
        BrowseZone::Graveyard(PlayerId::new(0)),
        "the exile card sorted ahead of the graveyard it is not in"
    );
    assert_eq!(rows[1].zone, BrowseZone::Exile(PlayerId::new(0)));
}

/// The cycle is one control, so a direction must not survive a key change.
#[test]
fn cycling_the_sort_key_starts_it_the_right_way_up() {
    let mut b = Browser::new();
    b.reverse();
    assert!(b.descending());
    b.cycle_sort();
    assert_eq!(b.sort(), SortKey::Name);
    assert!(!b.descending(), "descending carried into a new key");
}
