//! Auras and Equipment with real cards: Holy Strength ({W}, enchanted
//! creature gets +1/+2), Pacifism ({1}{W}, enchanted creature can't attack
//! or block) and Leonin Scimitar (equipped creature gets +1/+1, equip {1}).
//! An Aura spell targets what it will enchant (CR 303.4a) and is put into
//! the graveyard once that is gone (CR 704.5m); Equipment stays when its
//! creature leaves (CR 704.5n) and equips only at sorcery speed (CR 702.6a).

use super::testkit::{
    Duel, basic_forest, card_index, in_graveyard, in_hand, keep_mulligans, on_battlefield,
    pass_until, pt, quiet_creature, stack_is_empty, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const SEED: u64 = 0xa77a_0001;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

type Game = Engine<super::testkit::RegistryLookup>;

fn holy_strength() -> CardIndex {
    card_index("9357de36-f8be-4f49-b2c8-9fe9eaf82b07")
}

fn pacifism() -> CardIndex {
    card_index("5f5e0b10-c8cf-450c-bfd3-bcb0528ec330")
}

fn scimitar() -> CardIndex {
    card_index("cde26d69-f3e7-4dd0-a53b-cd0ec812d717")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn duel(hand: &[CardIndex], mine: &[CardIndex]) -> Game {
    duel_against(hand, mine, &[quiet_creature()])
}

fn duel_against(hand: &[CardIndex], mine: &[CardIndex], theirs: &[CardIndex]) -> Game {
    let mut engine = Duel::new(SEED, basic_forest())
        .hand(0, hand)
        .battlefield(0, mine)
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

fn enchant(engine: &mut Game, aura: CardIndex, target: ObjectId) {
    tap_all_mana(engine, P0);
    let card = in_hand(engine, P0, aura).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

fn body(engine: &mut Game, id: ObjectId) -> (i16, i16) {
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    pt(engine, id)
}

fn kill(engine: &mut Game, id: ObjectId) {
    let state = engine.dev_state_mut(P0).expect("dev board");
    crate::sba::destroy(state, id);
}

#[test]
fn an_aura_resolves_attached_to_its_target_and_changes_it() {
    let mut engine = duel(&[holy_strength()], &[plains(), quiet_creature()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    enchant(&mut engine, holy_strength(), elf);
    let aura = on_battlefield(&engine, P0, holy_strength()).expect("the aura");
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(elf));
    assert_eq!(body(&mut engine, elf), (2, 3));
}

#[test]
fn an_aura_goes_to_the_graveyard_after_its_creature() {
    let mut engine = duel(&[holy_strength()], &[plains(), quiet_creature()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    enchant(&mut engine, holy_strength(), elf);
    kill(&mut engine, elf);
    pass_until(&mut engine, |e| {
        on_battlefield(e, P0, holy_strength()).is_none()
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(in_graveyard(&engine, P0, holy_strength()).is_some());
}

#[test]
fn an_aura_may_enchant_an_opponents_creature() {
    let mut engine = duel(&[pacifism()], &[plains(), plains()]);
    let theirs = on_battlefield(&engine, P1, quiet_creature()).unwrap();
    enchant(&mut engine, pacifism(), theirs);
    let aura = on_battlefield(&engine, P0, pacifism()).expect("controlled by its caster");
    assert_eq!(
        engine.state().object(aura).unwrap().attached_to,
        Some(theirs)
    );
}

#[test]
fn equipping_attaches_the_equipment_and_its_bonus() {
    let mut engine = duel(&[], &[plains(), quiet_creature(), scimitar()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let blade = on_battlefield(&engine, P0, scimitar()).unwrap();
    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: blade,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseTargets { .. }) {
        engine
            .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(blade).unwrap().attached_to, Some(elf));
    assert_eq!(body(&mut engine, elf), (2, 2));
}

/// An Equipment attached to something that is no longer a creature becomes
/// unattached and stays on the battlefield (CR 704.5n). The creature is
/// turned into a plain enchantment rather than killed, so only that
/// state-based action can have moved the Equipment.
#[test]
fn equipment_stays_on_the_battlefield_when_its_creature_stops_being_one() {
    let mut engine = duel(&[], &[plains(), quiet_creature(), scimitar()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let blade = on_battlefield(&engine, P0, scimitar()).unwrap();
    assert!(equip(&mut engine, blade, elf));
    assert_eq!(engine.state().object(blade).unwrap().attached_to, Some(elf));
    {
        let state = engine.dev_state_mut(P0).expect("dev board");
        state.object_mut(elf).unwrap().base_mut().types = baylee_core::types::TypeSet::ENCHANTMENT;
        state.invalidate_projections();
        state.refresh_characteristics();
    }
    assert_eq!(
        engine.state().object(blade).unwrap().attached_to,
        Some(elf),
        "nothing has checked state-based actions yet"
    );
    pass_until(&mut engine, |e| {
        e.state().object(blade).unwrap().attached_to.is_none()
    });
    assert!(on_battlefield(&engine, P0, scimitar()).is_some());
}

#[test]
fn equip_is_refused_in_the_opponents_turn() {
    let mut engine = duel(&[], &[plains(), quiet_creature(), scimitar()]);
    let blade = on_battlefield(&engine, P0, scimitar()).unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: blade,
                    ability_index: 0,
                },
            )
            .is_err()
    );
}

/// An Aura spell whose target is gone as it resolves does not resolve
/// and goes to the graveyard, never onto the battlefield (CR 608.2b,
/// 303.4a).
#[test]
fn an_aura_spell_whose_target_left_goes_to_the_graveyard() {
    let mut engine = duel(&[holy_strength()], &[plains(), quiet_creature()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    tap_all_mana(&mut engine, P0);
    let card = in_hand(&engine, P0, holy_strength()).unwrap();
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .move_object(
            elf,
            crate::zone::ZoneLocation::Hand(P0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, holy_strength()).is_none());
    assert!(in_graveyard(&engine, P0, holy_strength()).is_some());
}

/// The Aura's bonus is gone the moment it is: a Holy Strength that is
/// destroyed leaves its creature a 1/1 again.
#[test]
fn the_bonus_leaves_with_the_aura() {
    let mut engine = duel(&[holy_strength()], &[plains(), quiet_creature()]);
    let elf = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    enchant(&mut engine, holy_strength(), elf);
    let aura = on_battlefield(&engine, P0, holy_strength()).unwrap();
    kill(&mut engine, aura);
    assert_eq!(body(&mut engine, elf), (1, 1));
}

fn equip(engine: &mut Game, blade: ObjectId, onto: ObjectId) -> bool {
    tap_all_mana(engine, P0);
    if engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: blade,
                ability_index: 0,
            },
        )
        .is_err()
    {
        return false;
    }
    if matches!(engine.pending(), Pending::ChooseTargets { .. })
        && engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![onto],
                },
            )
            .is_err()
    {
        return false;
    }
    pass_until(engine, stack_is_empty);
    true
}

/// Every creature `seat` controls that is a `quiet_creature`, in
/// battlefield order.
fn creatures_of(engine: &Game, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.card.is_some_and(|c| c.index == quiet_creature())
            })
        })
        .collect()
}

/// Equipping a second creature moves the Equipment, and the first one
/// loses the bonus (CR 701.3a: attaching takes it from where it is). The
/// second equip is paid in a later turn, from freshly untapped lands.
#[test]
fn re_equipping_moves_the_bonus() {
    let mut engine = duel(
        &[],
        &[
            plains(),
            plains(),
            quiet_creature(),
            quiet_creature(),
            scimitar(),
        ],
    );
    let elves = creatures_of(&engine, P0);
    assert_eq!(elves.len(), 2);
    let blade = on_battlefield(&engine, P0, scimitar()).unwrap();
    assert!(equip(&mut engine, blade, elves[0]));
    assert_eq!(body(&mut engine, elves[0]), (2, 2));
    pass_until(&mut engine, |e| e.state().turn.active == P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(equip(&mut engine, blade, elves[1]));
    assert_eq!(
        engine.state().object(blade).unwrap().attached_to,
        Some(elves[1])
    );
    assert_eq!(body(&mut engine, elves[0]), (1, 1));
    assert_eq!(body(&mut engine, elves[1]), (2, 2));
}

/// Equip targets "a creature you control" only (CR 702.6a): with a creature
/// of its own on the board, seat 0 is not offered the opponent's.
#[test]
fn equip_cannot_target_an_opponents_creature() {
    let mut engine = duel(&[], &[plains(), quiet_creature(), scimitar()]);
    let blade = on_battlefield(&engine, P0, scimitar()).unwrap();
    let mine = creatures_of(&engine, P0)[0];
    let theirs = creatures_of(&engine, P1)[0];
    tap_all_mana(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: blade,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target question, got {:?}", engine.pending());
    };
    assert!(options.contains(&mine));
    assert!(!options.contains(&theirs));
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![theirs],
                },
            )
            .is_err()
    );
}

/// Pacifism on one of two identical creatures stops that one from
/// attacking (the Aura's text): the other, with
/// nothing on it, still can.
#[test]
fn a_pacified_creature_cannot_attack() {
    let mut engine = duel_against(
        &[pacifism()],
        &[plains(), plains()],
        &[quiet_creature(), quiet_creature()],
    );
    let theirs = creatures_of(&engine, P1);
    enchant(&mut engine, pacifism(), theirs[0]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::FirstMain
    });
    engine
        .dev_state_mut(P0)
        .expect("dev board")
        .refresh_characteristics();
    assert!(crate::combat::can_attack(engine.state(), P1, theirs[1]));
    assert!(!crate::combat::can_attack(engine.state(), P1, theirs[0]));
}

/// Pacifism stops blocking as well as attacking: in a real declare-blockers
/// step the enchanted creature's block is refused and its twin's is taken.
#[test]
fn a_pacified_creature_cannot_block() {
    let mut engine = duel_against(
        &[pacifism()],
        &[plains(), plains(), quiet_creature()],
        &[quiet_creature(), quiet_creature()],
    );
    let theirs = creatures_of(&engine, P1);
    let mine = creatures_of(&engine, P0)[0];
    // Keep the attacker untapped: only the lands pay for the Aura.
    super::testkit::tap_mana_except(&mut engine, P0, mine);
    let card = in_hand(&engine, P0, pacifism()).expect("in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mine, baylee_core::ids::Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(crate::combat::can_block(
        engine.state(),
        P1,
        theirs[1],
        mine
    ));
    assert!(!crate::combat::can_block(
        engine.state(),
        P1,
        theirs[0],
        mine
    ));
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(theirs[0], mine)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(theirs[1], mine)],
            },
        )
        .unwrap();
}
