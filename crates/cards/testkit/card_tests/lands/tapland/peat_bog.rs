//! `cards/lands/tapland/peat_bog.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Peat Bog's whole life: "{T}, Remove a depletion counter from this land:
/// Add {B}{B}. If there are no depletion counters on this land, sacrifice
/// it."
///
/// Two mana twice and then the land is gone, which is three things at once
/// and they are three different mechanisms. The cost takes a counter off
/// (arithmetic, no chooser). The first effect makes the mana. The second
/// effect reads the counters the cost just spent and, on the second
/// activation only, sacrifices the source — an ordinary effect in the same
/// list, because that is how the card prints it, and not a state-based
/// action that would fire somewhere else entirely.
///
/// What the order of the two effects does **not** decide is the mana: the
/// pool belongs to the player and not to the land, so a sacrifice running
/// first would still leave {B}{B} behind. Gemstone Mine is where the order
/// is observable, and that is the test below.
#[test]
fn a_depletion_land_pays_twice_and_the_second_payment_kills_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(552, forest()).hand(0, &[peat_bog()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, peat_bog());
    assert!(entered_tapped(&engine, land), "it arrives tapped");

    for turn in 0..2u16 {
        cross_into_the_next_own_main(&mut engine, p0);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority, got {:?}", engine.pending())
        };
        assert!(
            legal.abilities.contains(&(land, 0)),
            "turn {turn}: {} counter(s) left, so the line is payable: {:?}",
            counters_on(&engine, land, counters::DEPLETION),
            legal.abilities
        );

        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: land,
                    ability_index: 0,
                },
            )
            .expect("a counter is there to pay with");

        // {B}{B} names its colour, so nothing is asked and the whole
        // ability — cost, mana and the clause after it — is over already.
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "turn {turn}: a fixed colour asks nobody anything: {:?}",
            engine.pending()
        );
        assert_eq!(
            engine.state().players[0]
                .mana_pool
                .available(ManaColor::Black),
            2,
            "turn {turn}: two black, which is what the land prints"
        );
    }

    assert!(
        in_graveyard(&engine, p0, peat_bog()).is_some(),
        "the second activation left no depletion counters, so the same \
         ability that made the mana sacrificed the land"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == land),
        "and a land in a graveyard is offered nothing: {:?}",
        legal.abilities
    );
}
