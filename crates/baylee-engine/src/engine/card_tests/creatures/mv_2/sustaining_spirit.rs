//! `cards/creatures/mv_2/sustaining_spirit.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sustaining_spirit() -> CardIndex {
    card_index("683c81ae-38cd-4db1-b7e7-a09d34b6431e")
}

fn soul_feast() -> CardIndex {
    baylee_core::generated::index::SOUL_FEAST
}

/// The price the pending "pay?" question names, as a mana value.
fn price(engine: &Engine<RegistryLookup>) -> u32 {
    let Pending::YesNo {
        prompt: crate::choice::YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    else {
        panic!("a payment question, got {:?}", engine.pending())
    };
    cost.cmc()
}

/// A game on P0's first main phase with Sustaining Spirit in play and its first
/// cumulative upkeep ({1}{W}) paid. P0 holds Lightning Bolt and one floating {R}.
fn spirit_game(
    life0: i32,
    life1: i32,
    p1_board: &[CardIndex],
    p1_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .life(0, life0)
        .life(1, life1)
        .battlefield(0, &[sustaining_spirit(), plains(), plains(), mountain()])
        .battlefield(1, p1_board)
        .hand(0, &[lightning_bolt()])
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    // Upkeep: pay cumulative upkeep {1}{W} for the first age counter.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(price(&engine), 2, "{{1}}{{W}} for the one age counter");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    engine.refresh_offer();
    engine
}

/// P0 casts Lightning Bolt at `target`.
fn bolt(engine: &mut Engine<RegistryLookup>, target: PlayerId) {
    let p0 = PlayerId::new(0);
    let bolt = in_hand(engine, p0, lightning_bolt()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![target],
            },
        )
        .unwrap();
}

/// Sustaining Spirit ({1}{W} 0/3 Creature — Angel Spirit):
/// - "Damage that would reduce your life total to less than 1 reduces it to 1 instead."
#[test]
fn sustaining_spirit_floors_damage_life_loss_at_one_and_only_when_lethal() {
    let p0 = PlayerId::new(0);
    // 3 damage at 2 life leaves 1; at 6 life it leaves 3 (the floor only
    // matters when the damage would reduce the total to less than 1).
    for (start, expected) in [(2, 1), (6, 3)] {
        let mut engine = spirit_game(start, 20, &[], &[]);
        bolt(&mut engine, p0);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().players[0].life, expected, "from {start}");
        assert_eq!(engine.state().players[1].life, 20);
    }
}

/// "your life total": the opponent at 2 life is not protected, the same
/// Lightning Bolt takes them below 1 and they lose.
#[test]
fn sustaining_spirit_does_not_protect_the_opponent() {
    let p1 = PlayerId::new(1);
    let mut engine = spirit_game(20, 2, &[], &[]);
    bolt(&mut engine, p1);
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert_eq!(engine.state().players[1].life, -1);
    assert_eq!(engine.state().players[0].life, 20);
}

/// "Damage that would reduce...": life loss that is not damage is not floored.
#[test]
fn sustaining_spirit_does_not_floor_life_loss_that_is_not_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = spirit_game(2, 20, &[swamp(); 5], &[soul_feast()]);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, soul_feast());
    let Pending::ChoosePlayer { player, options } = engine.pending() else {
        panic!("Soul Feast asks for a player, got {:?}", engine.pending())
    };
    assert_eq!(*player, p1);
    assert!(options.contains(&p0));
    engine.apply(p1, PlayerAction::ChoosePlayer(p0)).unwrap();
    pass_until(&mut engine, |e| matches!(e.pending(), Pending::GameOver(_)));
    assert_eq!(engine.state().players[0].life, -2);
}

/// Sustaining Spirit — "Cumulative upkeep {1}{W}": one more {1}{W} each of your
/// upkeeps, and the creature is sacrificed when the price is not paid.
#[test]
fn sustaining_spirit_is_sacrificed_when_its_cumulative_upkeep_is_not_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[sustaining_spirit(), plains(), plains(), plains(), plains()],
        )
        .start();
    keep_mulligans(&mut engine);
    let spirit = on_battlefield(&engine, p0, sustaining_spirit()).unwrap();

    // Upkeeps one and two are paid: {1}{W}, then {1}{W}{1}{W}, each with an
    // age counter more on the creature.
    for age in [1u16, 2] {
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
        });
        assert_eq!(
            price(&engine),
            2 * u32::from(age),
            "{{1}}{{W}} per age counter"
        );
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        tap_all_mana(&mut engine, p0);
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
        reach_main_phase(&mut engine, p0);
        let object = engine.state().object(spirit).expect("still in play");
        assert_eq!(object.zone, Zone::Battlefield, "paid upkeep {age}");
        assert_eq!(
            object.counters.get(baylee_cards_dsl::counters::AGE),
            age,
            "age counters after upkeep {age}"
        );
        // Through the opponent's turn to P0's next upkeep.
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
        });
    }

    // Upkeep three asks for six mana, of which there are four: declined.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        price(&engine),
        6,
        "{{1}}{{W}} for each of three age counters"
    );
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, sustaining_spirit()).is_some());
}
