//! The panel is the complement of what the table and the hand bar already make clickable, and this is where that line is held: every id a pending choice offers is drawn on exactly one of the two and never on both, a choice confined to the board leaves the sheet shut, a choice whose options the engine leaves implicit lights nothing up, and watching another seat choose offers this seat nothing. `Browser::wanted` and `RowStanding::selectable` are the two predicates under test. Where the sheet then stands, which tab it shows and how its rows are ordered are three other files.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_library_search_is_shown_where_the_board_cannot_show_it() {
    // Four cards the engine is *showing* the seat. They are in nobody's
    // graveyard and on no battlefield, so before the browser existed the
    // only thing on screen was the prompt.
    let shown: Vec<_> = (10..14).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2).with_looking_at(shown).build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: (10..14).map(obj).collect(),
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );

    assert!(Browser::wanted(&view, &it), "nothing else can draw these");
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(b.is_open());

    let rows = b.rows(&view, Some(&it), Names::projected());
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.zone == BrowseZone::Looking));
    assert!(
        rows.iter().all(|r| r.standing.selectable),
        "all four were offered"
    );
    assert!(rows.iter().all(|r| r.art.is_some()), "each has a picture");
}

/// The invariant the whole module exists for: an id the engine offered
/// is an id somebody draws. `BoardModel` covers the table and the hand,
/// `Browser` covers everything else, and the two are disjoint by
/// construction — which is why [`BrowseZone`] has no `Battlefield`.
#[test]
fn every_offered_object_is_drawn_somewhere() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_exile(1, vec![printed(5, 1, "Path to Exile", 4)])
        .with_command(0, vec![printed(6, 0, "Sisay", 5)])
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();

    let choices = [
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1), obj(3), obj(4), obj(5), obj(6), obj(7)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(4), obj(7)],
            min: 1,
            max: 2,
            prompt: ChoicePrompt::Generic,
        },
        Pending::LegendChoice {
            player: me(),
            options: vec![obj(1)],
        },
        Pending::OrderObjects {
            player: me(),
            objects: vec![obj(7), obj(4)],
        },
    ];

    let table = drawn_on_the_table(&view);
    for pending in choices {
        let it = Interaction::new(pending.clone(), me());
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        let rows = b.rows(&view, Some(&it), Names::projected());
        for id in it.selectable() {
            let on_table = table.contains(id);
            let in_tray = rows.iter().any(|r| r.id == *id && r.standing.selectable);
            assert!(
                on_table || in_tray,
                "{pending:?} offers {id:?} and nothing draws it"
            );
            assert!(
                !(on_table && in_tray),
                "{id:?} is drawn twice — the two models are meant to be disjoint"
            );
        }
    }
}

#[test]
fn a_choice_confined_to_the_table_leaves_the_tray_shut() {
    let view = ViewBuilder::new(2)
        .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let it = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(1), obj(2)],
            player_options: Vec::new(),
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    assert!(!Browser::wanted(&view, &it));
    let mut b = Browser::new();
    b.follow(&view, Some(&it));
    assert!(
        !b.is_open(),
        "a target on the board is clicked on the board"
    );
}

/// A discard leaves its options implicit — the engine means "your hand".
/// `is_selectable` says yes to anything for those, so a browser that
/// asked *that* question would offer every graveyard card as a discard.
#[test]
fn an_implicit_choice_does_not_light_up_the_whole_table() {
    let view = ViewBuilder::new(2)
        .with_hand(vec![("Lightning Bolt", 1, 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .build();
    let it = Interaction::new(
        Pending::DiscardChoice {
            player: me(),
            count: 1,
        },
        me(),
    );
    assert!(it.is_selectable(obj(4)), "the interaction accepts anything");
    assert!(
        !Browser::wanted(&view, &it),
        "but the hand is already drawn"
    );
    let b = Browser::new();
    assert!(
        b.rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| !r.standing.selectable),
        "a graveyard card is not a legal discard"
    );
}

#[test]
fn a_choice_for_another_seat_offers_nothing() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: PlayerId::new(1),
            options: vec![obj(7)],
            min: 1,
            max: 1,
            prompt: ChoicePrompt::Generic,
        },
        me(),
    );
    assert!(!Browser::wanted(&view, &it));
    assert!(
        Browser::new()
            .rows(&view, Some(&it), Names::projected())
            .iter()
            .all(|r| !r.standing.selectable),
        "watching another seat choose is not choosing"
    );
}
