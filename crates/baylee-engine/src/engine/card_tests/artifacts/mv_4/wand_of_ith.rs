//! `cards/artifacts/mv_4/wand_of_ith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;

fn wand_of_ith() -> CardIndex {
    card_index("3ec1ccd7-4970-4032-be7a-9e76ca6a139e")
}

fn serra_angel() -> CardIndex {
    card_index("4b7ac066-e5c7-43e6-9e7e-2739b24a905d")
}

fn life(engine: &Engine<RegistryLookup>, seat: PlayerId) -> i32 {
    engine.state().players[seat.get() as usize].life
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

fn graveyard_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(seat))
        .len()
}

/// Names `victim` for "target player", whichever way the engine asks.
#[track_caller]
fn name_player(engine: &mut Engine<RegistryLookup>, seat: PlayerId, victim: PlayerId) {
    match engine.pending().clone() {
        Pending::ChoosePlayer { .. } => {
            engine
                .apply(seat, PlayerAction::ChoosePlayer(victim))
                .expect("the player was on the offer");
        }
        Pending::ChooseTargets { .. } => {
            engine
                .apply(
                    seat,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![victim],
                    },
                )
                .expect("the player was on the offer");
        }
        other => panic!("expected the player question, got {other:?}"),
    }
}

/// p0 has the Wand and six Forests; p1's whole hand is `held`. p0's main.
fn a_wand_over_a_hand(held: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut mine = vec![wand_of_ith()];
    mine.extend([forest(); 6]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &mine)
        .hand(1, held)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(hand_size(&engine, PlayerId::new(1)), held.len());
    engine
}

/// Activates the Wand on p1 and runs to p1's life-payment question.
#[track_caller]
fn wand_on_p1_until_asked(engine: &mut Engine<RegistryLookup>) -> YesNoPrompt {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let seq_before = engine.state().journal.last_seq();
    tap_mana_where(engine, p0, |_| true);
    activate(engine, p0, wand_of_ith(), 0);
    name_player(engine, p0, p1);
    pass_until(engine, |e| matches!(e.pending(), Pending::YesNo { .. }));
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p1, "the revealed card's owner decides");
    let revealed = engine.state().journal.entries().iter().any(|entry| {
        entry.seq > seq_before
            && matches!(
                entry.event,
                crate::event::GameEvent::Revealed { player: revealed_by, .. } if revealed_by == p1
            )
    });
    assert!(revealed, "the card was revealed");
    prompt
}

/// Wand of Ith (3ec1ccd7-4970-4032-be7a-9e76ca6a139e) — "{3}, {T}: Target
/// player reveals a card at random from their hand. If it's a land card, that
/// player discards it unless they pay 1 life. If it isn't a land card, the
/// player discards it unless they pay life equal to its mana value. Activate
/// only during your turn."
///
/// A land, the price declined: it is discarded and no life is lost.
#[test]
fn wand_of_ith_land_declined_is_discarded() {
    let p1 = PlayerId::new(1);
    let mut engine = a_wand_over_a_hand(&[forest()]);
    let wand = on_battlefield(&engine, PlayerId::new(0), wand_of_ith()).expect("Wand");
    let prompt = wand_on_p1_until_asked(&mut engine);
    assert!(is_tapped(&engine, wand), "{{T}} was paid");
    assert_eq!(prompt, YesNoPrompt::PayLife { amount: 1 }, "a land costs 1");
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p1), 0);
    assert_eq!(graveyard_size(&engine, p1), 1, "discarded");
    assert_eq!(life(&engine, p1), 20);
}

/// A land, the 1 life paid: the card stays in hand.
#[test]
fn wand_of_ith_land_paid_stays_in_hand() {
    let p1 = PlayerId::new(1);
    let mut engine = a_wand_over_a_hand(&[forest()]);
    let prompt = wand_on_p1_until_asked(&mut engine);
    assert_eq!(prompt, YesNoPrompt::PayLife { amount: 1 });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p1), 1, "kept");
    assert_eq!(graveyard_size(&engine, p1), 0);
    assert_eq!(life(&engine, p1), 19);
}

/// A nonland card costs life equal to its mana value: Serra Angel, 5. Paid,
/// the Angel stays and p1 is at 15.
#[test]
fn wand_of_ith_nonland_paid_costs_its_mana_value() {
    let p1 = PlayerId::new(1);
    let mut engine = a_wand_over_a_hand(&[serra_angel()]);
    let prompt = wand_on_p1_until_asked(&mut engine);
    assert_eq!(prompt, YesNoPrompt::PayLife { amount: 5 });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 15);
    assert!(in_hand(&engine, p1, serra_angel()).is_some(), "kept");
}

/// The Angel's price declined: it is discarded, no life lost.
#[test]
fn wand_of_ith_nonland_declined_is_discarded() {
    let p1 = PlayerId::new(1);
    let mut engine = a_wand_over_a_hand(&[serra_angel()]);
    wand_on_p1_until_asked(&mut engine);
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), 20);
    assert!(in_graveyard(&engine, p1, serra_angel()).is_some());
    assert_eq!(hand_size(&engine, p1), 0);
}

/// "Reveals a card at random": with a hand of one land and one nonland, the
/// question is for whichever was revealed, and the other card is never
/// touched, whatever the answer.
#[test]
fn wand_of_ith_touches_only_the_revealed_card() {
    let p1 = PlayerId::new(1);
    let mut engine = a_wand_over_a_hand(&[forest(), serra_angel()]);
    let prompt = wand_on_p1_until_asked(&mut engine);
    let revealed_is_land = match prompt {
        YesNoPrompt::PayLife { amount: 1 } => true,
        YesNoPrompt::PayLife { amount: 5 } => false,
        other => panic!("a land costs 1 and the Angel 5, got {other:?}"),
    };
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine, p1), 1, "one card discarded, one kept");
    assert_eq!(graveyard_size(&engine, p1), 1);
    assert_eq!(
        in_hand(&engine, p1, forest()).is_some(),
        !revealed_is_land,
        "the unrevealed card is the one left in hand"
    );
}

/// "Activate only during your turn." On p0's turn the Wand is on offer; on
/// p1's turn, with three untapped Forests, it is not.
#[test]
fn wand_of_ith_is_only_for_its_controllers_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_wand_over_a_hand(&[forest()]);
    let wand = on_battlefield(&engine, p0, wand_of_ith()).expect("Wand");
    let forests = all_on_battlefield(&engine, p0, forest());
    let first_three = forests[..3].to_vec();
    tap_mana_where(&mut engine, p0, |id| first_three.contains(&id));
    assert!(
        priority_offer(&engine).abilities.contains(&(wand, 0)),
        "paid for, on its controller's turn: offered"
    );

    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "p0 holds priority in p1's main phase"
    );
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0].mana_pool.total() >= 3,
        "{{3}} is floating"
    );
    assert!(
        !priority_offer(&engine).abilities.contains(&(wand, 0)),
        "not on the opponent's turn"
    );
}

/// An empty hand reveals nothing: no question is asked and nothing is lost.
#[test]
fn wand_of_ith_against_an_empty_hand_does_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_wand_over_a_hand(&[]);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, wand_of_ith(), 0);
    name_player(&mut engine, p0, p1);
    pass_until(&mut engine, |e| {
        stack_is_empty(e) || matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "nothing was revealed, so nobody is asked to pay"
    );
    assert_eq!(life(&engine, p1), 20);
}
