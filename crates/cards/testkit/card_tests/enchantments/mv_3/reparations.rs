//! `cards/enchantments/mv_3/reparations.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn reparations() -> CardIndex {
    card_index("3cb86deb-f4e5-41c5-bfcd-614a24f3499c")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Whenever an opponent casts a spell targeting you or a creature you control,
/// you may draw a card (accept and decline branches); spells targeting
/// opponent's own side or cast by you do not trigger.
#[test]
fn reparations_triggers_when_opponent_targets_you_or_your_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[reparations(), grizzly_bears()],
        &[savannah_lions()],
        &[giant_growth()],
        &[lightning_bolt(), lightning_bolt(), lightning_bolt()],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let their_lions = lk::obj(&e, p1, savannah_lions());

    // Casting a spell targeting my own creature does not trigger Reparations.
    lk::float(&mut e, p0, &[(ManaColor::Green, 1)]);
    cast_with_floating(&mut e, p0, giant_growth());
    lk::aim(&mut e, &[my_bear], &[]);
    lk::settle(&mut e);

    // Opponent Bolt targeting their own creature -> no trigger.
    let p0_hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, p1, &[(ManaColor::Red, 3)]);
    let bolt1 = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt1 })
        .unwrap();
    lk::aim(&mut e, &[their_lions], &[]);
    lk::settle(&mut e);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), p0_hand);

    // Opponent Bolt targeting P0 -> trigger offers optional draw (accept).
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt2 = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt2 })
        .unwrap();
    lk::aim(&mut e, &[], &[p0]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand + 1
    );

    // Opponent Bolt targeting P0's creature -> trigger offers optional draw (decline).
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    let bolt3 = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt3 })
        .unwrap();
    lk::aim(&mut e, &[my_bear], &[]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(p0)).len(),
        p0_hand + 1
    );
}
