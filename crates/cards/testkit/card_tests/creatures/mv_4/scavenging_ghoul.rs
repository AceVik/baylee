//! `cards/creatures/mv_4/scavenging_ghoul.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scavenging Ghoul — "At the beginning of **each** end step, put a corpse
/// counter on this creature **for each** creature that died this turn." /
/// "Remove a corpse counter from this creature: Regenerate this creature."
/// Both italicized words get their own witness here: the two creatures
/// that die are the opponent's, killed on the opponent's own turn (p1's),
/// not the Ghoul controller's (p0's) — so the counter must still land, and
/// it must land as 2, not 1.
#[test]
fn scavenging_ghoul_gathers_a_corpse_counter_and_spends_it_to_regenerate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                scavenging_ghoul(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[pearled_unicorn(), llanowar_elves()])
        .hand(0, &[hero_s_downfall(), hero_s_downfall()])
        .start();
    keep_mulligans(&mut engine);
    let ghoul = on_battlefield(&engine, p0, scavenging_ghoul()).expect("seated");
    assert_eq!(
        counters_on(&engine, ghoul, baylee_cards_dsl::counters::CORPSE),
        0
    );

    // p0's lands stand untapped through the whole of turn 1 (p0's own
    // turn): nothing is cast there, so both copies of Hero's Downfall wait
    // for p1's turn, cast at instant speed after the active player passes.
    reach_their_main_phase(&mut engine, p1);
    let unicorn = on_battlefield(&engine, p1, pearled_unicorn()).expect("seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hero_s_downfall());
    let _ = aim_at(&mut engine, p0, unicorn);
    cast_with_floating(&mut engine, p0, hero_s_downfall());
    let _ = aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, pearled_unicorn()).is_some(),
        "destroyed on p1's turn, not p0's"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "a second creature destroyed the same turn"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && e.state().turn.number >= 3
    });
    assert_eq!(
        counters_on(&engine, ghoul, baylee_cards_dsl::counters::CORPSE),
        2,
        "two creatures died on p1's turn; \"each end step\" fired on p1's turn too, \
         and \"for each\" counted both"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(ghoul, 1)),
        "the removal ability is offered while a counter sits on it: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, scavenging_ghoul(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ghoul).unwrap().regeneration_shields,
        1,
        "the removed counter paid for a regeneration shield"
    );
    assert_eq!(
        counters_on(&engine, ghoul, baylee_cards_dsl::counters::CORPSE),
        1,
        "one of the two counters spent, one left"
    );
}
