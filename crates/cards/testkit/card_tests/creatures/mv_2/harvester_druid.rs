//! `cards/creatures/mv_2/harvester_druid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn harvester_druid() -> CardIndex {
    card_index("c2b4f43a-6609-43ca-ad0d-897b6f8cb22f")
}

/// Harvester Druid ({1}{G} 1/1 Human Druid):
/// "{T}: Add one mana of any color that a land you control could produce."
#[test]
fn harvester_druid_adds_only_colors_your_lands_could_produce() {
    let mut e = lk::game(
        SEED,
        &[harvester_druid(), badlands()],
        &[island()],
        &[],
        &[],
    );
    let druid = lk::obj(&e, lk::P0, harvester_druid());

    activate(&mut e, lk::P0, harvester_druid(), 0);
    let Pending::ChooseColor { options, .. } = e.pending().clone() else {
        panic!("expected color choice from Badlands, got {:?}", e.pending());
    };
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Red],
        "opponent's Island does not grant Blue; only your Badlands' Black/Red are offered"
    );
    e.apply(lk::P0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert!(lk::tapped(&e, druid));
    assert_eq!(lk::pool_of(&e, lk::P0, ManaColor::Red), 1);

    // With no lands you control, Harvester Druid produces no mana.
    let mut e2 = lk::game(SEED, &[harvester_druid()], &[island()], &[], &[]);
    activate(&mut e2, lk::P0, harvester_druid(), 0);
    assert_eq!(lk::pool_total(&e2, lk::P0), 0);
}
