//! Forget — `{U}{U}` Sorcery (Homelands)
//!
//! Target player discards two cards, then draws as many cards as they discarded
//! this way.

#[allow(clippy::wildcard_imports)]
use super::*;

fn forget() -> CardIndex {
    card_index("619ef7e1-33cd-4470-a1d4-83c5f1f5c31e")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

#[test]
fn forget_makes_target_player_discard_two_and_draw_as_many_as_discarded() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[forget(), forget()])
        .hand(1, &[bears(), gray_ogre()])
        .battlefield(0, &[island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forget1 = in_hand(&engine, p0, forget()).unwrap();
    let c1 = in_hand(&engine, p1, bears()).unwrap();
    let c2 = in_hand(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: forget1 })
        .unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![c1, c2],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // p1 discarded 2 cards (bears and gray_ogre) and drew 2 replacement Islands from library.
    assert_eq!(engine.state().object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(c2).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        2,
        "target player drew 2 cards for the 2 cards discarded"
    );
}

/// "As many cards as they discarded this way": a hand of one is one discard
/// and one draw, not two.
#[test]
fn forget_draws_only_as_many_cards_as_were_discarded() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[forget()])
        .hand(1, &[bears()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, forget()).unwrap();
    let only_card = in_hand(&engine, p1, bears()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![only_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(only_card).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        1,
        "one card discarded, one card drawn"
    );
    assert!(in_graveyard(&engine, p0, forget()).is_some());
}
