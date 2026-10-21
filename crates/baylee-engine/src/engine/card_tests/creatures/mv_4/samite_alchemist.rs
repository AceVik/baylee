//! Samite Alchemist — `{3}{W}` 0/2 Creature — Human Cleric (Homelands)
//!
//! `{W}{W}`, `{T}`: Prevent the next 4 damage that would be dealt this turn to target creature you
//! control. Tap that creature. It doesn't untap during your next untap step.

#[allow(clippy::wildcard_imports)]
use super::*;

fn samite_alchemist() -> CardIndex {
    card_index("120b565a-e22b-4d5d-9841-b11750b4ee49")
}

#[test]
fn samite_alchemist_prevents_four_damage_taps_target_and_suppresses_next_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                samite_alchemist(),
                grizzly_bears(),
                plains(),
                plains(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let alchemist = on_battlefield(&engine, p0, samite_alchemist()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: alchemist,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, bears),
        "Samite Alchemist tapped the targeted creature"
    );

    // Cast Lightning Bolt at Grizzly Bears: all 3 damage is prevented by the 4-damage shield.
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield,
        "Grizzly Bears survived Lightning Bolt thanks to the 4-damage prevention shield"
    );
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);

    // Advance through p1's turn to p0's next FirstMain: Grizzly Bears stays tapped!
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(
        is_tapped(&engine, bears),
        "Grizzly Bears did not untap during p0's next untap step"
    );
}
