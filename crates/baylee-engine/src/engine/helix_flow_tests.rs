//! A spell with a targeted and an untargeted half, with a real card
//! (Lightning Helix, {R}{W}: 3 damage to any target and you gain 3 life).
//! Both halves happen when it resolves; when its only target is illegal it
//! does not resolve at all and the life is not gained either (CR 608.2b).

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0x4e11_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn helix() -> CardIndex {
    card_index("800c258a-cfc4-4a54-a667-065ea8dea69e")
}

fn duel() -> Game {
    let mountain = card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685");
    let plains = card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99");
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[helix()])
        .battlefield(0, &[mountain, plains])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn cast(engine: &mut Game, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, helix()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(P0, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

fn life(engine: &Game, seat: PlayerId) -> i32 {
    engine.state().players[usize::from(seat.get())].life
}

#[test]
fn helix_at_a_player_deals_three_and_gains_three() {
    let mut engine = duel();
    cast(&mut engine, vec![], vec![P1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P1), 17);
    assert_eq!(life(&engine, P0), 23);
    assert!(in_graveyard(&engine, P0, helix()).is_some());
}

#[test]
fn helix_at_a_creature_still_gains_its_caster_three() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    cast(&mut engine, vec![elf], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_none());
    assert_eq!(life(&engine, P0), 23);
    assert_eq!(life(&engine, P1), 20);
}

#[test]
fn helix_at_oneself_nets_nothing() {
    let mut engine = duel();
    cast(&mut engine, vec![], vec![P0]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, P0), 20);
}

#[test]
fn helix_whose_target_left_gains_no_life() {
    let mut engine = duel();
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    cast(&mut engine, vec![elf], vec![]);
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .move_object(
            elf,
            crate::zone::ZoneLocation::Hand(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life(&engine, P0),
        20,
        "a spell that does not resolve does nothing"
    );
    assert!(in_graveyard(&engine, P0, helix()).is_some());
}
