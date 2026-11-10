//! `cards/creatures/mv_4/dawn_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn dawn_elemental_prevents_all_damage_that_would_be_dealt_to_it() {
    let elemental = card_index("307421ed-aa59-4518-a0eb-689c60d7b709");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[elemental, mountain()])
        .hand(0, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elem_id = on_battlefield(&engine, p0, elemental).unwrap();
    assert!(keywords(&engine, elem_id).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::LIGHTNING_BOLT);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elem_id],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, elemental).is_some(),
        "Dawn Elemental prevents all 3 damage from Lightning Bolt and survives"
    );
    assert_eq!(engine.state().object(elem_id).unwrap().damage, 0);
}
