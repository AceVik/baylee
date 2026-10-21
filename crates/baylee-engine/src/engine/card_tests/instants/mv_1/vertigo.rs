//! Vertigo — `{R}` Instant (Ice Age)
//!
//! Vertigo deals 2 damage to target creature with flying. That creature loses
//! flying until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vertigo() -> CardIndex {
    card_index("1dabfade-79e5-44e2-afa6-b20498c6bb8b")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

#[test]
fn vertigo_targets_only_flying_creature_deals_two_damage_and_strips_flying_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[vertigo()])
        .battlefield(0, &[mountain(), air_elemental()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let flyer = on_battlefield(&engine, p0, air_elemental()).unwrap();
    let ground = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, vertigo()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    // Ground Grizzly Bears is excluded; only Air Elemental with flying can be targeted.
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(options.contains(&flyer));
    assert!(
        !options.contains(&ground),
        "creature without flying cannot be targeted by Vertigo"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![flyer],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let flyer_obj = engine.state().object(flyer).unwrap();
    assert_eq!(flyer_obj.damage, 2, "Air Elemental took 2 damage");
    assert!(
        !flyer_obj
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING),
        "Air Elemental lost flying until end of turn"
    );

    // Because Air Elemental lost flying, opponent's ground Grizzly Bears can block it!
    let blocks = attack_and_collect_blocks(&mut engine, flyer, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == ground && b.attackers.contains(&flyer)),
        "ground Grizzly Bears can block Air Elemental after Vertigo removes flying"
    );
}
