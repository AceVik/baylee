//! `cards/enchantments/mv_2/lurking_skirge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
fn toughness_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<i16> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.characteristics().toughness)
}

const P1: PlayerId = PlayerId::new(1);

fn lurking_skirge() -> CardIndex {
    card_index("21e90420-aedb-4a84-ba57-d5eae497b09b")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn lurking_skirge_animates_only_when_creature_enters_opponents_graveyard_from_battlefield() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[lurking_skirge()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, lurking_skirge());
    pass_until(&mut e, stack_is_empty);
    let skirge = on_battlefield(&e, P0, lurking_skirge()).unwrap();

    // P0 creature dying doesn't trigger Skirge.
    let p0_bears = on_battlefield(&e, P0, grizzly_bears()).unwrap();
    kill(&mut e, p0_bears);
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state()
            .object(skirge)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );

    // P1 discarding a creature from hand doesn't trigger Skirge.
    hand_to_graveyard(&mut e, P1, grizzly_bears());
    pass_until(&mut e, stack_is_empty);
    assert!(
        !e.state()
            .object(skirge)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );

    // P1 creature dying from battlefield animates Skirge into 3/2 Flying Phyrexian Imp.
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    kill(&mut e, p1_bears);
    pass_until(&mut e, stack_is_empty);

    let chars = e.state().object(skirge).unwrap().characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(!chars.types.contains(TypeSet::ENCHANTMENT));
    assert_eq!(
        (power_of(&e, skirge), toughness_of(&e, skirge)),
        (Some(3), Some(2))
    );
    assert!(keywords_of(&e, skirge).contains(KeywordSet::FLYING));
}
