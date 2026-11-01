//! Touch of Vitae — `{2}{G}` Instant (Ice Age)
//!
//! Target creature gains haste until end of turn. Until end of turn, that
//! creature gains "`{0}`: Untap this creature. Activate only once each turn."
//! Draw a card at the beginning of the next turn's upkeep.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::GRANTED_ABILITY;

fn touch_of_vitae() -> CardIndex {
    card_index("84f1e596-f56f-48c9-9bff-2c0bb2cea2d3")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

#[test]
fn touch_of_vitae_grants_haste_and_once_per_turn_free_untap_and_next_upkeep_cantrip() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[prodigal_sorcerer(), touch_of_vitae()])
        .battlefield(
            0,
            &[island(), forest(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Prodigal Sorcerer (summoning sick).
    let tim_card = in_hand(&engine, p0, prodigal_sorcerer()).unwrap();
    let tov_card = in_hand(&engine, p0, touch_of_vitae()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: tim_card })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let tim = on_battlefield(&engine, p0, prodigal_sorcerer()).unwrap();

    // Summoning-sick Prodigal Sorcerer cannot activate its {T} ability yet.
    assert!(!ability_offered(&engine, tim, 0));

    // Cast Touch of Vitae targeting Prodigal Sorcerer.
    engine
        .apply(p0, PlayerAction::CastSpell { card: tov_card })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![tim] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Prodigal Sorcerer now has haste, so its {T} ping ability is legal!
    assert!(ability_offered(&engine, tim, 0));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, tim));
    assert_eq!(engine.state().players[1].life, 19);

    // Activate the granted "{0}: Untap this creature. Activate only once each turn."
    assert!(ability_offered(&engine, tim, GRANTED_ABILITY));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: GRANTED_ABILITY,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, tim),
        "granted {{0}} ability untapped Prodigal Sorcerer"
    );

    // Tap Prodigal Sorcerer a second time; the once-per-turn untap ability cannot be activated again.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(
        !ability_offered(&engine, tim, GRANTED_ABILITY),
        "granted untap ability is limited to once each turn"
    );

    // Advance to p1's upkeep: Touch of Vitae's delayed trigger draws a card for p0.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "Touch of Vitae draws a card at the next turn's upkeep"
    );
}
