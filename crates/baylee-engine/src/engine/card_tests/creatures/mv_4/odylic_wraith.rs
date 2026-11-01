//! `cards/creatures/mv_4/odylic_wraith.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn odylic_wraith() -> CardIndex {
    card_index("613fa223-856e-49c4-8952-224895e00b5e")
}

fn llanowar_elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// Odylic Wraith (613fa223-856e-49c4-8952-224895e00b5e) — "Swampwalk" and
/// "whenever this creature deals damage to a player, that player discards a
/// card." The defender controls a Swamp, so the Llanowar Elves may not block it;
/// 2 damage goes through and player 1 (not the attacker) is asked which of their
/// two cards to discard, from their own hand.
#[test]
fn odylic_wraith_is_unblockable_by_a_swamp_player_and_makes_them_discard() {
    let mut e = lk::game(
        SEED,
        &[odylic_wraith()],
        &[swamp(), llanowar_elves()],
        &[],
        &[island(), plains()],
    );
    let wraith = lk::obj(&e, P0, odylic_wraith());
    let elves = lk::obj(&e, P1, llanowar_elves());
    let island_id = in_hand(&e, P1, island()).expect("Island in hand");
    let plains_id = in_hand(&e, P1, plains()).expect("Plains in hand");

    let offer = lk::attack(&mut e, &[wraith]);
    assert!(
        !lk::may_block(&offer, elves, wraith),
        "swampwalk: no block against a Swamp"
    );
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, P1, "the damaged player discards");
    assert!(
        options.contains(&island_id) && options.contains(&plains_id) && options.len() == 2,
        "from their own hand: {options:?}"
    );
    e.apply(
        P1,
        PlayerAction::ChooseObjects {
            objects: vec![island_id],
        },
    )
    .unwrap();
    lk::through_combat(&mut e);

    assert_eq!(lk::life(&e, P1), 18, "2 damage");
    assert!(
        in_graveyard(&e, P1, island()).is_some(),
        "the chosen card is discarded"
    );
    assert!(in_hand(&e, P1, plains()).is_some(), "only one card goes");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "the attacker discards nothing"
    );
}

/// Odylic Wraith — without a Swamp on the defender's side the Elves may block
/// (swampwalk only works against a Swamp), the Wraith deals its damage to the
/// Elves and not to a player, so nobody discards and the Island stays in hand.
#[test]
fn odylic_wraith_can_be_blocked_without_a_swamp_and_then_no_one_discards() {
    let mut e = lk::game(
        SEED,
        &[odylic_wraith()],
        &[llanowar_elves()],
        &[],
        &[island()],
    );
    let wraith = lk::obj(&e, P0, odylic_wraith());
    let elves = lk::obj(&e, P1, llanowar_elves());

    let offer = lk::attack(&mut e, &[wraith]);
    assert!(
        lk::may_block(&offer, elves, wraith),
        "no Swamp, so the Elves may block"
    );
    lk::block(&mut e, &[(elves, wraith)]);
    lk::through_combat(&mut e);

    assert!(
        in_graveyard(&e, P1, llanowar_elves()).is_some(),
        "the 1/1 Elves die"
    );
    assert_eq!(lk::life(&e, P1), 20, "no damage to the player");
    assert!(
        in_hand(&e, P1, island()).is_some(),
        "damage to a creature makes no one discard"
    );
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "nothing was asked"
    );
}
