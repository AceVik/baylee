//! Guerrilla Tactics — `{1}{R}` Instant (Alliances)
//!
//! Guerrilla Tactics deals 2 damage to any target.
//! When a spell or ability an opponent controls causes you to discard this card,
//! it deals 4 damage to any target.

#[allow(clippy::wildcard_imports)]
use super::*;

fn guerrilla_tactics() -> CardIndex {
    card_index("65ad90f5-4dae-494b-83fb-8052e77e0b12")
}

fn thoughtseize() -> CardIndex {
    card_index("edd8d1e8-be43-4c38-bb3a-83081fbaf0b5")
}

#[test]
fn guerrilla_tactics_cast_deals_2_damage_and_discarded_by_opponent_deals_4_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[guerrilla_tactics(), guerrilla_tactics()])
        .battlefield(0, &[mountain(), mountain()])
        .hand(1, &[thoughtseize()])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. Cast Guerrilla Tactics normally -> deals 2 damage to p1.
    let gt1 = in_hand(&engine, p0, guerrilla_tactics()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: gt1 })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 18);

    // 2. Advance to p1's main phase. p1 casts Thoughtseize targeting p0, discarding the second Guerrilla Tactics.
    reach_their_main_phase(&mut engine, p1);
    let ts = in_hand(&engine, p1, thoughtseize()).unwrap();
    let gt2 = in_hand(&engine, p0, guerrilla_tactics()).unwrap();

    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ts })
        .unwrap();
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p1, PlayerAction::ChooseObjects { objects: vec![gt2] })
        .unwrap();

    // Thoughtseize also makes p1 lose 2 life (to 16), and Guerrilla Tactics's DiscardedThisByOpponent trigger asks p0 for any target.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        12,
        "18 - 2 (Thoughtseize) - 4 (Guerrilla Tactics discard trigger) = 12"
    );
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// "Deals 2 damage to any target": a creature is a target too, it takes 2 (a Hill Giant survives
/// them) and the player is not hit.
#[test]
fn guerrilla_tactics_deals_two_damage_to_a_creature_and_nothing_to_the_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[guerrilla_tactics()])
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    cast_from_hand(&mut engine, p0, guerrilla_tactics());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().damage, 2);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
}

/// "A spell or ability an opponent controls causes you to discard this card": the opponent's
/// Thoughtseize takes the Grizzly Bears instead, so the Tactics stays in hand and nothing fires.
#[test]
fn guerrilla_tactics_does_not_fire_when_the_opponent_makes_you_discard_another_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[guerrilla_tactics(), grizzly_bears()])
        .hand(1, &[thoughtseize()])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    let ts = in_hand(&engine, p1, thoughtseize()).unwrap();
    let bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: ts })
        .unwrap();
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(in_hand(&engine, p0, guerrilla_tactics()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        18,
        "only Thoughtseize's own price"
    );
}

/// ... and a discard you choose yourself (down to seven cards at cleanup) is no opponent's spell:
/// the Tactics goes to the graveyard and nothing is asked or dealt.
#[test]
fn guerrilla_tactics_does_not_fire_when_you_discard_it_yourself() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .hand(
            0,
            &[
                guerrilla_tactics(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let turn = engine.state().turn.number;

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::DiscardChoice { .. }) || e.state().turn.number > turn
    });
    let Pending::DiscardChoice { player, count } = engine.pending().clone() else {
        panic!("eight or more cards in hand must ask for a discard")
    };
    assert_eq!(player, p0);
    let tactics = in_hand(&engine, p0, guerrilla_tactics()).unwrap();
    let mut cards = vec![tactics];
    cards.extend(
        engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .iter()
            .copied()
            .filter(|c| *c != tactics)
            .take(usize::from(count) - 1),
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: cards })
        .unwrap();

    // A trigger would stop this walk at its target question.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    assert!(in_graveyard(&engine, p0, guerrilla_tactics()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
