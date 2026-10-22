//! `cards/creatures/mv_4/rathi_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rathi_dragon() -> CardIndex {
    card_index("6a180f07-7afa-4605-9c58-4521a5f3fcc1")
}

/// `Rathi Dragon` ({2}{R}{R}, 5/5 Creature — Dragon, `Coverage::Implemented`):
/// `Flying`
/// `When this creature enters, sacrifice it unless you sacrifice two Mountains.`
#[test]
fn rathi_dragon_sacrifices_itself_unless_controller_sacrifices_two_mountains() {
    let p0 = PlayerId::new(0);

    // Branch 1: Decline sacrificing two Mountains -> Rathi Dragon is sacrificed.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[rathi_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, rathi_dragon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, rathi_dragon()).is_none());
    assert!(in_graveyard(&engine, p0, rathi_dragon()).is_some());

    // Branch 2: Sacrifice two Mountains -> Rathi Dragon stays on the battlefield.
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[rathi_dragon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mountains: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = engine.state().object(id).unwrap();
            o.controller == p0 && o.card.is_some_and(|c| c.index == mountain())
        })
        .collect();
    assert_eq!(mountains.len(), 4);

    cast_from_hand(&mut engine, p0, rathi_dragon());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mountains[0], mountains[1]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, rathi_dragon()).unwrap();
    let ch = engine.state().object(dragon).unwrap().characteristics();
    assert!(ch.keywords.contains(KeywordSet::FLYING));
    assert_eq!((ch.power, ch.toughness), (Some(5), Some(5)));
}
