//! CR 702.19b, 510.1c: an attacker with trample blocked by two or more
//! creatures, its damage divided by its controller under
//! `HouseRules::divide_trample`.
//!
//! Played with creatures built for it, as `banding_tests` is: bodies whose
//! only keyword is the one a test is about, and blockers with no power, so no
//! other sentence moves the damage a test counts.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, preset_both};
use super::*;
use crate::choice::NumberPrompt;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender};
use baylee_core::preset::{GamePreset, HouseRules};

// ---------------------------------------------------------------- fixtures

/// A 5/5 with trample.
const TRAMPLER: u32 = 1250;
/// An 8/8 with trample.
const COLOSSUS: u32 = 1251;
/// A 5/5 with trample and deathtouch.
const TOUCHER: u32 = 1252;
/// A 5/5 with trample and double strike.
const STRIKER: u32 = 1253;
/// A 0/3.
const WALL: u32 = 1254;
/// A 0/6.
const BULWARK: u32 = 1255;
/// A 0/1 with banding.
const HERO: u32 = 1256;

fn fighter(
    index: u32,
    name: &'static str,
    power: i16,
    toughness: i16,
    kw: KeywordSet,
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            keywords: kw,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords: kw,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities: &[],
    }))
}

fn lookup() -> SyntheticLookup {
    SyntheticLookup::new(vec![
        fighter(TRAMPLER, "Trampler", 5, 5, KeywordSet::TRAMPLE),
        fighter(COLOSSUS, "Colossus", 8, 8, KeywordSet::TRAMPLE),
        fighter(
            TOUCHER,
            "Toucher",
            5,
            5,
            KeywordSet::TRAMPLE.union(KeywordSet::DEATHTOUCH),
        ),
        fighter(
            STRIKER,
            "Striker",
            5,
            5,
            KeywordSet::TRAMPLE.union(KeywordSet::DOUBLE_STRIKE),
        ),
        fighter(WALL, "Wall", 0, 3, KeywordSet::EMPTY),
        fighter(BULWARK, "Bulwark", 0, 6, KeywordSet::EMPTY),
        fighter(HERO, "Banding Hero", 0, 1, KeywordSet::BANDING),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// `attacker` on seat 0 and `blockers` on seat 1, at a table with `rule`.
fn table(attacker: u32, blockers: &[u32], rule: bool) -> GamePreset {
    let mut preset = preset_both(19, &[attacker], blockers);
    preset.house_rules.divide_trample = rule;
    preset
}

/// The attack made and every blocker on it, in the order of `preset`, as the
/// priority round of declare blockers begins.
fn blocked(preset: &GamePreset) -> (Engine<SyntheticLookup>, ObjectId, Vec<ObjectId>) {
    let mut engine = Engine::new(preset, lookup()).unwrap();
    keep_mulligans(&mut engine);
    loop {
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question on the way to combat: {other:?}"),
        }
    }
    let on_their_side = |engine: &Engine<SyntheticLookup>, seat: PlayerId| -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.controller == seat)
            })
            .collect()
    };
    let attacker = on_their_side(&engine, ME)[0];
    let blockers = on_their_side(&engine, THEM);
    engine
        .apply(
            ME,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(THEM))],
            },
        )
        .expect("the attack is legal");
    loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers { .. } => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question before blocks: {other:?}"),
        }
    }
    engine
        .apply(
            THEM,
            PlayerAction::DeclareBlockers {
                blockers: blockers.iter().map(|b| (*b, attacker)).collect(),
            },
        )
        .expect("the block is legal");
    (engine, attacker, blockers)
}

/// Passes priority until something other than priority is asked, or the
/// combat is over, and returns that question.
fn next_question(engine: &mut Engine<SyntheticLookup>) -> Pending {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } if engine.state().turn.step != Step::CombatEnd => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => return other,
        }
    }
    panic!("combat never ended");
}

/// The share being asked of seat 0: its recipient, `min`, `max` and `lethal`.
fn asked(engine: &mut Engine<SyntheticLookup>) -> (ObjectId, u32, u32, Option<u32>) {
    let question = next_question(engine);
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason: NumberPrompt::CombatDamage {
            recipient, lethal, ..
        },
    } = question
    else {
        panic!("a share of the damage: {question:?}")
    };
    assert_eq!(player, ME, "its controller divides it");
    (recipient, min, max, lethal)
}

/// Answers the share asked, plays out the combat, and says no further share
/// was asked.
fn last_answer(engine: &mut Engine<SyntheticLookup>, n: u32) {
    engine.apply(ME, PlayerAction::ChooseNumber(n)).unwrap();
    let question = next_question(engine);
    assert!(
        matches!(question, Pending::Priority { .. }),
        "nothing further to divide: {question:?}"
    );
}

fn damage(engine: &Engine<SyntheticLookup>, id: ObjectId) -> u16 {
    engine.state().object(id).map_or(0, |o| o.damage)
}

fn on_battlefield(engine: &Engine<SyntheticLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .contains(&id)
}

fn their_life(engine: &Engine<SyntheticLookup>) -> i32 {
    engine.state().players[1].life
}

// ---------------------------------------------------------- short of lethal

/// Report 01a1afbd-7b64: 5 power cannot kill two 3-toughness creatures, and
/// which one dies is the attacker's controller's to choose (CR 510.1c),
/// the second declared as much as the first. One short of lethal and nothing
/// may go past them ("can't assign any damage to the player", 702.19b), so
/// the last takes all there is and is not asked.
#[test]
fn short_of_lethal_for_both_the_controller_chooses_which_dies() {
    let (mut engine, _, walls) = blocked(&table(TRAMPLER, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(3)));
    last_answer(&mut engine, 1);
    assert_eq!(damage(&engine, walls[0]), 1, "the first is spared");
    assert!(!on_battlefield(&engine, walls[1]), "the second took 4");
    assert_eq!(their_life(&engine), 20, "nothing reached the player");

    let (mut engine, _, walls) = blocked(&table(TRAMPLER, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(3)));
    last_answer(&mut engine, 3);
    assert!(!on_battlefield(&engine, walls[0]), "the first took 3");
    assert_eq!(damage(&engine, walls[1]), 2, "the last takes what is left");
    assert_eq!(their_life(&engine), 20);
}

/// All of it may go to one: the question about the last has no answer left.
#[test]
fn all_of_it_on_the_first_leaves_nothing_to_ask() {
    let (mut engine, _, walls) = blocked(&table(TRAMPLER, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(3)));
    last_answer(&mut engine, 5);
    assert!(!on_battlefield(&engine, walls[0]));
    assert_eq!(damage(&engine, walls[1]), 0);
    assert_eq!(their_life(&engine), 20);
}

// --------------------------------------------------------- enough for them all

/// Lethal to every blocker, and what is left goes where its controller says:
/// past them, or onto a blocker (CR 702.19b: "any excess damage is assigned
/// as its controller chooses among those blocking creatures and the player").
#[test]
fn with_power_to_spare_the_excess_goes_where_its_controller_says() {
    // Past them: 3, 3 and 2 to the player.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 8, Some(3)));
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(
        asked(&mut engine),
        (walls[1], 3, 5, Some(3)),
        "the last is owed lethal before anything goes on"
    );
    assert!(
        engine.apply(ME, PlayerAction::ChooseNumber(2)).is_err(),
        "below lethal, with an excess to send on"
    );
    assert_eq!(asked(&mut engine), (walls[1], 3, 5, Some(3)), "still asked");
    last_answer(&mut engine, 3);
    assert!(!on_battlefield(&engine, walls[0]));
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(their_life(&engine), 18, "the 2 left trampled over");

    // Onto the last: 3 and 5, none to the player.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL], true));
    next_question(&mut engine);
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 3, 5, Some(3)));
    last_answer(&mut engine, 5);
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(their_life(&engine), 20, "it all stayed on the blockers");

    // Onto the first: the 3 it leaves are lethal to the last and all there
    // is, so that is no question.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 8, Some(3)));
    last_answer(&mut engine, 5);
    assert!(!on_battlefield(&engine, walls[0]));
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(their_life(&engine), 20);
}

/// Three blockers: a shortfall on one ends the trample, wherever it falls.
/// The question after it is a plain one, and the last takes the rest.
#[test]
fn three_blockers_and_a_shortfall_in_the_middle() {
    // Short on the second: the third is not asked and takes the 4 left.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 8, Some(3)));
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 0, 5, Some(3)));
    last_answer(&mut engine, 1);
    assert_eq!(damage(&engine, walls[1]), 1);
    assert!(!on_battlefield(&engine, walls[2]));
    assert_eq!(their_life(&engine), 20);

    // Short on the first: the second's question says nothing of lethal.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL, WALL], true));
    next_question(&mut engine);
    engine.apply(ME, PlayerAction::ChooseNumber(2)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 0, 6, None));
    last_answer(&mut engine, 2);
    assert_eq!(damage(&engine, walls[0]), 2);
    assert_eq!(damage(&engine, walls[1]), 2);
    assert!(!on_battlefield(&engine, walls[2]), "the 4 left");
    assert_eq!(their_life(&engine), 20);

    // Nothing left in the middle: neither of the others is asked a share of
    // nothing.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL, WALL], true));
    next_question(&mut engine);
    last_answer(&mut engine, 8);
    assert!(!on_battlefield(&engine, walls[0]));
    assert_eq!(damage(&engine, walls[1]), 0);
    assert_eq!(damage(&engine, walls[2]), 0);
    assert_eq!(their_life(&engine), 20);

    // Lethal to two: the third takes the 2 left, which is all it can take.
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL, WALL], true));
    next_question(&mut engine);
    engine.apply(ME, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 0, 5, Some(3)));
    last_answer(&mut engine, 3);
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(damage(&engine, walls[2]), 2);
    assert_eq!(their_life(&engine), 20);
}

// ---------------------------------------------------------- what lethal is

/// CR 702.2c: from a source with deathtouch any nonzero amount is lethal, so
/// one each is enough and 3 of the 5 trample over.
#[test]
fn deathtouch_owes_one_each_before_the_rest_goes_on() {
    let (mut engine, _, walls) = blocked(&table(TOUCHER, &[WALL, WALL], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(1)));
    engine.apply(ME, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 1, 4, Some(1)));
    last_answer(&mut engine, 1);
    assert!(!on_battlefield(&engine, walls[0]));
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(their_life(&engine), 17);
}

/// CR 702.19b: "take into account damage already marked on the creature".
#[test]
fn damage_already_marked_counts_toward_lethal() {
    let (mut engine, _, walls) = blocked(&table(COLOSSUS, &[WALL, WALL], true));
    engine
        .state
        .object_mut(walls[0])
        .expect("the first blocker")
        .damage = 2;
    assert_eq!(asked(&mut engine), (walls[0], 0, 8, Some(1)));
    engine.apply(ME, PlayerAction::ChooseNumber(1)).unwrap();
    assert_eq!(asked(&mut engine), (walls[1], 3, 7, Some(3)));
    last_answer(&mut engine, 3);
    assert_eq!(their_life(&engine), 16);
}

// ----------------------------------------------------------- double strike

/// CR 510.4: a double striker divides in each step. `collect_combat_damage`
/// spends the first division, and the second question counts what the first
/// step marked.
#[test]
fn a_double_striker_is_asked_in_both_steps() {
    let (mut engine, _, walls) = blocked(&table(STRIKER, &[BULWARK, BULWARK], true));
    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(6)));
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);
    engine.apply(ME, PlayerAction::ChooseNumber(2)).unwrap();

    assert_eq!(asked(&mut engine), (walls[0], 0, 5, Some(4)), "asked again");
    assert_eq!(engine.state().turn.step, Step::CombatDamageFirst);
    assert_eq!(damage(&engine, walls[0]), 2);
    assert_eq!(damage(&engine, walls[1]), 3, "the rest of the first 5");
    last_answer(&mut engine, 4);
    assert!(!on_battlefield(&engine, walls[0]), "2 and 4 are its 6");
    assert_eq!(damage(&engine, walls[1]), 4, "3 and the 1 left");
    assert_eq!(their_life(&engine), 20);
}

// ------------------------------------------------------------- what stays

/// CR 702.22j: banding hands the division to the defending player, among the
/// blockers only. The rule changes nothing about it.
#[test]
fn banding_keeps_its_division_under_the_rule() {
    let (mut engine, _, walls) = blocked(&table(TRAMPLER, &[HERO, WALL], true));
    let question = next_question(&mut engine);
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason: NumberPrompt::CombatDamage { lethal, .. },
    } = question
    else {
        panic!("banding's division: {question:?}")
    };
    assert_eq!((player, min, max, lethal), (THEM, 0, 5, None));
    engine.apply(THEM, PlayerAction::ChooseNumber(0)).unwrap();
    next_question(&mut engine);
    assert!(on_battlefield(&engine, walls[0]), "the hero was spared");
    assert!(!on_battlefield(&engine, walls[1]), "the wall took all 5");
    assert_eq!(their_life(&engine), 20);
}

/// One blocker is still the engine's: lethal to it and the rest past it.
#[test]
fn one_blocker_is_not_asked() {
    let (mut engine, _, walls) = blocked(&table(TRAMPLER, &[WALL], true));
    assert!(matches!(
        next_question(&mut engine),
        Pending::Priority { .. }
    ));
    assert!(!on_battlefield(&engine, walls[0]));
    assert_eq!(their_life(&engine), 18);
}

/// The rule off is the game as it was: nobody asked, lethal to each blocker
/// in declaration order, the rest past them; and a preset read from before
/// the field holds the rule off and plays to the same hash. Only the rule
/// on moves it, and that before anything happens on the board.
#[test]
fn without_the_rule_nothing_is_asked_and_nothing_moves() {
    let plain = table(COLOSSUS, &[WALL, WALL], false);
    let (mut engine, _, walls) = blocked(&plain);
    assert!(matches!(
        next_question(&mut engine),
        Pending::Priority { .. }
    ));
    assert!(!on_battlefield(&engine, walls[0]));
    assert!(!on_battlefield(&engine, walls[1]));
    assert_eq!(their_life(&engine), 18, "3, 3 and the 2 left");

    let written = serde_json::to_value(&plain.house_rules).unwrap();
    assert!(written.get("divide_trample").is_none(), "{written}");
    let read: HouseRules = serde_json::from_value(written).unwrap();
    assert!(!read.divide_trample);
    let mut old = plain.clone();
    old.house_rules = read;
    let (mut again, _, _) = blocked(&old);
    next_question(&mut again);
    assert_eq!(again.snapshot_hash(), engine.snapshot_hash());

    let (on, _, _) = blocked(&table(COLOSSUS, &[WALL, WALL], true));
    let (off, _, _) = blocked(&plain);
    assert_eq!(
        on.state().snapshot_hash(),
        off.state().snapshot_hash(),
        "the board is identical; only the question to come differs"
    );
    assert_ne!(on.snapshot_hash(), off.snapshot_hash());
}

/// What stands on the wire: the question's `lethal` is absent where it says
/// nothing, so a question that was asked before the field reads and writes
/// as it did, and a reader of the old shape meets only an extra field.
#[test]
fn a_question_without_lethal_is_written_as_it_was() {
    let share = |lethal| NumberPrompt::CombatDamage {
        source: ObjectId::new(1, 0),
        recipient: ObjectId::new(2, 0),
        index: 0,
        of: 2,
        left: 5,
        lethal,
    };
    let plain = serde_json::to_string(&share(None)).unwrap();
    assert!(!plain.contains("lethal"), "{plain}");
    let back: NumberPrompt = serde_json::from_str(&plain).unwrap();
    assert_eq!(back, share(None));
    let told = serde_json::to_string(&share(Some(3))).unwrap();
    assert!(told.contains("\"lethal\":3"), "{told}");
    let back: NumberPrompt = serde_json::from_str(&told).unwrap();
    assert_eq!(back, share(Some(3)));
}
