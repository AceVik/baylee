//! Autumn Willow — `{4}{G}{G}` 4/4 Legendary Creature — Avatar (Homelands)
//!
//! Shroud
//! `{G}`: Until end of turn, Autumn Willow can be the target of spells and
//! abilities controlled by target player as though it didn't have shroud.
//!
//! (`Coverage::Partial`: per-player shroud bypass until end of turn is not in
//! `Effect`/`Modifier`; Autumn Willow has shroud and its `{G}` target-player
//! activated ability.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn autumn_willow() -> CardIndex {
    card_index("c8c868f8-19a5-4cc5-b769-aa382f7b97ad")
}

#[test]
fn autumn_willow_has_shroud_and_activates_green_target_player_ability() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[autumn_willow()])
        .hand(1, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let willow_hand = in_hand(&engine, p0, autumn_willow()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: willow_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let willow = on_battlefield(&engine, p0, autumn_willow()).unwrap();
    assert_eq!(pt(&engine, willow), (4, 4));
    assert!(keywords(&engine, willow).contains(KeywordSet::SHROUD));

    // Activate {G} targeting p0.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: willow,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Opponent's Lightning Bolt cannot target Autumn Willow because of shroud.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt_hand = in_hand(&engine, p1, lightning_bolt()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: bolt_hand })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !options.contains(&willow),
        "Autumn Willow has shroud and cannot be targeted by Lightning Bolt"
    );
}
