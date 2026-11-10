//! Lumengrid Augur loots and untaps only after an artifact discard.

use super::*;

fn lumengrid_augur() -> CardIndex {
    card_index("d5409a7c-b928-4819-a560-e80226cf61f0")
}

fn loot(discard: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lumengrid_augur(), island()])
        .hand(0, &[discard])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, lumengrid_augur(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p0],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let card = in_hand(&e, p0, discard).unwrap();
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    e
}

#[test]
fn lumengrid_augur_untaps_only_for_an_artifact_discard() {
    let p0 = PlayerId::new(0);
    let e = loot(quiet_artifact());
    let augur = on_battlefield(&e, p0, lumengrid_augur()).unwrap();
    assert!(!is_tapped(&e, augur));
    let e = loot(forest());
    let augur = on_battlefield(&e, p0, lumengrid_augur()).unwrap();
    assert!(is_tapped(&e, augur));
}

/// "Target player draws a card, then discards a card": aimed at the
/// opponent, the opponent draws, the opponent is the one asked to discard,
/// and their artifact discard untaps the Augur.
#[test]
fn lumengrid_augur_aimed_at_the_opponent_lets_them_choose_the_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[lumengrid_augur(), island()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let our_hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, lumengrid_augur(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { player, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p1, "the target player chooses what to discard");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p1)).len(),
        2,
        "the opponent drew first"
    );
    let card = in_hand(&e, p1, quiet_artifact()).unwrap();
    e.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![card],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p1, quiet_artifact()).is_some());
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p1)).len(), 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), our_hand);
    let augur = on_battlefield(&e, p0, lumengrid_augur()).unwrap();
    assert!(!is_tapped(&e, augur), "an artifact was discarded this way");
}
