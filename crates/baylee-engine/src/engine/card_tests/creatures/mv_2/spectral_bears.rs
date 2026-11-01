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

fn pest_summoning() -> CardIndex {
    card_index("f6e0cb77-4115-4948-8e1e-fdb774b1b254")
}

/// P0 attacks P1 with the Bears alone, P1 does not block, and combat ends.
fn attack_with_the_bears(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bears = on_battlefield(engine, p0, spectral_bears()).unwrap();
    pass_until(engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    bears
}

/// Runs to the first main phase of P0's next turn, the one after the untap step in question.
fn to_p0_next_main_phase(engine: &mut Engine<RegistryLookup>) {
    let p0 = PlayerId::new(0);
    pass_until(engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
}

#[test]
fn spectral_bears_stays_tapped_when_defender_controls_no_black_nontoken_permanent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // The defender controls a green creature and a land, neither black. The attacker's own black
    // Sengir Vampire is not the defending player's permanent and does not count.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spectral_bears(), sengir_vampire()])
        .battlefield(1, &[grizzly_bears(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = attack_with_the_bears(&mut engine);
    assert_eq!(engine.state().players[1].life, 17);
    assert!(is_tapped(&engine, bears), "the Bears attacked");

    // P0's next untap step: the Bears did not untap.
    to_p0_next_main_phase(&mut engine);
    assert!(
        is_tapped(&engine, bears),
        "the defender controlled no black nontoken permanent when the Bears attacked, so they stay tapped"
    );

    // "Next untap step" is one untap step: the one after that untaps them again.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    to_p0_next_main_phase(&mut engine);
    assert!(!is_tapped(&engine, bears));
}

#[test]
fn spectral_bears_untaps_normally_when_defender_controls_a_black_nontoken_permanent() {
    let p0 = PlayerId::new(0);
    // The defending player controls a black nontoken permanent, Sengir Vampire, so the trigger's
    // "if" is false when the Bears attack.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[spectral_bears()])
        .battlefield(1, &[sengir_vampire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = attack_with_the_bears(&mut engine);
    assert!(is_tapped(&engine, bears), "the Bears attacked");

    to_p0_next_main_phase(&mut engine);
    assert!(
        !is_tapped(&engine, bears),
        "a black nontoken permanent across the table: the Bears untap as usual"
    );
}

#[test]
fn spectral_bears_ignores_black_tokens_when_it_checks_the_defender() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    // P1 makes two black-and-green Pest tokens on its own turn. Tokens are not nontoken permanents,
    // so on P0's next attack the defender still "controls no black nontoken permanents".
    let mut engine = Duel::new(SEED, forest())
        .hand(1, &[pest_summoning()])
        .battlefield(0, &[spectral_bears()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, spectral_bears()).unwrap();
    reach_their_main_phase(&mut engine, p1);
    let summoning = in_hand(&engine, p1, pest_summoning()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: summoning })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bears),
        "the Bears untapped normally on turn three"
    );
    let bears = attack_with_the_bears(&mut engine);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the Pests did not block"
    );

    to_p0_next_main_phase(&mut engine);
    assert!(
        is_tapped(&engine, bears),
        "black tokens are not black nontoken permanents, so the Bears stay tapped"
    );
}

#[test]
fn spectral_bears_untaps_for_a_new_controller() {
    crate::engine::mechanics_tests::untap_scope::assert_untaps_after_control_change(
        spectral_bears(),
    );
}
