//! `cards/instants/mv_3/rush_of_inspiration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rush of Inspiration // Crackling Falls (`Coverage::Partial`): "Draw two
/// cards. Then discard a card at random unless you pay {E}{E}. // This land
/// enters tapped. {T}: Add {U} or {R}."
///
/// Under `Coverage::Partial`, the energy payment and random discard are not
/// expressible in the DSL, but the "draw two cards" effect is implemented.
/// The test casts Rush of Inspiration off three Islands and verifies the
/// caster's hand grows by one card after spending the spell.
#[test]
fn rush_of_inspiration_draws_two_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(44, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[rush_of_inspiration()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, rush_of_inspiration());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "casting costs one card from hand and draws two cards, giving a net gain of one"
    );
    assert!(
        in_graveyard(&engine, p0, rush_of_inspiration()).is_some(),
        "resolved spell is in the graveyard"
    );
}
