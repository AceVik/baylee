//! `cards/creatures/mv_5/shard_phoenix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn shard_phoenix() -> CardIndex {
    card_index("3b629793-0efb-4d60-9d82-38b7b400abfd")
}

/// Shard Phoenix (`3b629793-0efb-4d60-9d82-38b7b400abfd`) — {4}{R} 2/2 Creature — Phoenix (Implemented):
/// "Flying. Sacrifice this creature: It deals 2 damage to each creature without flying. {R}{R}{R}: Return this card from your graveyard to your hand. Activate only during your upkeep."
#[test]
fn shard_phoenix_sacrifices_to_damage_nonfliers_and_returns_during_upkeep() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                shard_phoenix(),
                index::AIR_ELEMENTAL,
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    activate(&mut engine, P0, shard_phoenix(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, PlayerId::new(1), index::GRIZZLY_BEARS).is_none(),
        "Grizzly Bears without flying should be destroyed by 2 damage"
    );
    let phoenix = in_graveyard(&engine, P0, shard_phoenix()).unwrap();
    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    assert!(
        !lk::offered(&engine, shard_phoenix(), P0, 1),
        "main phase is not upkeep"
    );
    reach_their_main_phase(&mut engine, lk::P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    assert!(lk::offered(&engine, shard_phoenix(), P0, 1));
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: phoenix,
                ability_index: 1,
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(in_hand(&engine, P0, shard_phoenix()).is_some());
    assert_eq!(lk::pool_total(&engine, P0), 0);
    let air = on_battlefield(&engine, P0, index::AIR_ELEMENTAL).unwrap();
    assert_eq!(
        engine.state().object(air).unwrap().damage,
        0,
        "Air Elemental with flying should take no damage"
    );
}
