//! `cards/creatures/mv_2/goblin_sappers.rs`, played.
//!
//! Oracle:
//! - "{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it and this creature at end of combat."
//! - "{R}{R}{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it at end of combat."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_sappers() -> CardIndex {
    card_index("9e095761-cdae-4ab2-b2d3-099b0b39600a")
}

/// Activates `ability` of Goblin Sappers at player 0's Grizzly Bears, with a
/// Hill Giant on the other side that could block the Bears and is no target,
/// then attacks with the Bears and plays the combat out.
fn sap_and_attack(ability: u32, red: u32) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = lk::game(
        SEED,
        &[goblin_sappers(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let sappers = lk::obj(&engine, P0, goblin_sappers());
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    lk::float(&mut engine, P0, &[(ManaColor::Red, red)]);
    activate(&mut engine, P0, goblin_sappers(), ability);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&bears),
        "a creature you control is a target"
    );
    assert!(
        !options.contains(&giant),
        "an opponent's creature is not a target: \"target creature you control\""
    );
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(
        lk::zone_of(&engine, bears),
        Zone::Battlefield,
        "nothing is destroyed before end of combat"
    );

    let offer = lk::attack(&mut engine, &[bears]);
    assert!(
        !lk::may_block(&offer, giant, bears),
        "the target can't be blocked this turn"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    (engine, sappers, bears)
}

#[test]
fn goblin_sappers_cheap_ability_destroys_both_at_end_of_combat() {
    let (engine, sappers, bears) = sap_and_attack(0, 2);
    assert_eq!(lk::life(&engine, P1), 18, "the unblockable Bears connected");
    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, sappers), Zone::Graveyard);
}

#[test]
fn goblin_sappers_expensive_ability_destroys_only_the_target() {
    let (engine, sappers, bears) = sap_and_attack(1, 4);
    assert_eq!(lk::life(&engine, P1), 18, "the unblockable Bears connected");
    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, sappers), Zone::Battlefield);
}
