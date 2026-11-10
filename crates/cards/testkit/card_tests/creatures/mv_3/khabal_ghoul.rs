//! `cards/creatures/mv_3/khabal_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Khabál Ghoul — `{2}{B}` 1/1 Zombie: "At the beginning of each end step,
/// put a +1/+1 counter on this creature for each creature that died this
/// turn."
///
/// The count is all creatures put into graveyards from the battlefield
/// during the turn (Scryfall's ruling), so the board dies in three shapes:
/// a creature card of the Ghoul's controller's, a creature card of the
/// opponent's, and a Soldier token from Raise the Alarm. One of them dies
/// *before* the Ghoul is cast, which the ruling also says still counts. The
/// count is the turn's own: p1's end step, with no deaths on p1's turn,
/// adds nothing.
#[test]
fn khabal_ghoul_counts_every_creature_that_died_this_turn_tokens_included() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                swamp(),
                swamp(),
                swamp(),
                mountain(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[khabal_ghoul(), raise_the_alarm(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");

    // A death before the Ghoul is even cast still counts: the effect counts
    // what died this turn, not what the Ghoul watched. A Bolt of my own is
    // what kills the Elf, because a spell's controller holds priority again
    // in the same main phase once it resolves — a `kill` would hand the
    // phase on before the Ghoul could be cast.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected Lightning Bolt's target, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&my_elf),
        "my own Elf is a legal \"any target\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_elf],
                players: vec![],
            },
        )
        .expect("the Elf was one of the options it enumerated");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Bolt killed my own Elf before the Ghoul was cast"
    );

    tap_all_mana(&mut engine, p0);
    // The Alarm first: its {1}{W} leaves the three black sources for the
    // Ghoul's {2}{B}, while the other order lets the Ghoul's generic take
    // both Plains and strands the white spell.
    cast_with_floating(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, khabal_ghoul());
    pass_until(&mut engine, stack_is_empty);

    let ghoul = on_battlefield(&engine, p0, khabal_ghoul()).expect("the Ghoul resolved");
    assert_eq!(
        pt(&engine, ghoul),
        (1, 1),
        "the printed body, before the end step"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "Raise the Alarm made both Soldiers");

    kill(&mut engine, their_elf);
    kill(&mut engine, tokens[0]);

    pass_until(&mut engine, |e| {
        counters_on(e, ghoul, CounterKind::P1P1) > 0
    });
    assert_eq!(
        counters_on(&engine, ghoul, CounterKind::P1P1),
        3,
        "my Elf, their Elf and a Soldier token all died from the battlefield \
         this turn, whichever side of the table they were on"
    );
    assert_eq!(
        pt(&engine, ghoul),
        (4, 4),
        "one +1/+1 counter for each of the three"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, ghoul, CounterKind::P1P1),
        3,
        "no creature died on p1's turn, so the count for that end step is zero"
    );
}
