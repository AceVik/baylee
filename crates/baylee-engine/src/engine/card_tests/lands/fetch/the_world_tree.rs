//! `cards/lands/fetch/the_world_tree.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The World Tree: "This land enters tapped." / "{T}: Add {G}."
/// Playing this land causes it to enter tapped, and after untapping on a
/// subsequent turn it taps for green mana.
#[test]
fn the_world_tree_enters_tapped_and_taps_for_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(211, forest())
        .hand(0, &[the_world_tree()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_world_tree());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, the_world_tree(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert!(is_tapped(&engine, land));
}

/// "{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}, {T}, Sacrifice this land: Search your
/// library for any number of God cards, put them onto the battlefield, then
/// shuffle."
///
/// Two Gods and a creature that is not one in the library: the search
/// offers the two Gods and nothing else, may settle for none, may take
/// both, and puts both onto the battlefield; the Tree is sacrificed.
#[test]
fn the_world_tree_finds_any_number_of_gods() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(213, forest())
        .battlefield(
            0,
            &[
                the_world_tree(),
                plains(),
                plains(),
                island(),
                island(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                forest(),
                forest(),
            ],
        )
        .hand(
            0,
            &[
                ojer_kaslem_deepest_growth(),
                ojer_taq_deepest_foundation(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let moves: Vec<ObjectId> = [
        ojer_kaslem_deepest_growth(),
        ojer_taq_deepest_foundation(),
        llanowar_elves(),
    ]
    .into_iter()
    .map(|card| in_hand(&engine, p0, card).expect("dealt into the hand"))
    .collect();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for card in moves {
            state
                .move_object(
                    card,
                    ZoneLocation::Library(p0),
                    ZonePosition::Top,
                    crate::event::Cause::DevCommand,
                )
                .expect("into the library");
        }
    }
    engine.refresh_offer();

    let tree = on_battlefield(&engine, p0, the_world_tree()).expect("the Tree is out");
    tap_mana_except(&mut engine, p0, tree);
    activate(&mut engine, p0, the_world_tree(), 2);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched");
    };
    let offered: Vec<CardIndex> = options
        .iter()
        .filter_map(|id| engine.state().object(*id).and_then(|o| o.card))
        .map(|c| c.index)
        .collect();
    assert_eq!(
        offered.len(),
        2,
        "the two Gods and nothing else: {offered:?}"
    );
    assert!(offered.contains(&ojer_kaslem_deepest_growth()));
    assert!(offered.contains(&ojer_taq_deepest_foundation()));
    assert_eq!(
        (min, max),
        (0, 2),
        "any number: none, or as many as there are"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: options })
        .expect("both Gods");
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, ojer_kaslem_deepest_growth()).is_some());
    assert!(on_battlefield(&engine, p0, ojer_taq_deepest_foundation()).is_some());
    assert!(
        in_graveyard(&engine, p0, the_world_tree()).is_some(),
        "the Tree was sacrificed"
    );
}
