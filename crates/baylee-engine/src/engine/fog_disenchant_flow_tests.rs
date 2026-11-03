//! Two answers with real cards: Fog ({G}, prevent all combat damage this
//! turn) cast during combat, and Naturalize ({1}{G}, destroy target
//! artifact or enchantment) against an Equipment. Fog stops
//! damage in both directions and only this turn (CR 615.1a); Naturalize
//! may target only what it names (CR 115.1a).

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, quiet_creature, stack_is_empty, tap_mana_where,
};
use super::*;
use baylee_core::ids::{CardIndex, Defender, ObjectId};

const SEED: u64 = 0xf064_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn fog() -> CardIndex {
    card_index("27e9db49-7af7-4bef-ad4c-bf5dfb92030d")
}

fn naturalize() -> CardIndex {
    card_index("bdb3ca68-ec1f-4e16-81cc-d23f8f52c728")
}

fn scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

fn troll() -> CardIndex {
    card_index("c2f86723-4765-4dac-a9f6-9cfa543613b2")
}

fn lands_of(engine: &Game, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == basic_forest())
            })
        })
        .collect()
}

/// Seat 0 attacks with a troll; seat 1 blocks with an elf and casts Fog
/// in the declare blockers step.
fn fogged_combat(block: bool) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(1, &[fog()])
        .battlefield(0, &[troll()])
        .battlefield(1, &[basic_forest(), quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    let attacker = on_battlefield(&engine, P0, troll()).unwrap();
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let blockers = if block { vec![(elf, attacker)] } else { vec![] };
    engine
        .apply(P1, PlayerAction::DeclareBlockers { blockers })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::DeclareBlockers
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    let forests = lands_of(&engine, P1);
    tap_mana_where(&mut engine, P1, |id| forests.contains(&id));
    let card = in_hand(&engine, P1, fog()).unwrap();
    engine.apply(P1, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    engine
}

#[test]
fn fog_keeps_an_unblocked_attacker_off_the_player() {
    let engine = fogged_combat(false);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fog_saves_both_sides_of_a_block() {
    let engine = fogged_combat(true);
    assert!(on_battlefield(&engine, P1, quiet_creature()).is_some());
    assert!(on_battlefield(&engine, P0, troll()).is_some());
}

#[test]
fn fog_lasts_only_this_turn() {
    let mut engine = fogged_combat(false);
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(engine.state().shields.is_empty());
}

fn naturalize_duel(theirs: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[naturalize()])
        .battlefield(0, &[basic_forest(), basic_forest()])
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    engine
}

fn naturalize_options(engine: &mut Game) -> Vec<ObjectId> {
    let forests = lands_of(engine, P0);
    tap_mana_where(engine, P0, |id| forests.contains(&id));
    let card = in_hand(engine, P0, naturalize()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    match engine.pending() {
        Pending::ChooseTargets { options, .. } => options.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn naturalize_destroys_an_equipment() {
    let mut engine = naturalize_duel(&[scimitar()]);
    let blade = on_battlefield(&engine, P1, scimitar()).unwrap();
    assert!(naturalize_options(&mut engine).contains(&blade));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![blade],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, P1, scimitar()).is_some());
}

#[test]
fn naturalize_does_not_offer_a_creature_or_a_land() {
    let mut engine = naturalize_duel(&[quiet_creature(), basic_forest(), scimitar()]);
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    let land = on_battlefield(&engine, P1, basic_forest()).unwrap();
    let options = naturalize_options(&mut engine);
    assert!(!options.contains(&elf));
    assert!(!options.contains(&land));
}

/// Seat 0 pacifies seat 1's elf; in its own turn seat 1 naturalizes the
/// Pacifism, and the elf may attack again.
#[test]
fn destroying_pacifism_frees_the_creature() {
    let pacifism = card_index("5f5e0b10-c8cf-450c-bfd3-bcb0528ec330");
    let plains = card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99");
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, &[pacifism])
        .hand(1, &[naturalize()])
        .battlefield(0, &[plains, plains])
        .battlefield(1, &[quiet_creature(), basic_forest(), basic_forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let elf = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    super::testkit::tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, pacifism).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, P0, pacifism).expect("Pacifism");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    assert!(!crate::combat::can_attack(engine.state(), P1, elf));
    let forests = lands_of(&engine, P1);
    tap_mana_where(&mut engine, P1, |id| forests.contains(&id));
    let card = in_hand(&engine, P1, naturalize()).unwrap();
    engine.apply(P1, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![aura],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, P0, pacifism).is_some(),
        "an Aura goes to its owner's graveyard"
    );
    engine
        .dev_state_mut(P1)
        .expect("dev board")
        .refresh_characteristics();
    assert!(crate::combat::can_attack(engine.state(), P1, elf));
}
