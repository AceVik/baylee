//! `cards/artifacts/equipment/mv_1/zephyr_boots.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn zephyr_boots() -> CardIndex {
    card_index("02f3b9c3-f611-4f05-ab3e-b296916efdad")
}

/// Zephyr Boots — "Equipped creature has flying. Whenever equipped creature
/// deals combat damage to a player, draw a card, then discard a card." and
/// "Equip {2}". Their ground Elf cannot block the flying wearer; the hit
/// draws a card and then asks for a discard.
#[test]
fn zephyr_boots_fly_and_loot_on_combat_damage() {
    let elf = llanowar_elves();
    let mut engine = rig(2, &[mountain()], &[zephyr_boots(), elf], &[elf]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let host = on_battlefield(&engine, p0, elf).expect("the Elf");
    equip(&mut engine, zephyr_boots(), 2, host);
    assert!(keywords(&engine, host).contains(KeywordSet::FLYING));

    let blocks = attack_and_collect_blocks(&mut engine, host, p1);
    assert!(
        blocks.iter().all(|b| !b.attackers.contains(&host)),
        "a ground Elf cannot block a flyer"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(engine.state().players[1].life, 19, "the Elf hit for 1");
    assert_eq!(hand(&engine), 2, "drawn first: Mountain and a new card");
    let Pending::ChooseCards {
        player,
        min,
        max,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!((player, min, max), (p0, 1, 1), "then discard a card");
    let mountain = in_hand(&engine, p0, mountain()).expect("the Mountain");
    assert!(options.contains(&mountain));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mountain],
            },
        )
        .expect("discard the Mountain");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(hand(&engine), 1);
    assert!(in_graveyard(&engine, p0, self::mountain()).is_some());
}
