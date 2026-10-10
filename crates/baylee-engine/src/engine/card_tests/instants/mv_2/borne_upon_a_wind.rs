//! `cards/instants/mv_2/borne_upon_a_wind.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn borne_upon_a_wind_draws_the_top_card_and_lands_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(83, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[borne_upon_a_wind()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Library(p0))
        .clone();
    let top = *library_before
        .last()
        .expect("p0 has a library to draw from");
    let hand_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();
    assert!(
        in_hand(&engine, p0, borne_upon_a_wind()).is_some(),
        "the instant starts in hand"
    );

    cast_from_hand(&mut engine, p0, borne_upon_a_wind());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, borne_upon_a_wind()).is_some(),
        "an instant that has resolved is put into its owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before.len() - 1,
        "\"Draw a card\" is exactly one off the top, and not two"
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .contains(&top),
        "the card drawn is the one that was on top of the library"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand_before,
        "the spell left the hand and one card replaced it — a draw of zero \
         would leave one fewer"
    );
}

#[test]
fn borne_allows_the_creature_it_draws_to_be_cast_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, llanowar_elves())
        .hand(0, &[borne_upon_a_wind()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    let drawn = *e
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Blue, 2);
    pool.add(ManaColor::Green, 1);
    e.refresh_offer();
    let wind = in_hand(&e, p0, borne_upon_a_wind()).unwrap();
    e.apply(p0, PlayerAction::CastSpell { card: wind }).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(drawn).unwrap().zone, Zone::Hand);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    e.apply(p0, PlayerAction::CastSpell { card: drawn })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(drawn).unwrap().zone, Zone::Battlefield);
}
