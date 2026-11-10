//! `cards/creatures/mv_4/uncle_istvan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn uncle_istvan() -> CardIndex {
    card_index("44e855a4-95f9-4515-a868-a8ad7a2db751")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn float(engine: &mut Engine<RegistryLookup>, seat: usize, mana: &[(ManaColor, u32)]) {
    let state = engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness may set boards up");
    for (color, n) in mana {
        state.players[seat].mana_pool.add(*color, *n);
    }
    engine.refresh_offer();
}

/// Uncle Istvan (44e855a4-95f9-4515-a868-a8ad7a2db751) — "Prevent all damage
/// that would be dealt to this creature by creatures."
///
/// A 1/3 blocks a 3/3 Hill Giant: it takes nothing and deals its own damage.
#[test]
fn uncle_istvan_takes_no_combat_damage_from_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[uncle_istvan()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    let istvan = on_battlefield(&engine, p0, uncle_istvan()).expect("Istvan is out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    reach_their_main_phase(&mut engine, p1);

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    assert!(
        attackers.contains(&giant),
        "the Giant may attack: {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p0))],
            },
        )
        .expect("the attack is legal");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(istvan, giant)],
            },
        )
        .expect("Istvan blocks the Giant");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);

    assert_eq!(
        engine.state().object(istvan).expect("Istvan lives").damage,
        0,
        "the Giant's 3 damage was prevented"
    );
    assert_eq!(
        engine
            .state()
            .object(giant)
            .expect("the Giant lives")
            .damage,
        1,
        "Istvan's own damage is not prevented"
    );
    assert_eq!(engine.state().players[0].life, 20, "blocked: none through");
}

/// Only damage by *creatures* is prevented: a Lightning Bolt kills it.
#[test]
fn uncle_istvan_still_dies_to_a_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[uncle_istvan()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let istvan = on_battlefield(&engine, p0, uncle_istvan()).expect("Istvan is out");
    float(&mut engine, 0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, istvan);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, uncle_istvan()).is_some(),
        "3 damage from a spell kills the 1/3"
    );
}
