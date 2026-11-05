//! The plain combat damage step, one rule at a time: an unblocked attacker
//! hits the player it attacks (CR 510.1a), a blocked one hits its blocker
//! and not the player (CR 510.1c), a creature with no power deals nothing
//! (CR 510.1a), a creature that stays blocked after its blocker left deals
//! nothing without trample (CR 509.1h, 510.1c), lifelink on the attacker
//! (CR 702.15b), trample's excess (CR 702.19b) and the two damage steps
//! (CR 510.4).

use super::*;

fn life(state: &GameState, seat: PlayerId) -> i32 {
    state.players[usize::from(seat.get())].life
}

#[test]
fn an_unblocked_attacker_deals_its_power_to_the_player_it_attacks() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
    attack(&mut state, bear, P1);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 17);
    assert_eq!(
        life(&state, P0),
        20,
        "the attacker's controller is untouched"
    );
}

#[test]
fn several_unblocked_attackers_add_up() {
    let mut state = empty_state();
    for power in [1, 2, 4] {
        let id = creature(&mut state, P0, power, 1, KeywordSet::EMPTY);
        attack(&mut state, id, P1);
    }
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 13);
}

#[test]
fn a_blocked_attacker_deals_its_damage_to_the_blocker_and_none_to_the_player() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 1, 4, KeywordSet::EMPTY);
    attack(&mut state, attacker, P1);
    block(&mut state, blocker, attacker);
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, blocker), 3);
    assert_eq!(damage(&state, attacker), 1, "the blocker strikes back");
    assert_eq!(life(&state, P1), 20);
}

#[test]
fn a_zero_power_attacker_deals_no_damage() {
    let mut state = empty_state();
    let wall = creature(&mut state, P0, 0, 5, KeywordSet::EMPTY);
    attack(&mut state, wall, P1);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
}

#[test]
fn a_negative_power_attacker_deals_no_damage_and_heals_nobody() {
    let mut state = empty_state();
    let shrunk = creature(&mut state, P0, -2, 1, KeywordSet::EMPTY);
    attack(&mut state, shrunk, P1);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
}

#[test]
fn a_zero_power_blocker_deals_nothing_back() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let wall = creature(&mut state, P1, 0, 4, KeywordSet::EMPTY);
    attack(&mut state, attacker, P1);
    block(&mut state, wall, attacker);
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, attacker), 0);
    assert_eq!(damage(&state, wall), 2);
}

/// CR 509.1h: an attacker stays blocked after its only blocker leaves;
/// without trample it assigns no combat damage at all.
#[test]
fn an_attacker_whose_blocker_left_deals_no_damage_without_trample() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 4, 4, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
    attack(&mut state, attacker, P1);
    block(&mut state, blocker, attacker);
    state.combat.remove_from_combat(blocker);
    assert!(state.combat.is_blocked(attacker));
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 20);
    assert_eq!(
        damage(&state, blocker),
        0,
        "nothing reaches the removed blocker"
    );
}

#[test]
fn lifelink_on_an_unblocked_attacker_gains_its_controller_the_damage() {
    let mut state = empty_state();
    let lifelinker = creature(&mut state, P0, 3, 3, KeywordSet::LIFELINK);
    attack(&mut state, lifelinker, P1);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 17);
    assert_eq!(life(&state, P0), 23);
}

#[test]
fn lifelink_on_a_blocked_attacker_gains_what_it_dealt_to_the_blocker() {
    let mut state = empty_state();
    let lifelinker = creature(&mut state, P0, 2, 5, KeywordSet::LIFELINK);
    let blocker = creature(&mut state, P1, 1, 6, KeywordSet::EMPTY);
    attack(&mut state, lifelinker, P1);
    block(&mut state, blocker, lifelinker);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P0), 22);
    assert_eq!(life(&state, P1), 20);
}

#[test]
fn trample_assigns_lethal_to_the_blocker_and_the_rest_to_the_player() {
    let mut state = empty_state();
    let trampler = creature(&mut state, P0, 5, 5, KeywordSet::TRAMPLE);
    let blocker = creature(&mut state, P1, 1, 2, KeywordSet::EMPTY);
    attack(&mut state, trampler, P1);
    block(&mut state, blocker, trampler);
    deal_combat_damage(&mut state, false);
    let to_blocker = damage(&state, blocker);
    let to_player = 20 - life(&state, P1);
    assert!(
        to_blocker >= 2,
        "at least lethal to the blocker, got {to_blocker}"
    );
    assert_eq!(
        i32::from(to_blocker) + to_player,
        5,
        "all five points are dealt, none twice"
    );
}

#[test]
fn trample_into_a_bigger_blocker_reaches_nobody_else() {
    let mut state = empty_state();
    let trampler = creature(&mut state, P0, 3, 3, KeywordSet::TRAMPLE);
    let blocker = creature(&mut state, P1, 0, 7, KeywordSet::EMPTY);
    attack(&mut state, trampler, P1);
    block(&mut state, blocker, trampler);
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, blocker), 3);
    assert_eq!(life(&state, P1), 20);
}

/// CR 510.4: with a first striker in combat, only creatures with first strike
/// or double strike assign damage in the first-strike damage step; the
/// ordinary attacker waits for the regular one.
#[test]
fn an_ordinary_attacker_deals_nothing_in_the_first_strike_step() {
    let mut state = empty_state();
    let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let knight = creature(&mut state, P0, 3, 3, KeywordSet::FIRST_STRIKE);
    attack(&mut state, bear, P1);
    attack(&mut state, knight, P1);
    deal_combat_damage(&mut state, true);
    assert_eq!(life(&state, P1), 17, "only the knight's three");
    deal_combat_damage(&mut state, false);
    assert_eq!(
        life(&state, P1),
        15,
        "the bear's two come in the regular step"
    );
}

#[test]
fn a_first_striker_kills_its_blocker_before_the_blocker_strikes() {
    let mut state = empty_state();
    let knight = creature(&mut state, P0, 2, 2, KeywordSet::FIRST_STRIKE);
    let bear = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, knight, P1);
    block(&mut state, bear, knight);
    deal_combat_damage(&mut state, true);
    assert_eq!(damage(&state, bear), 2);
    assert_eq!(damage(&state, knight), 0);
    crate::sba::run(&mut state, &NoCards);
    assert!(!on_battlefield(&state, bear));
    deal_combat_damage(&mut state, false);
    assert_eq!(damage(&state, knight), 0, "a dead blocker deals nothing");
    assert!(on_battlefield(&state, knight));
}

#[test]
fn an_unblocked_double_striker_hits_the_player_twice() {
    let mut state = empty_state();
    let striker = creature(&mut state, P0, 3, 3, KeywordSet::DOUBLE_STRIKE);
    attack(&mut state, striker, P1);
    deal_combat_damage(&mut state, true);
    assert_eq!(life(&state, P1), 17);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 14);
}

#[test]
fn attackers_on_two_players_each_hit_their_own_target() {
    let mut state = seated(3);
    let p2 = PlayerId::new(2);
    let one = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let two = creature(&mut state, P0, 5, 5, KeywordSet::EMPTY);
    attack(&mut state, one, P1);
    attack(&mut state, two, p2);
    deal_combat_damage(&mut state, false);
    assert_eq!(life(&state, P1), 18);
    assert_eq!(life(&state, p2), 15);
    assert_eq!(life(&state, P0), 20);
}

#[test]
fn combat_damage_is_marked_and_kills_only_after_the_check() {
    let mut state = empty_state();
    let attacker = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
    let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
    attack(&mut state, attacker, P1);
    block(&mut state, blocker, attacker);
    deal_combat_damage(&mut state, false);
    assert!(on_battlefield(&state, attacker), "damage is only marked");
    assert!(on_battlefield(&state, blocker));
    crate::sba::run(&mut state, &NoCards);
    assert!(!on_battlefield(&state, attacker), "and kills at the check");
    assert!(!on_battlefield(&state, blocker));
}
