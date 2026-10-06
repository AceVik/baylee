//! `cards/lands/utility/volrath_s_stronghold.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Volrath's Stronghold charges `{1}{B}` beside its `{T}`, and the proof is
/// that `{B}` alone does not buy it.
///
/// This card is the reason the price is worth a test rather than a glance.
/// Its `//! Oracle:` header had dropped the mana from the printed sentence
/// and the code was written from the header, so for as long as it existed it
/// was a *free*, repeatable recursion of any creature in the graveyard —
/// which is a different card, and one that reads as correct from every side
/// except the printing. `xtask validate` closed that hole by comparing the
/// header against Scryfall; this closes the other one, which is that nothing
/// played it.
///
/// One Swamp against two, and the offer is the answer: with a single black
/// floating the ability must not appear, and the second one is the whole
/// difference.
#[test]
fn volrath_s_stronghold_is_not_offered_one_mana_short() {
    for (pass, swamps) in [1usize, 2].into_iter().enumerate() {
        let p0 = PlayerId::new(0);
        let seed = 4_830 + u64::try_from(pass).expect("two passes");
        let mut board = vec![volrath_s_stronghold(), ignoble_hierarch()];
        board.extend(std::iter::repeat_n(swamp(), swamps));
        let mut engine = Duel::new(seed, forest()).battlefield(0, &board).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        // A creature card in the graveyard, put there the way the board puts
        // one there: the library is Forests, so seeding would give a land and
        // the filter has to have something it could wrongly accept.
        let hierarch =
            on_battlefield(&engine, p0, ignoble_hierarch()).expect("the creature is on the table");
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        crate::sba::destroy(state, hierarch);
        engine.refresh_offer();

        let land = on_battlefield(&engine, p0, volrath_s_stronghold())
            .expect("the Stronghold is on the table");
        tap_mana_except(&mut engine, p0, land);

        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        let offered = legal.abilities.contains(&(land, 1));
        assert_eq!(
            offered,
            swamps == 2,
            "with {swamps} Swamp(s) tapped the recursion was {} offered: {:?}",
            if offered { "wrongly" } else { "wrongly not" },
            legal.abilities
        );
        if !offered {
            continue;
        }

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 1,
                },
            )
            .expect("an offered ability must be payable");
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("expected a target choice, got {:?}", engine.pending())
        };
        let card = in_graveyard(&engine, p0, ignoble_hierarch())
            .expect("the creature card is in my graveyard");
        assert!(
            options.contains(&card),
            "a creature card in your own graveyard is what it finds: {options:?}"
        );

        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![card],
                },
            )
            .expect("a card the menu named is a legal answer");
        pass_until(&mut engine, stack_is_empty);
        assert!(
            in_graveyard(&engine, p0, ignoble_hierarch()).is_none(),
            "the card left the graveyard"
        );
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            0,
            "the {{1}}{{B}} was spent"
        );

        // "On top of your library" is asserted by *drawing* it rather than by
        // indexing the zone. Which end of that `Vec` is the top is the
        // library's business and not this test's, and a test that guessed
        // would pass or fail on the guess; the next draw step cannot be
        // wrong about it.
        cross_into_the_next_own_main(&mut engine, p0);
        assert!(
            in_hand(&engine, p0, ignoble_hierarch()).is_some(),
            "the card put on top was the next card drawn"
        );
    }
}
