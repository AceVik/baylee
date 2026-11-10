//! `cards/instants/mv_1/canopy_claws.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn canopy_claws_removes_flying_from_target_creature_and_flashes_back_from_graveyard() {
    let canopy_claws = card_index("195c6453-e5cc-4b3b-8518-eaccf29bc4a6");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[index::WIND_DRAKE, index::SCRYB_SPRITES])
        .hand(0, &[canopy_claws])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drake = on_battlefield(&engine, p1, index::WIND_DRAKE).unwrap();
    let sprites = on_battlefield(&engine, p1, index::SCRYB_SPRITES).unwrap();
    assert!(keywords(&engine, drake).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, sprites).contains(KeywordSet::FLYING));

    let f0 = all_on_battlefield(&engine, p0, forest())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: f0 })
        .unwrap();
    let spell = in_hand(&engine, p0, canopy_claws).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![drake],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, drake).contains(KeywordSet::FLYING));
    assert!(in_graveyard(&engine, p0, canopy_claws).is_some());

    let f1 = all_on_battlefield(&engine, p0, forest())[1];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: f1 })
        .unwrap();
    let gy_claws = in_graveyard(&engine, p0, canopy_claws).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: gy_claws })
        .expect("flashback from graveyard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![sprites],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(!keywords(&engine, sprites).contains(KeywordSet::FLYING));
    assert_eq!(
        engine.state().object(gy_claws).unwrap().zone,
        Zone::Exile,
        "exiled after flashback"
    );
}
