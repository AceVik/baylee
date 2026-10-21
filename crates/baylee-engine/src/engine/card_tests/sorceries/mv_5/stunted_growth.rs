//! Stunted Growth — `{3}{G}{G}` Sorcery (Ice Age)
//!
//! Target player chooses three cards from their hand and puts them on top of
//! their library in any order.
//! (`Coverage::Partial`: putting cards from a targeted player's hand on top of
//! their library in any order is not supported; target player discards three
//! cards instead.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn stunted_growth() -> CardIndex {
    card_index("e3480c8f-15ad-451b-bb34-b129805569e1")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn stunted_growth_makes_target_player_discard_three_chosen_cards_from_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[stunted_growth()])
        .hand(1, &[grizzly_bears(), hill_giant(), mountain(), swamp()])
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sg = in_hand(&engine, p0, stunted_growth()).unwrap();
    let c1 = in_hand(&engine, p1, grizzly_bears()).unwrap();
    let c2 = in_hand(&engine, p1, hill_giant()).unwrap();
    let c3 = in_hand(&engine, p1, mountain()).unwrap();
    let keep = in_hand(&engine, p1, swamp()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: sg })
        .unwrap();
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![c1, c2, c3],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)),
        &[keep],
        "only the unchosen card remains in target player's hand"
    );
}
