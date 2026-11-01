//! `cards/enchantments/mv_4/statecraft.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "854ef7c2-52e9-486a-aab6-7e3b3a6185ea"
fn statecraft() -> CardIndex {
    card_index("854ef7c2-52e9-486a-aab6-7e3b3a6185ea")
}

/// `Statecraft` (`Coverage::Implemented`):
/// "Prevent all combat damage that would be dealt to and dealt by creatures you control."
#[test]
fn statecraft_prevents_combat_damage_to_and_by_controlled_creatures() {
    let mut engine = lk::game(
        SEED,
        &[statecraft(), index::GRAY_OGRE],
        &[index::GRIZZLY_BEARS],
        &[],
        &[index::SHOCK],
    );
    resync(&mut engine);

    let ogre = lk::obj(&engine, P0, index::GRAY_OGRE);
    let bear = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    // P0 attacks with Gray Ogre (2/2) and P1 blocks with Grizzly Bears (2/2).
    // Statecraft prevents all combat damage dealt to and dealt by P0's creatures,
    // so both creatures survive with 0 damage marked.
    let options = attack_and_collect_blocks(&mut engine, ogre, P1);
    assert!(options.iter().any(|o| o.blocker == bear));
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, ogre)],
            },
        )
        .expect("block declared");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(
        on_battlefield(&engine, P0, index::GRAY_OGRE).is_some(),
        "P0's Gray Ogre takes no combat damage from Grizzly Bears"
    );
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "P1's Grizzly Bears takes no combat damage from P0's Gray Ogre"
    );
    assert_eq!(engine.state().object(ogre).unwrap().damage, 0);
    assert_eq!(engine.state().object(bear).unwrap().damage, 0);

    // Noncombat damage to P0's creature is NOT prevented by Statecraft.
    lk::float(&mut engine, P1, &[(ManaColor::Red, 1)]);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_with_floating(&mut engine, P1, index::SHOCK);
    lk::aim(&mut engine, &[ogre], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, index::GRAY_OGRE).is_some(),
        "Noncombat damage from Shock is not prevented by Statecraft"
    );
}
