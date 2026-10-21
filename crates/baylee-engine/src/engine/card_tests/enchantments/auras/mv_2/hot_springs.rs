//! `cards/enchantments/auras/mv_2/hot_springs.rs`, played.
//!
//! Oracle:
//! - "Enchant land you control"
//! - "Enchanted land has \"{T}: Prevent the next 1 damage that would be dealt to any target this turn.\""

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hot_springs() -> CardIndex {
    card_index("123880ad-7b54-4ab0-82eb-c78b12ab89a7")
}

#[test]
fn hot_springs_grants_land_tap_ability_to_prevent_one_damage() {
    let mut engine = lk::game(
        SEED,
        &[index::FOREST],
        &[index::FOREST],
        &[hot_springs(), index::LIGHTNING_BOLT],
        &[],
    );
    let land = lk::obj(&engine, P0, index::FOREST);
    let theirs = lk::obj(&engine, P1, index::FOREST);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut engine, P0, hot_springs());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&land),
        "a land you control may be enchanted"
    );
    assert!(
        !options.contains(&theirs),
        "\"enchant land you control\": an opponent's land is no target"
    );
    lk::aim(&mut engine, &[land], &[]);
    lk::settle(&mut engine);

    let grant = crate::effects::granted_activated(engine.state(), land)
        .next()
        .unwrap()
        .id;
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: grant,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[], &[P0]);
    lk::settle(&mut engine);
    assert!(
        lk::tapped(&engine, land),
        "the granted ability's cost taps the land"
    );

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    lk::cast_at(&mut engine, P0, index::LIGHTNING_BOLT, &[], &[P0]);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "1 of Lightning Bolt's 3 damage is prevented by the land enchanted with Hot Springs"
    );
}
