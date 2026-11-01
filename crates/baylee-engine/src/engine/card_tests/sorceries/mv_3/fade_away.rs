//! `cards/sorceries/mv_3/fade_away.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fade_away() -> CardIndex {
    card_index("f92c2d47-f2f7-43d9-acf3-0145bad1977d")
}

/// Fade Away (`f92c2d47-f2f7-43d9-acf3-0145bad1977d`) — {2}{U} Sorcery (Implemented):
/// "For each creature, its controller sacrifices a permanent unless they pay {1}."
#[test]
fn fade_away_causes_player_to_sacrifice_a_permanent_for_each_creature_unless_paying_1() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[fade_away()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, fade_away());
    kit::settle(&mut engine, P0, |_, p| {
        if matches!(p, Pending::YesNo { .. }) {
            return Some(PlayerAction::YesNo(false));
        }
        kit::choose_cards(p, &[bear])
    });

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}

#[test]
fn fade_away_asks_each_creatures_controller_and_can_sacrifice_a_noncreature() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[index::HILL_GIANT, forest()],
        &[fade_away()],
        &[],
    );
    let land = lk::obj(&e, P1, forest());
    lk::float(&mut e, P0, &[(ManaColor::Blue, 10)]);
    cast_with_floating(&mut e, P0, fade_away());
    kit::settle(&mut e, P0, |_, p| {
        if let Pending::YesNo { player, .. } = p {
            return Some(PlayerAction::YesNo(*player == P0));
        }
        kit::choose_cards(p, &[land])
    });
    assert!(on_battlefield(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P1, index::HILL_GIANT).is_some());
    assert!(in_graveyard(&e, P1, forest()).is_some());
    assert_eq!(
        lk::pool_total(&e, P0),
        6,
        "three to cast, one for your own creature"
    );
}
