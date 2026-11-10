//! Oracle scenarios for Skyshooter.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn skyshooter_sacrifices_to_destroy_attacking_creature_with_flying() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let card = card_index("9c5a3407-e3e6-4f5f-9c24-6f24c19e1a2e");
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[card])
        .battlefield(1, &[index::STORM_CROW])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);

    let shooter = on_battlefield(&e, p0, card).unwrap();
    assert!(keywords(&e, shooter).contains(KeywordSet::REACH));
    let crow = on_battlefield(&e, p1, index::STORM_CROW).unwrap();

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(crow, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: shooter,
            ability_index: 0,
        },
    )
    .unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![crow],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, p0, card).is_none());
    assert!(on_battlefield(&e, p1, index::STORM_CROW).is_none());
}
