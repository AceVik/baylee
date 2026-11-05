//! Two keywords that bend timing, with real cards: Zealous Guardian
//! ({W/U}, flash) may be cast whenever its controller could cast an
//! instant (CR 702.8a), and Mountain Bandit ({R}, haste) may attack the
//! turn it arrives (CR 702.10b). Each is checked against a creature
//! without the keyword.

use super::testkit::{
    Duel, basic_forest, card_index, cast_from_hand, in_hand, keep_mulligans, on_battlefield,
    pass_until, quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender};

const SEED: u64 = 0xf1a5_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn guardian() -> CardIndex {
    card_index("508fdfb1-87d6-4342-a05a-01158f253896")
}

fn bandit() -> CardIndex {
    card_index("e3638c54-e494-47d2-b614-94c9d9add85a")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn duel(hand: &[CardIndex], lands: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, hand)
        .battlefield(0, lands)
        .start();
    keep_mulligans(&mut engine);
    engine
}

fn priority_in_opponents_turn(engine: &mut Game) {
    pass_until(engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
}

#[test]
fn a_flash_creature_is_cast_in_the_opponents_turn() {
    let mut engine = duel(&[guardian()], &[plains()]);
    priority_in_opponents_turn(&mut engine);
    cast_from_hand(&mut engine, P0, guardian());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, guardian()).is_some());
}

#[test]
fn a_creature_without_flash_is_not() {
    let mut engine = duel(&[quiet_creature()], &[basic_forest()]);
    priority_in_opponents_turn(&mut engine);
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, quiet_creature()).unwrap();
    assert!(engine.apply(P0, PlayerAction::CastSpell { card }).is_err());
}

#[test]
fn a_flash_creature_is_cast_over_a_spell_on_the_stack() {
    let mut engine = duel(&[quiet_creature(), guardian()], &[basic_forest(), plains()]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    super::testkit::cast_with_floating(&mut engine, P0, quiet_creature());
    super::testkit::cast_with_floating(&mut engine, P0, guardian());
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .len(),
        2
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, guardian()).is_some());
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_some());
}

#[test]
fn a_haste_creature_attacks_the_turn_it_arrives() {
    let mut engine = duel(&[bandit()], &[mountain()]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    cast_from_hand(&mut engine, P0, bandit());
    pass_until(&mut engine, stack_is_empty);
    let rogue = on_battlefield(&engine, P0, bandit()).unwrap();
    assert!(crate::combat::can_attack(engine.state(), P0, rogue));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(rogue, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 19);
}

#[test]
fn a_creature_without_haste_cannot_attack_the_turn_it_arrives() {
    let mut engine = duel(&[quiet_creature()], &[basic_forest()]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    cast_from_hand(&mut engine, P0, quiet_creature());
    pass_until(&mut engine, stack_is_empty);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    assert!(!crate::combat::can_attack(engine.state(), P0, elf));
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}

fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn shred_freak() -> CardIndex {
    card_index("e900b2b5-4bb2-482d-89ee-b5bbaf246698")
}

fn castable_with(card: CardIndex, lands: &[CardIndex]) -> bool {
    let mut engine = duel(&[card], lands);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    let spell = in_hand(&engine, P0, card).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .is_ok()
}

/// A hybrid symbol is paid with either of its colours (CR 107.4e).
#[test]
fn a_hybrid_symbol_takes_either_colour_and_no_other() {
    assert!(castable_with(guardian(), &[plains()]));
    assert!(castable_with(guardian(), &[island()]));
    assert!(!castable_with(guardian(), &[basic_forest()]));
}

#[test]
fn two_hybrid_symbols_take_one_of_each_or_two_of_one() {
    assert!(castable_with(shred_freak(), &[swamp(), mountain()]));
    assert!(castable_with(shred_freak(), &[swamp(), swamp()]));
    assert!(castable_with(shred_freak(), &[mountain(), mountain()]));
    assert!(!castable_with(shred_freak(), &[swamp(), basic_forest()]));
    assert!(!castable_with(shred_freak(), &[swamp()]));
}
