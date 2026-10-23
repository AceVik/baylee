//! `cards/creatures/mv_3/soldevi_heretic.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_heretic() -> CardIndex {
    card_index("eed353a6-757d-47f6-a118-b951ccf95600")
}

/// Soldevi Heretic ({2}{U} 2/2 Creature — Human Cleric, Partial):
/// "{W}, {T}: Prevent the next 2 damage that would be dealt to target creature
/// this turn. Target opponent may draw a card."
/// A second opponent target can be announced, but no effect-player reference
/// names that second target for the optional draw. The shield remains absent
/// until that independently bound opponent can be offered their decision.
#[test]
fn soldevi_heretic_offers_no_shield_without_its_drawback() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[soldevi_heretic(), grizzly_bears(), plains(), mountain()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let heretic = on_battlefield(&engine, p0, soldevi_heretic()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    tap_all_mana_but(&mut engine, p0, Some(soldevi_heretic()));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == heretic),
        "the {{W}}, {{T}} ability is not offered"
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: heretic,
                    ability_index: 0,
                },
            )
            .is_err()
    );

    // Nothing is prevented: Lightning Bolt kills the Bears.
    cast_with_floating(&mut engine, p0, lightning_bolt());
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
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(!is_tapped(&engine, heretic));
}
