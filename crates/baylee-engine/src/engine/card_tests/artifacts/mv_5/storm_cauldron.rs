//! `cards/artifacts/mv_5/storm_cauldron.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn storm_cauldron() -> CardIndex {
    card_index("5a51b168-02d1-4eb4-8ccc-614ab6f6cffa")
}

/// Storm Cauldron ({5} Artifact):
/// - "Each player may play an additional land during each of their turns."
/// - "Whenever a land is tapped for mana, return it to its owner's hand."
#[test]
fn storm_cauldron_grants_extra_land_drop_and_bounces_lands_tapped_for_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[storm_cauldron()])
        .hand(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Play two Forests on the same turn thanks to the extra land play.
    for _ in 0..2 {
        let f = in_hand(&engine, p0, forest()).unwrap();
        engine
            .apply(p0, PlayerAction::PlayLand { card: f })
            .unwrap();
    }
    let land = on_battlefield(&engine, p0, forest()).unwrap();

    // Tapping a Forest for mana triggers Storm Cauldron to return that Forest to its owner's hand.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(land).unwrap().zone, Zone::Hand);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}
