//! The panel makes offered answers reachable when the table cannot: mixed choices repeat obscured battlefield answers in the sheet, other ordinary offers are disjoint, a choice confined to the board leaves the sheet shut, a choice whose options the engine leaves implicit lights nothing up, and watching another seat choose offers this seat nothing. `Browser::wanted` and `RowStanding::selectable` are the two predicates under test. Where the sheet then stands, which tab it shows and how its rows are ordered are three other files.

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
            total: None,
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
/// `Browser` covers everything else. A mixed choice additionally repeats its
/// obscured battlefield answers in the mandatory sheet; every other offered
/// object belongs to exactly one of those two surfaces.
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
            total: None,
        },
        Pending::LegendChoice {
            player: me(),
            options: vec![obj(1)],
        },
        put_back(vec![obj(7), obj(4)]),
    ];

    let table = drawn_on_the_table(&view);
    for pending in choices {
        let it = Interaction::new(pending.clone(), me());
        let mut b = Browser::new();
        b.follow(&view, Some(&it));
        let rows = b.rows(&view, Some(&it), Names::projected());
        assert_eq!(
            rows.len(),
            rows.iter()
                .map(|row| row.id)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            "the sheet must not repeat an offered object"
        );
        for id in it.selectable() {
            let on_table = table.contains(id);
            let in_tray = rows.iter().any(|r| r.id == *id && r.standing.selectable);
            assert!(
                on_table || in_tray,
                "{pending:?} offers {id:?} and nothing draws it"
            );
            let shared_battlefield_answer = b.for_choice()
                && matches!(
                    pending,
                    Pending::ChooseCards { .. } | Pending::ChooseTargets { .. }
                )
                && view.battlefield.iter().any(|object| object.id == *id);
            assert_eq!(
                on_table && in_tray,
                shared_battlefield_answer,
                "{id:?}: only obscured battlefield answers must appear on both surfaces"
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
            total: None,
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

#[test]
fn graveyard_target_chooser_hides_ineligible_cards_but_manual_browsing_keeps_them() {
    let view = ViewBuilder::new(2)
        .with_graveyard(
            0,
            vec![
                printed(1, 0, "Creature", 1),
                printed(2, 0, "Instant", 2),
                printed(3, 0, "Sorcery", 3),
            ],
        )
        .build();
    let it = Interaction::new(
        Pending::ChooseTargets {
            player: me(),
            options: vec![obj(2), obj(3)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    );
    let mut browser = Browser::new();
    browser.follow(&view, Some(&it));
    let rows = browser.rows(&view, Some(&it), Names::projected());
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![obj(2), obj(3)]
    );
    assert!(rows.iter().all(|r| r.standing.selectable));
    // After answering, reopening the pile is an ordinary inspection again.
    browser.follow(&view, None);
    browser.open_at(BrowseZone::Graveyard(me()));
    assert_eq!(browser.rows(&view, None, Names::projected()).len(), 3);
}

/// A pile choice names no object: it is answered by position, so none of the
/// revealed cards is selectable. They are still what the player chooses
/// between, and the sheet the reveal opened draws all of them; the rule that
/// a chooser lists only its answers had left it empty.
#[test]
fn a_pile_choice_draws_the_cards_in_its_piles() {
    let shown: Vec<_> = (20..23).map(|s| printed(s, 0, "Forest", 1)).collect();
    let view = ViewBuilder::new(2).with_looking_at(shown).build();
    let it = Interaction::new(
        Pending::ChoosePile {
            player: me(),
            piles: vec![vec![obj(21)], vec![obj(20), obj(22)]],
            label: None,
        },
        me(),
    );
    let mut b = Browser::new();
    b.saw_reveal(&view);
    b.follow(&view, Some(&it));
    assert!(
        b.is_open(),
        "the question arriving took away the sheet the reveal opened"
    );
    assert_eq!(ticks(&b), vec![BrowseZone::Looking]);
    let rows = b.rows(&view, Some(&it), Names::projected());
    assert_eq!(
        rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![obj(20), obj(21), obj(22)],
        "every card of every pile is on the sheet"
    );
    assert!(
        rows.iter().all(|r| !r.standing.selectable),
        "a pile is taken by its row in the prompt, not by clicking a card"
    );
}

/// Private inspection opens the offered hand without presenting checkboxes
/// as legal choices. Confirming is the only answer.
#[test]
fn a_hand_inspection_shows_every_card_without_making_it_selectable() {
    let view = ViewBuilder::new(2)
        .with_looking_at(vec![
            printed(10, 1, "Island", 1),
            printed(11, 1, "Sol Ring", 2),
        ])
        .build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(10), obj(11)],
            min: 0,
            max: 0,
            prompt: ChoicePrompt::LookAtHand,
            total: None,
        },
        me(),
    );
    assert!(Browser::wanted(&view, &it));
    let mut browser = Browser::new();
    browser.follow(&view, Some(&it));
    let rows = browser.rows(&view, Some(&it), Names::projected());
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter()
            .all(|row| row.zone == BrowseZone::Looking && !row.standing.selectable)
    );
    assert!(it.can_confirm());
}

/// Reverse Damage can offer a permanent and a spell in the same source question.
#[test]
fn mixed_battlefield_and_stack_card_choice_keeps_every_legal_source_reachable() {
    let view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            vec![printed(1, 0, "Power Leak", 1), printed(2, 0, "Island", 2)],
        )
        .with_stack(vec![printed(3, 0, "Reverse Damage", 3)])
        .build();
    let pending = Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(3)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    };
    let mut interaction = Interaction::new(pending.clone(), me());
    let mut browser = Browser::new();
    browser.follow(&view, Some(&interaction));
    assert!(browser.answers_here(Some(&interaction)));
    let rows = browser.rows(&view, Some(&interaction), Names::projected());
    assert_eq!(rows.len(), 2, "an unoffered Island is not a source option");
    assert!(
        rows.iter()
            .any(|row| row.id == obj(1) && row.zone == BrowseZone::Battlefield)
    );
    assert!(
        rows.iter()
            .any(|row| row.id == obj(3) && row.zone == BrowseZone::Stack)
    );
    for row in rows {
        assert!(row.standing.selectable);
        interaction.toggle(row.id);
        assert_eq!(pending.answer_fault(&interaction.confirm().unwrap()), None);
        interaction.cancel();
    }
    browser.follow(&view, None);
    assert!(!browser.zones(&view).contains(&BrowseZone::Battlefield));
    browser.open_at(BrowseZone::Stack);
    assert!(
        browser
            .rows(&view, None, Names::projected())
            .iter()
            .all(|row| row.id != obj(1))
    );
}

#[test]
fn compact_target_offers_are_complete_small_and_never_select_for_the_player() {
    let view = ViewBuilder::new(2)
        .with_stack((1..7).map(|id| printed(id, 0, "Spell", 1)).collect())
        .build();
    for (count, players, expected) in [
        (1, false, true),
        (4, false, true),
        (5, false, false),
        (1, true, false),
    ] {
        let mut view = view.clone();
        view.stack.truncate(count);
        let it = Interaction::new(
            Pending::ChooseTargets {
                player: me(),
                options: view.stack.iter().map(|o| o.id).collect(),
                player_options: if players { vec![me()] } else { vec![] },
                min: 1,
                max: 1,
                reason: TargetPrompt::Targets,
            },
            me(),
        );
        let mut browser = Browser::new();
        browser.follow(&view, Some(&it));
        assert_eq!(browser.compact_targets(&view, Some(&it)), expected);
        assert!(!it.can_confirm());
        assert!(it.selected().next().is_none());
    }
}

#[test]
fn a_filtered_private_inspection_displays_ineligible_cards_without_selecting_them() {
    let view = ViewBuilder::new(2)
        .with_looking_at(
            (10..16)
                .map(|id| printed(id, 0, "Inspected card", 1))
                .collect(),
        )
        .build();
    let it = Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: vec![obj(12)],
            min: 0,
            max: 1,
            prompt: ChoicePrompt::PutIntoHand,
            total: None,
        },
        me(),
    );
    let mut browser = Browser::new();
    browser.follow(&view, Some(&it));
    let rows = browser.rows(&view, Some(&it), Names::projected());
    assert_eq!(rows.len(), 6);
    let selectable: Vec<_> = rows
        .iter()
        .filter(|r| r.standing.selectable)
        .map(|r| r.id)
        .collect();
    assert_eq!(selectable, vec![obj(12)]);
}

/// One inspected library with fetchable lands, another land, an artifact,
/// an Ally and a non-Ally creature. The pending offer supplies legality.
fn tutor_library() -> PlayerView {
    use baylee_core::generated::subtypes::{creature, land};
    use baylee_core::types::SubtypeSet;

    let card = |slot, name, types, subtypes: &[baylee_core::ids::SubtypeId]| {
        let mut card = printed(slot, 0, name, 1);
        card.types = types;
        card.subtypes = SubtypeSet::from_slice(subtypes);
        if types != TypeSet::CREATURE {
            card.power = None;
            card.toughness = None;
        }
        card
    };
    ViewBuilder::new(2)
        .with_looking_at(vec![
            card(10, "Forest", TypeSet::LAND, &[land::FOREST]),
            card(11, "Plains", TypeSet::LAND, &[land::PLAINS]),
            card(12, "Savannah", TypeSet::LAND, &[land::FOREST, land::PLAINS]),
            card(13, "Island", TypeSet::LAND, &[land::ISLAND]),
            card(14, "Sol Ring", TypeSet::ARTIFACT, &[]),
            card(15, "Hada Freeblade", TypeSet::CREATURE, &[creature::ALLY]),
            card(16, "Grizzly Bears", TypeSet::CREATURE, &[creature::BEAR]),
        ])
        .build()
}

/// The engine's eligible ids and bounds; the browser must not infer either
/// from names, type text or how many inspected cards are in the view.
fn tutor_question(options: &[u32], min: u8) -> Interaction {
    Interaction::new(
        Pending::ChooseCards {
            player: me(),
            options: options.iter().copied().map(obj).collect(),
            min,
            max: u8::from(!options.is_empty()),
            prompt: ChoicePrompt::SearchLibrary,
            total: None,
        },
        me(),
    )
}

#[test]
fn fetch_and_tazri_search_rows_only_show_the_engine_offer() {
    use crate::interaction::SelectionOutcome;
    use baylee_engine::choice::PlayerAction;

    let view = tutor_library();
    // A Forest-or-Plains fetch includes the dual, but not Island. General
    // Tazri's offer contains the Ally, not just any creature in the library.
    for (offered, rejected) in [
        (vec![12, 11, 10], vec![13, 14, 15, 16]),
        (vec![15], vec![10, 11, 12, 13, 14, 16]),
    ] {
        let mut it = tutor_question(&offered, 0);
        let mut browser = Browser::new();
        browser.saw_reveal(&view);
        browser.follow(&view, Some(&it));
        assert!(browser.answers_here(Some(&it)));
        let rows = browser.rows(&view, Some(&it), Names::projected());
        let mut expected: Vec<_> = offered.iter().copied().map(obj).collect();
        expected.sort_unstable();
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), expected);
        assert_eq!(
            it.selectable(),
            offered.iter().copied().map(obj).collect::<Vec<_>>()
        );
        assert!(rows.iter().all(|row| row.standing.selectable));
        assert_eq!(view.looking_at.len(), 7, "inspection data is not rewritten");
        for id in rejected {
            assert_eq!(it.toggle(obj(id)), SelectionOutcome::Rejected);
        }
        // The rendered row carries its object id, not its filtered/sorted
        // index. Click those rows: every wire answer must retain that id.
        for row in rows {
            assert_eq!(it.toggle(row.id), SelectionOutcome::Added);
            assert_eq!(
                it.confirm(),
                Some(PlayerAction::ChooseObjects {
                    objects: vec![row.id]
                })
            );
            assert_eq!(it.toggle(row.id), SelectionOutcome::Removed);
        }
        // Typing an ineligible card's name must not bring it back into the
        // chooser: the text filter only narrows the engine's legal offer.
        browser.set_filter("Sol Ring");
        assert!(
            browser
                .rows(&view, Some(&it), Names::projected())
                .is_empty()
        );
    }
}

#[test]
fn tutor_rows_preserve_fail_to_find_and_mandatory_search_bounds() {
    use baylee_engine::choice::PlayerAction;

    let view = tutor_library();
    for offered in [vec![10, 11, 12], vec![15], vec![]] {
        let it = tutor_question(&offered, 0);
        let mut browser = Browser::new();
        browser.follow(&view, Some(&it));
        assert!(browser.is_open());
        assert!(browser.answers_here(Some(&it)));
        assert_eq!(
            browser.rows(&view, Some(&it), Names::projected()).len(),
            offered.len()
        );
        assert!(it.can_confirm());
        assert_eq!(
            it.confirm(),
            Some(PlayerAction::ChooseObjects { objects: vec![] })
        );
    }
    // No client-side relaxation of the engine's minimum, even when a text
    // filter leaves the dialog with no visible cards.
    let mut required = tutor_question(&[15], 1);
    let mut browser = Browser::new();
    browser.follow(&view, Some(&required));
    browser.set_filter("no matching name");
    assert!(
        browser
            .rows(&view, Some(&required), Names::projected())
            .is_empty()
    );
    assert!(!required.can_confirm());
    assert_eq!(required.confirm(), None);
    browser.clear_filter();
    required.toggle(obj(15));
    assert_eq!(
        required.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(15)]
        })
    );
}

#[test]
fn tutor_rows_follow_changed_offers_and_keep_unrestricted_searches_complete() {
    let view = tutor_library();
    let mut browser = Browser::new();
    let any_card = tutor_question(&(10..17).collect::<Vec<_>>(), 1);
    browser.follow(&view, Some(&any_card));
    assert_eq!(
        browser
            .rows(&view, Some(&any_card), Names::projected())
            .len(),
        7
    );

    // Even another Forest is hidden when its id is absent from a new offer.
    // The browser follows legality, rather than rebuilding a type predicate.
    let narrowed = tutor_question(&[12], 0);
    browser.follow(&view, Some(&narrowed));
    let rows = browser.rows(&view, Some(&narrowed), Names::projected());
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![obj(12)]
    );
    assert_eq!(narrowed.selectable(), &[obj(12)]);

    // A reveal/inspection without a search still presents every shown card.
    let mut inspection = Browser::new();
    inspection.saw_reveal(&view);
    assert_eq!(inspection.rows(&view, None, Names::projected()).len(), 7);
    inspection.open_at(BrowseZone::Looking);
    assert_eq!(inspection.rows(&view, None, Names::projected()).len(), 7);
}
