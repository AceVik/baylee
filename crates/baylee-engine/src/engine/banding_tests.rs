//! Banding (CR 702.22): the band declared with the attack, a block on one
//! member blocking the band, and the damage divisions banding hands to the
//! other player.
//!
//! Played with creatures built for it: a 1/1 with banding and nothing else,
//! and vanilla bodies whose only keyword is the one a test is about, so no
//! other sentence moves the damage a test counts.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both};
use super::*;
use baylee_cards_dsl::{CardDef, CommanderRule, Coverage, FaceDef, KeywordSet};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 1/1 with banding.
const HERO: u32 = 1120;
/// A vanilla 3/3.
const BRUTE: u32 = 1121;
/// A 2/2 with flying.
const FLIER: u32 = 1122;
/// A 5/5 with trample.
const TRAMPLER: u32 = 1123;
/// A vanilla 2/2.
const BEAR: u32 = 1124;

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
        fighter(HERO, "Banding Hero", 1, 1, KeywordSet::BANDING),
        fighter(BRUTE, "Brute", 3, 3, KeywordSet::EMPTY),
        fighter(FLIER, "Flier", 2, 2, KeywordSet::FLYING),
        fighter(TRAMPLER, "Trampler", 5, 5, KeywordSet::TRAMPLE),
        fighter(BEAR, "Bear", 2, 2, KeywordSet::EMPTY),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// Seat 0 with `mine`, seat 1 with `theirs`, both hands kept, and seat 0
/// asked to declare attackers on its first turn.
fn combat(mine: &[u32], theirs: &[u32]) -> Engine<SyntheticLookup> {
    let mut engine = Engine::new(&preset_both(31, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    for _ in 0..40 {
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME => return engine,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question on the way to combat: {other:?}"),
        }
    }
    panic!("seat 0 was never asked to attack");
}

/// Seat `seat`'s one permanent from `index`.
fn one(engine: &Engine<SyntheticLookup>, index: u32, seat: PlayerId) -> ObjectId {
    permanents(engine, index)
        .into_iter()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat)
        })
        .expect("the permanent is on the battlefield")
}

fn attack(engine: &mut Engine<SyntheticLookup>, attackers: &[ObjectId]) {
    engine
        .apply(
            ME,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(THEM)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
}

/// Passes priority until seat 1 is asked for blockers.
fn to_blockers(engine: &mut Engine<SyntheticLookup>) {
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { .. } => return,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question before blocks: {other:?}"),
        }
    }
    panic!("seat 1 was never asked to block");
}

fn block(engine: &mut Engine<SyntheticLookup>, blocks: &[(ObjectId, ObjectId)]) {
    engine
        .apply(
            THEM,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the block is legal");
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

// ------------------------------------------------------------------- bands

/// CR 508.1e, 702.22c: the attack asks the creature with banding which
/// attackers band with it, and a band holds any number with banding and at
/// most one without. Naming two without banding is refused and the question
/// stands; naming one forms the band, and the one left out is in none.
#[test]
fn a_band_holds_at_most_one_creature_without_banding() {
    let mut engine = combat(&[HERO, BRUTE, BEAR], &[]);
    let (hero, brute, bear) = (
        one(&engine, HERO, ME),
        one(&engine, BRUTE, ME),
        one(&engine, BEAR, ME),
    );
    attack(&mut engine, &[hero, brute, bear]);

    let Pending::ChooseCards {
        player,
        options,
        min,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the band question, got {:?}", engine.pending());
    };
    assert_eq!(player, ME, "the attacking player announces bands");
    assert_eq!(prompt, crate::choice::ChoicePrompt::Band { with: hero });
    assert_eq!(min, 0, "attacking without a band is an answer");
    assert_eq!(
        options,
        vec![brute, bear],
        "every other attacker on the same player"
    );

    let refused = engine.apply(
        ME,
        PlayerAction::ChooseObjects {
            objects: vec![brute, bear],
        },
    );
    assert!(
        refused.is_err(),
        "two creatures without banding in one band"
    );
    assert!(
        matches!(engine.pending(), Pending::ChooseCards { .. }),
        "the question stands after a refusal"
    );

    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![brute],
            },
        )
        .expect("one creature without banding joins");
    let c = &engine.state().combat;
    assert!(c.band_of(hero).is_some(), "the hero is in a band");
    assert_eq!(c.band_of(hero), c.band_of(brute), "with the brute");
    assert_eq!(c.band_of(bear), None, "the bear was named by nobody");
    assert_eq!(c.band_mates(hero), vec![brute]);
}

/// With no creature with banding attacking, nothing is asked: the attack
/// goes straight on to priority. The counter-board for the test above.
#[test]
fn an_attack_without_banding_asks_for_no_band() {
    let mut engine = combat(&[BRUTE, BEAR], &[]);
    let (brute, bear) = (one(&engine, BRUTE, ME), one(&engine, BEAR, ME));
    attack(&mut engine, &[brute, bear]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "expected priority, got {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().combat.band_of(brute), None);
}

// ------------------------------------------------------------------ blocks

/// CR 702.22h and its example: a band of a flier and a creature without
/// flying is blocked by a creature that could block only the second, and
/// the flier becomes blocked by it too — journalled as a block of its own,
/// so "becomes blocked by a creature" sees it. Unbanded, the same block
/// leaves the flier unblocked.
#[test]
fn blocking_one_member_of_a_band_blocks_the_band() {
    for banded in [false, true] {
        let mut engine = combat(&[HERO, FLIER], &[BEAR]);
        let (hero, flier, bear) = (
            one(&engine, HERO, ME),
            one(&engine, FLIER, ME),
            one(&engine, BEAR, THEM),
        );
        attack(&mut engine, &[hero, flier]);
        let band = if banded { vec![flier] } else { vec![] };
        engine
            .apply(ME, PlayerAction::ChooseObjects { objects: band })
            .unwrap();
        to_blockers(&mut engine);
        let from = engine.state().journal.len();
        block(&mut engine, &[(bear, hero)]);

        let c = &engine.state().combat;
        assert!(
            c.is_blocked(hero),
            "the hero was blocked by the declaration"
        );
        assert_eq!(
            c.is_blocked(flier),
            banded,
            "the flier is blocked only in a band"
        );
        assert_eq!(c.blockers_of(flier).contains(&bear), banded);
        let journalled = engine.state().journal.entries()[from..].iter().any(|e| {
            matches!(
                e.event,
                GameEvent::BecameBlocker { object, attacker } if object == bear && attacker == flier
            )
        });
        assert_eq!(
            journalled, banded,
            "the band's block is journalled as a block"
        );
    }
}

// ------------------------------------------------------------------ damage

/// CR 702.22k and 510.1d: a creature blocking a band blocks each member,
/// and deals its combat damage once, divided by the *active* player. Here
/// the active player puts all 3 on the hero and none on the brute: the hero
/// dies and the brute is unmarked — 3 dealt in all, the blocker's power, and
/// not 3 to each creature it blocks.
#[test]
fn a_creature_blocking_a_band_deals_its_damage_once_as_the_attacker_divides_it() {
    let mut engine = combat(&[HERO, BRUTE], &[BRUTE]);
    let (hero, brute) = (one(&engine, HERO, ME), one(&engine, BRUTE, ME));
    let blocker = one(&engine, BRUTE, THEM);
    attack(&mut engine, &[hero, brute]);
    engine
        .apply(
            ME,
            PlayerAction::ChooseObjects {
                objects: vec![brute],
            },
        )
        .unwrap();
    to_blockers(&mut engine);
    block(&mut engine, &[(blocker, brute)]);

    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason,
    } = next_question(&mut engine)
    else {
        panic!("expected the division, got {:?}", engine.pending());
    };
    assert_eq!(player, ME, "the active player divides the blocker's damage");
    assert_eq!((min, max), (0, 3), "the blocker's power, any of it");
    assert_eq!(
        reason,
        crate::choice::NumberPrompt::CombatDamage {
            source: blocker,
            recipient: brute,
            index: 0,
            of: 2,
            left: 3,
        },
        "the declared block first, the band's after it"
    );
    engine.apply(ME, PlayerAction::ChooseNumber(0)).unwrap();

    assert!(on_battlefield(&engine, brute), "the brute survived");
    assert_eq!(
        damage(&engine, brute),
        0,
        "nothing was assigned to the brute"
    );
    assert!(
        !on_battlefield(&engine, hero),
        "the hero took the rest and died"
    );
    assert!(
        !on_battlefield(&engine, blocker),
        "the band's own damage went to the blocker"
    );
}

/// CR 702.22j: a creature with trample blocked by a creature with banding
/// has its damage divided by the defending player among the creatures
/// blocking it, and nothing reaches the player. Blocked by a creature
/// without banding, the same attack tramples 3 over the bear.
#[test]
fn a_trampler_blocked_by_a_creature_with_banding_deals_all_of_it_to_the_blocker() {
    for (blocker_index, lost) in [(BEAR, 3), (HERO, 0)] {
        let mut engine = combat(&[TRAMPLER], &[blocker_index]);
        let trampler = one(&engine, TRAMPLER, ME);
        let blocker = one(&engine, blocker_index, THEM);
        attack(&mut engine, &[trampler]);
        to_blockers(&mut engine);
        block(&mut engine, &[(blocker, trampler)]);
        next_question(&mut engine);
        assert_eq!(
            engine.state().players[1].life,
            20 - lost,
            "blocked by {blocker_index}, the player loses {lost}"
        );
        assert!(
            !on_battlefield(&engine, blocker),
            "the blocker died either way"
        );
    }
}

/// CR 702.22j: an attacker blocked by a creature with banding and another
/// has its damage divided by the *defending* player. Seat 1 puts none of
/// the brute's 3 on the hero and all of it on the bear; left to the
/// attacker, the first blocker would have taken it.
#[test]
fn an_attacker_blocked_by_a_creature_with_banding_is_divided_by_the_defending_player() {
    let mut engine = combat(&[BRUTE], &[HERO, BEAR]);
    let brute = one(&engine, BRUTE, ME);
    let (hero, bear) = (one(&engine, HERO, THEM), one(&engine, BEAR, THEM));
    attack(&mut engine, &[brute]);
    to_blockers(&mut engine);
    block(&mut engine, &[(hero, brute), (bear, brute)]);

    let Pending::ChooseNumber { player, reason, .. } = next_question(&mut engine) else {
        panic!("expected the division, got {:?}", engine.pending());
    };
    assert_eq!(
        player, THEM,
        "the defending player divides the attacker's damage"
    );
    assert!(matches!(
        reason,
        crate::choice::NumberPrompt::CombatDamage { source, recipient, .. }
            if source == brute && recipient == hero
    ));
    engine.apply(THEM, PlayerAction::ChooseNumber(0)).unwrap();

    assert!(
        on_battlefield(&engine, hero),
        "the hero was given none of it"
    );
    assert!(!on_battlefield(&engine, bear), "the bear took all 3");
}

/// The same block without banding is divided too, by the attacker's
/// controller rather than the defending player.
#[test]
fn an_attacker_blocked_by_two_is_divided_by_its_controller() {
    // CR 510.1c: "If two or more creatures are blocking it, it assigns its
    // combat damage to those creatures divided as its controller chooses
    // among them." Without banding the engine once put all of it on the
    // first-declared blocker and asked nobody.
    let mut engine = combat(&[BRUTE], &[BEAR, BEAR]);
    let brute = one(&engine, BRUTE, ME);
    let bears: Vec<ObjectId> = permanents(&engine, BEAR);
    attack(&mut engine, &[brute]);
    to_blockers(&mut engine);
    block(&mut engine, &[(bears[0], brute), (bears[1], brute)]);
    let question = next_question(&mut engine);
    let Pending::ChooseNumber {
        player,
        min,
        max,
        reason:
            crate::choice::NumberPrompt::CombatDamage {
                source,
                recipient,
                index,
                of,
                ..
            },
    } = question
    else {
        panic!("the attacker's controller divides: {question:?}")
    };
    assert_eq!(
        (player, source, recipient, index, of, min, max),
        (ME, brute, bears[0], 0, 2, 0, 3)
    );
    // One to the first, and the last takes the rest.
    engine.apply(ME, PlayerAction::ChooseNumber(1)).unwrap();
    next_question(&mut engine);
    assert!(on_battlefield(&engine, bears[0]));
    assert_eq!(damage(&engine, bears[0]), 1);
    assert!(!on_battlefield(&engine, bears[1]), "two to the second");
}
