//! `cards/creatures/mv_8/infernal_denizen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn infernal_denizen() -> CardIndex {
    card_index("2f33aced-c737-4b3c-ba25-aff7b340c2d7")
}

/// Infernal Denizen (2f33aced-c737-4b3c-ba25-aff7b340c2d7) — {7}{B} 5/7 Creature — Demon:
/// - "At the beginning of your upkeep, sacrifice two Swamps. If you can't, tap this creature, and an opponent may gain control of a creature you control of their choice for as long as this creature remains on the battlefield."
/// - "{T}: Gain control of target creature for as long as this creature remains on the battlefield."
#[test]
fn infernal_denizen_sacrifices_two_swamps_when_able_and_steals_creature_while_on_battlefield() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[infernal_denizen(), index::SWAMP, index::SWAMP, index::SWAMP],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[swords_to_plowshares(), plains()])
        .start();
    keep_mulligans(&mut engine);

    let denizen = on_battlefield(&engine, P0, infernal_denizen()).expect("Denizen seated");
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).expect("Bears seated");

    // First upkeep with 3 Swamps: P0 sacrifices two Swamps, and Infernal Denizen stays untapped.
    for _ in 0..2 {
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            unreachable!()
        };
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    assert!(
        !is_tapped(&engine, denizen),
        "sacrificing two Swamps leaves Infernal Denizen untapped"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        2,
        "two Swamps were sacrificed"
    );

    // P0 activates "{T}: Gain control of target creature for as long as this creature remains on the battlefield."
    activate(&mut engine, P0, infernal_denizen(), 1);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, denizen));
    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P0,
        "P0 gains control of Grizzly Bears"
    );

    // On P1's turn, P1 plays Plains and exiles Infernal Denizen with Swords to Plowshares;
    // control of Grizzly Bears returns to P1 immediately.
    reach_their_main_phase(&mut engine, P1);
    let p1_plains = in_hand(&engine, P1, plains()).expect("Plains in P1 hand");
    engine
        .apply(P1, PlayerAction::PlayLand { card: p1_plains })
        .unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    cast_from_hand(&mut engine, P1, swords_to_plowshares());
    aim_at(&mut engine, P1, denizen);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(bears).unwrap().controller,
        P1,
        "control of Grizzly Bears ends as soon as Infernal Denizen leaves the battlefield"
    );
}

/// With only one Swamp on P0's side at upkeep, P0 sacrifices that single Swamp
/// (CR 609.3: do as much as possible) and Infernal Denizen taps because P0
/// could not sacrifice two Swamps.
#[test]
fn infernal_denizen_sacrifices_one_swamp_and_taps_when_only_one_swamp_is_controlled() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[infernal_denizen(), index::SWAMP, forest()])
        .start();
    keep_mulligans(&mut engine);

    let denizen = on_battlefield(&engine, P0, infernal_denizen()).expect("Denizen seated");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, P0);

    assert!(
        in_graveyard(&engine, P0, index::SWAMP).is_some(),
        "the single Swamp is still sacrificed (CR 609.3)"
    );
    assert!(
        on_battlefield(&engine, P0, forest()).is_some(),
        "non-Swamp land is untouched"
    );
    assert!(
        is_tapped(&engine, denizen),
        "being unable to sacrifice two Swamps taps Infernal Denizen"
    );
}
