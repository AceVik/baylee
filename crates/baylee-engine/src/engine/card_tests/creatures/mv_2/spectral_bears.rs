//! Spectral Bears — `{1}{G}` 3/3 Creature — Bear Spirit (Homelands)
//!
//! Whenever this creature attacks, if defending player controls no black nontoken permanents, it
//! doesn't untap during your next untap step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn spectral_bears() -> CardIndex {
    card_index("539899f3-f3f2-4f7a-a6f6-e80d4a39f753")
}

fn sengir_vampire() -> CardIndex {
    card_index("749141aa-f6c4-4ad8-b146-406e68ae9b0b")
}

#[test]
fn spectral_bears_stays_tapped_when_defender_controls_no_black_nontoken_and_untaps_otherwise() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(1, &[sengir_vampire()])
        .battlefield(0, &[spectral_bears()])
        .battlefield(1, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, spectral_bears()).unwrap();

    // Attack p1 when p1 controls only colorless Swamps (no black nontoken permanent):
    // Spectral Bears' trigger fires and suppresses its next untap!
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);

    // On p1's turn, p1 casts Sengir Vampire (a black nontoken permanent).
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    let vamp_card = in_hand(&engine, p1, sengir_vampire()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: vamp_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to p0's next turn: Spectral Bears did NOT untap during this untap step!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, bears),
        "Spectral Bears stayed tapped during p0's next untap step because p1 controlled no black nontoken permanent when it attacked"
    );

    // Advance one more round to p0's following turn: now Spectral Bears untaps, and when it attacks p1 (who controls Sengir Vampire), the trigger does NOT fire!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(!is_tapped(&engine, bears));
}
