//! `cards/sorceries/mv_4/thunder_of_hooves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Thunder of Hooves (`4b505a4c-ccc3-4461-bbc4-ad5d39f9b879`) — {3}{R} Sorcery:
/// "Thunder of Hooves deals X damage to each creature without flying and each player, where X is the number of Beasts on the battlefield."
#[test]
fn thunder_of_hooves_deals_beast_count_damage_to_nonflyers_and_players() {
    let thunder_card = card_index("4b505a4c-ccc3-4461-bbc4-ad5d39f9b879");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[thunder_card])
        .battlefield(
            0,
            &[
                index::ANURID_BARKRIPPER,
                index::GARRUK_S_COMPANION,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::PHANTOM_MONSTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thunder_card);
    pass_until(&mut engine, stack_is_empty);

    // 2 Beasts on the battlefield -> 2 damage to each creature without flying and each player.
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(in_graveyard(&engine, p1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, p1, index::PHANTOM_MONSTER).is_some());
}
