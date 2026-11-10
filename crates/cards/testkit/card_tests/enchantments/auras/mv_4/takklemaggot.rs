//! `cards/enchantments/auras/mv_4/takklemaggot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Takklemaggot ({2}{B}{B}):
/// "Enchant creature"
/// "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature."
/// "When enchanted creature dies, that creature's controller chooses a creature that this card could enchant. If the player does, return this card to the battlefield under your control attached to that creature. If they don't, return this card to the battlefield under your control as a non-Aura enchantment. It loses 'enchant creature' and gains 'At the beginning of that player's upkeep, this enchantment deals 1 damage to that player.'"
#[test]
fn takklemaggot_puts_minus_zero_minus_one_counters_reattaches_and_becomes_non_aura_ping_enchantment()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let takklemaggot = baylee_core::generated::index::TAKKLEMAGGOT;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .hand(0, &[takklemaggot])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2);
    let (elf1, elf2) = (elves[0], elves[1]);

    cast_from_hand(&mut engine, p0, takklemaggot);
    aim_at(&mut engine, p0, elf1);
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, takklemaggot).expect("Takklemaggot on battlefield");
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(elf1));

    // Advance to p1's upkeep: Takklemaggot puts a -0/-1 counter on elf1 (1/1 -> 1/0),
    // killing elf1. When the death trigger resolves, p1 must choose the remaining
    // creature (elf2) for Takklemaggot to enchant.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, .. } if *player == p1),
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected p1 to choose a creature host, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(options, vec![elf2]);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elf2],
            },
        )
        .expect("p1 chooses elf2 as the new host for Takklemaggot");

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(elf2),
        "Takklemaggot returned under p0's control attached to elf2"
    );

    // Advance through p0's turn to p1's next upkeep: elf2 gets a -0/-1 counter and dies.
    // With no creatures left on the battlefield, Takklemaggot returns as a non-Aura
    // enchantment that deals 1 damage to p1 at the beginning of p1's upkeep.
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let aura = on_battlefield(&engine, p0, takklemaggot)
        .expect("Takklemaggot returned to the battlefield as a non-Aura enchantment");
    assert_eq!(engine.state().object(aura).unwrap().attached_to, None);
    assert!(
        !engine
            .state()
            .object(aura)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA),
        "Takklemaggot is no longer an Aura"
    );

    // On p1's next upkeep, the non-Aura Takklemaggot deals 1 damage to p1.
    assert_eq!(engine.state().players[1].life, 20);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "non-Aura Takklemaggot dealt 1 damage to p1 at the beginning of p1's upkeep"
    );
}
