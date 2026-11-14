//! Owner-reported life/stack cases, 14 November 2026.
//!
//! CR 119.4 permits spending the last life; CR 704.3/704.5a check losses
//! before priority, not after the whole stack. Two opposing Sheoldreds
//! therefore need both APNAP orders, including an intermediate zero total.
use super::testkit::{
    Duel, RegistryLookup, basic_forest, card_index, cast_from_hand, keep_mulligans, on_battlefield,
    pass_until, stack_is_empty,
};
use super::{Engine, GameEvent, Pending, Phase, PlayerAction, PlayerId};
use crate::event::LossReason;
use crate::win::Victor;
use baylee_core::ids::CardIndex;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
type Game = Engine<RegistryLookup>;

fn sheoldred() -> CardIndex {
    card_index("34f34409-326d-4994-a0ea-1a69aa278f03")
}

fn angel() -> CardIndex {
    card_index("b148578c-c0bf-4785-b97c-4b6f83028008")
}

fn life_changes(game: &Game, since: usize) -> Vec<i32> {
    game.journal().entries()[since..]
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::LifeChanged { player, new, .. } if player == P0 => Some(new),
            _ => None,
        })
        .collect()
}

/// A real four-card draw on the named player's turn. Only actual game
/// actions advance the scenario; the low life total is part of setup.
fn four_draws(active: PlayerId, life: i32, protected: bool, order: bool) -> (Game, usize) {
    let island = card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373");
    let opportunity = card_index("1f544a9f-c238-4858-be19-d6cd7d023dcc");
    let mut board = vec![island; 6];
    board.push(sheoldred());
    if protected {
        board.push(angel());
    }
    let mut game = Duel::new(0x51e0_1d4e, basic_forest())
        .battlefield(0, &board)
        .battlefield(1, &[sheoldred()])
        .hand(0, &[opportunity])
        .life(0, life)
        .without_capabilities()
        .start();
    game.house_rules.order_triggers = order;
    keep_mulligans(&mut game);
    pass_until(&mut game, |e| {
        e.state().turn.active == active
            && e.state().turn.phase == Phase::FirstMain
            && stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let since = game.journal().len();
    cast_from_hand(&mut game, P0, opportunity);
    game.apply(P0, PlayerAction::ChoosePlayer(P0))
        .expect("Opportunity targets its controller");
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::GameOver(_)) || stack_is_empty(e)
    });
    (game, since)
}

#[test]
fn opposing_sheoldred_triggers_kill_before_the_own_turn_healing() {
    for order in [false, true] {
        let (game, since) = four_draws(P0, 8, false, order);
        assert_eq!(life_changes(&game, since), [6, 4, 2, 0]);
        assert_eq!(game.state().players[0].loss, Some(LossReason::Life));
        assert!(matches!(game.pending(), Pending::GameOver(result)
            if result.winner == Some(Victor::Player(P1))));
    }
}

#[test]
fn surviving_the_four_losses_allows_the_later_four_gains() {
    for order in [false, true] {
        let (game, since) = four_draws(P0, 9, false, order);
        assert_eq!(life_changes(&game, since), [7, 5, 3, 1, 3, 5, 7, 9]);
        assert!(!game.state().players[0].has_lost());
    }
}

#[test]
fn drawing_in_the_opponents_turn_resolves_the_gains_first() {
    for order in [false, true] {
        let (game, since) = four_draws(P1, 2, false, order);
        assert_eq!(life_changes(&game, since), [4, 6, 8, 10, 8, 6, 4, 2]);
        assert!(!game.state().players[0].has_lost());
    }
}

#[test]
fn cannot_lose_keeps_zero_life_alive_until_the_healing_resolves() {
    let (game, since) = four_draws(P0, 8, true, true);
    assert_eq!(life_changes(&game, since), [6, 4, 2, 0, 2, 4, 6, 8]);
    assert!(!game.state().players[0].has_lost());
}

#[test]
fn a_fetchland_can_spend_the_last_life_but_does_not_resolve_after_defeat() {
    let fetch = card_index("29737a60-3ebd-40d9-b935-c4f54b90d45d");
    let mut game = Duel::new(0x1a57_11fe, basic_forest())
        .battlefield(0, &[fetch])
        .life(0, 1)
        .without_capabilities()
        .start();
    keep_mulligans(&mut game);
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let source = on_battlefield(&game, P0, fetch).expect("the fetchland");
    assert!(matches!(game.pending(), Pending::Priority { legal, .. }
        if legal.abilities.contains(&(source, 0))));
    let since = game.journal().len();
    game.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: 0,
        },
    )
    .expect("CR 119.4: exactly one life pays one life");
    assert_eq!(life_changes(&game, since), [0]);
    assert_eq!(game.state().players[0].loss, Some(LossReason::Life));
    assert!(matches!(game.pending(), Pending::GameOver(_)));
    assert!(
        !game.journal().entries()[since..]
            .iter()
            .any(|entry| { matches!(entry.event, GameEvent::Shuffled { .. }) }),
        "the search/shuffle ability has not resolved"
    );
}
