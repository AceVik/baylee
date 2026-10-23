//! `cards/creatures/mv_4/emberwilde_djinn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn emberwilde_djinn() -> CardIndex {
    card_index("3e1a7146-75d5-499f-8718-303bc6ea139b")
}

/// P0's Emberwilde Djinn is in play from the start; both players have two
/// Mountains, so neither question is skipped for want of mana.
/// The game stands at the mulligan's end, before P0's first upkeep.
fn djinn_game() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[emberwilde_djinn(), mountain(), mountain()])
        .battlefield(1, &[mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    let djinn = on_battlefield(&engine, P0, emberwilde_djinn()).unwrap();
    (engine, djinn)
}

/// Runs to the next payment question, which must be `seat`'s on turn `turn`.
#[track_caller]
fn next_question(engine: &mut Engine<RegistryLookup>, seat: PlayerId, turn: u32) {
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, .. } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, seat, "whose question");
    assert_eq!(engine.state().turn.number, turn, "which turn");
}

/// Answers both questions with no: the {R}{R} one, then the 2-life one.
#[track_caller]
fn decline_both(engine: &mut Engine<RegistryLookup>, seat: PlayerId, turn: u32) {
    next_question(engine, seat, turn);
    lk::yes_no(engine, false);
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, .. } if *player == seat),
        "declining the mana asks about the life next: {:?}",
        engine.pending()
    );
    lk::yes_no(engine, false);
}

fn controller(engine: &Engine<RegistryLookup>, id: ObjectId) -> PlayerId {
    engine.state().object(id).unwrap().controller
}

/// Emberwilde Djinn (3e1a7146-75d5-499f-8718-303bc6ea139b) — {2}{R}{R} 5/4:
/// "Flying. At the beginning of each player's upkeep, that player may pay
/// {R}{R} or 2 life. If the player does, they gain control of this creature."
#[test]
fn emberwilde_djinn_is_a_five_four_flyer() {
    let (engine, djinn) = djinn_game();

    assert_eq!(pt(&engine, djinn), (5, 4));
    assert!(
        engine
            .state()
            .object(djinn)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );
}

/// The opponent pays {R}{R} at their upkeep and takes the Djinn; no life is
/// paid for it.
#[test]
fn emberwilde_djinn_goes_to_the_player_who_pays_red_red() {
    let (mut engine, djinn) = djinn_game();
    decline_both(&mut engine, P0, 1);
    assert_eq!(controller(&engine, djinn), P0);

    next_question(&mut engine, P1, 2);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 2)]);
    lk::yes_no(&mut engine, true);

    assert_eq!(controller(&engine, djinn), P1);
    assert_eq!(engine.state().players[1].life, 20, "mana, not life");
}

/// "{R}{R} or 2 life": declining the mana, the opponent pays 2 life instead
/// and takes the Djinn.
#[test]
fn emberwilde_djinn_goes_to_the_player_who_pays_two_life() {
    let (mut engine, djinn) = djinn_game();
    decline_both(&mut engine, P0, 1);

    next_question(&mut engine, P1, 2);
    lk::yes_no(&mut engine, false);
    assert_eq!(controller(&engine, djinn), P0, "declining the mana alone");
    assert!(matches!(
        engine.pending(),
        Pending::YesNo { player, .. } if *player == P1
    ));
    lk::yes_no(&mut engine, true);

    assert_eq!(controller(&engine, djinn), P1);
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 20);
}

/// A player who pays nothing keeps their life and leaves the Djinn where it
/// was.
#[test]
fn emberwilde_djinn_stays_when_the_player_pays_nothing() {
    let (mut engine, djinn) = djinn_game();
    decline_both(&mut engine, P0, 1);
    decline_both(&mut engine, P1, 2);

    assert_eq!(controller(&engine, djinn), P0);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
}

/// "each player's upkeep": the first owner is asked again at their own next
/// upkeep and takes the Djinn back by paying life.
#[test]
fn emberwilde_djinn_is_taken_back_at_the_next_players_upkeep() {
    let (mut engine, djinn) = djinn_game();
    decline_both(&mut engine, P0, 1);
    next_question(&mut engine, P1, 2);
    lk::float(&mut engine, P1, &[(ManaColor::Red, 2)]);
    lk::yes_no(&mut engine, true);
    assert_eq!(controller(&engine, djinn), P1);

    next_question(&mut engine, P0, 3);
    lk::yes_no(&mut engine, false);
    lk::yes_no(&mut engine, true);

    assert_eq!(controller(&engine, djinn), P0);
    assert_eq!(engine.state().players[0].life, 18);
}
