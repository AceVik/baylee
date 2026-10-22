//! Ivory Gargoyle — `{4}{W}` 2/2 Creature — Gargoyle (Alliances)
//!
//! Flying
//! When this creature dies, return it to the battlefield under its owner's
//! control at the beginning of the next end step and you skip your next draw
//! step.
//! `{4}{W}`: Exile this creature.
//!
//! (`Coverage::Partial`: waiting until the beginning of the next end step to
//! return and skipping your next draw step when this creature dies are not
//! supported; has flying, returns from the graveyard to the battlefield under
//! its owner's control when it dies, and exiles itself for `{4}{W}`.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn ivory_gargoyle() -> CardIndex {
    card_index("e8cc9f83-4e4c-4820-9a7a-f82f728e9767")
}

#[test]
fn ivory_gargoyle_returns_when_it_dies_and_exiles_itself_for_4w() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                ivory_gargoyle(),
                mountain(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let gargoyle = on_battlefield(&engine, p0, ivory_gargoyle()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();

    // Kill Ivory Gargoyle with Lightning Bolt -> dies trigger returns it to the battlefield under p0's control.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gargoyle],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let returned = on_battlefield(&engine, p0, ivory_gargoyle())
        .expect("Ivory Gargoyle returned from the graveyard to the battlefield when it died");
    assert!(in_graveyard(&engine, p0, ivory_gargoyle()).is_none());

    // Activate {4}{W}: Exile this creature using the remaining floating mana.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: returned,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, ivory_gargoyle()).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 1);
}
