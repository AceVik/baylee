//! What effects add to the declaration of blockers: how many attackers a
//! creature may block (CR 509.1a, "can block an additional creature each
//! combat", "can block any number of creatures"), and the requirements
//! (CR 509.1c, "all creatures able to block this creature do so", "it
//! blocks each attacking creature if able"), obeyed as far as the most any
//! declaration could obey without breaking a restriction.
//!
//! Played with creatures built for it, each with one sentence, beside
//! vanilla bodies. Every scenario ends by handing the question's own
//! `obeying` declaration back to the engine, which is the property the
//! whole design rests on: the engine never asks for more than a
//! declaration it can name.

use super::synthetic::{SyntheticLookup, creature_face, keep_mulligans, permanents, preset_both};
use super::*;
use crate::choice::{BlockCapacity, BlockOption};
use baylee_cards_dsl::{
    AbilityDef, CardDef, CommanderRule, Coverage, FaceDef, Filter, KeywordSet, Modifier,
    static_ability,
};
use baylee_core::color::ColorSet;
use baylee_core::ids::{CardIndex, Defender};

// ---------------------------------------------------------------- fixtures

/// A 2/2: "All creatures able to block this creature do so."
const LURING: u32 = 1160;
/// A 2/2 with menace and the same sentence.
const MENACING_LURE: u32 = 1161;
/// A 2/2 with menace.
const MENACING: u32 = 1162;
/// A vanilla 2/2.
const BEAR: u32 = 1163;
/// A 1/1 with flying and the lure's sentence.
const LURING_FLIER: u32 = 1164;
/// A 4/4: "can block an additional creature each combat".
const GIANT: u32 = 1165;
/// A 1/4 with Blaze of Glory's two sentences as its own: "can block any
/// number of creatures; blocks each attacking creature if able".
const BLAZED: u32 = 1166;
/// A 1/4 that blocks each attacking creature if able and may block one.
const DUTIFUL: u32 = 1167;
/// A vanilla 1/1 with flying.
const FLIER: u32 = 1168;

static LURE: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::MustBeBlockedByAllAble
)];
static ONE_MORE: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::CanBlockAdditional(1)
)];
static BLAZE: &[AbilityDef] = &[
    static_ability!(Filter::This, Modifier::CanBlockAnyNumber),
    static_ability!(Filter::This, Modifier::BlocksEachAttackerIfAble),
];
static DUTY: &[AbilityDef] = &[static_ability!(
    Filter::This,
    Modifier::BlocksEachAttackerIfAble
)];

fn fighter(
    index: u32,
    name: &'static str,
    (power, toughness): (i16, i16),
    keywords: KeywordSet,
    abilities: &'static [AbilityDef],
) -> &'static CardDef {
    Box::leak(Box::new(CardDef {
        index: CardIndex::new(index),
        oracle_id: "test",
        scryfall_id: "test",
        faces: Box::leak(Box::new([FaceDef {
            abilities,
            keywords,
            ..creature_face(name, power, toughness, &[])
        }])),
        color_identity: ColorSet::EMPTY,
        keywords,
        commander: CommanderRule::NotEligible,
        partner: baylee_cards_dsl::PartnerKind::None,
        coverage: Coverage::Implemented,
        abilities,
    }))
}

fn lookup() -> SyntheticLookup {
    let none = KeywordSet::EMPTY;
    SyntheticLookup::new(vec![
        fighter(LURING, "Luring", (2, 2), none, LURE),
        fighter(
            MENACING_LURE,
            "Menacing Lure",
            (2, 2),
            KeywordSet::MENACE,
            LURE,
        ),
        fighter(MENACING, "Menacing", (2, 2), KeywordSet::MENACE, &[]),
        fighter(BEAR, "Bear", (2, 2), none, &[]),
        fighter(
            LURING_FLIER,
            "Luring Flier",
            (1, 1),
            KeywordSet::FLYING,
            LURE,
        ),
        fighter(GIANT, "Giant", (4, 4), none, ONE_MORE),
        fighter(BLAZED, "Blazed", (1, 4), none, BLAZE),
        fighter(DUTIFUL, "Dutiful", (1, 4), none, DUTY),
        fighter(FLIER, "Flier", (1, 1), KeywordSet::FLYING, &[]),
    ])
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

/// The block question, as the engine put it to `THEM`.
struct Question {
    options: Vec<BlockOption>,
    capacity: Vec<BlockCapacity>,
    obeying: Vec<(ObjectId, ObjectId)>,
}

/// Seat 0 with `mine`, seat 1 with `theirs`; `prepare` runs in seat 0's
/// first main phase, and then every one of `attack` (indices into seat 0's
/// board, in that order, one permanent per entry) attacks seat 1, and
/// seat 1 is asked to block.
fn blocks(
    mine: &[u32],
    theirs: &[u32],
    attack: &[ObjectId],
    prepare: impl FnOnce(&mut Engine<SyntheticLookup>),
) -> (Engine<SyntheticLookup>, Question) {
    let mut engine = Engine::new(&preset_both(53, mine, theirs), lookup()).unwrap();
    keep_mulligans(&mut engine);
    let mut prepare = Some(prepare);
    for _ in 0..60 {
        if engine.state().turn.step == crate::turn::Step::Main
            && let Some(prepare) = prepare.take()
        {
            prepare(&mut engine);
        }
        match engine.pending().clone() {
            Pending::ChooseAttackers { player, .. } if player == ME => {
                engine
                    .apply(
                        ME,
                        PlayerAction::DeclareAttackers {
                            attackers: attack
                                .iter()
                                .map(|a| (*a, Defender::Player(THEM)))
                                .collect(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseBlockers {
                player,
                blockers,
                capacity,
                obeying,
                ..
            } => {
                assert_eq!(player, THEM);
                return (
                    engine,
                    Question {
                        options: blockers,
                        capacity,
                        obeying,
                    },
                );
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected question on the way to blocks: {other:?}"),
        }
    }
    panic!("seat 1 was never asked to block");
}

/// The ids of a board's permanents, in the order the preset laid them out.
/// Asked before the game moves, so the order is the preset's.
fn ids(mine: &[u32], theirs: &[u32]) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let engine = Engine::new(&preset_both(53, mine, theirs), lookup()).unwrap();
    let side = |seat: PlayerId, cards: &[u32]| -> Vec<ObjectId> {
        let mut out: Vec<ObjectId> = Vec::new();
        for card in cards {
            let next = permanents(&engine, *card)
                .into_iter()
                .find(|id| {
                    !out.contains(id) && engine.state().object(*id).unwrap().controller == seat
                })
                .expect("the preset laid it out");
            out.push(next);
        }
        out
    };
    (side(ME, mine), side(THEM, theirs))
}

fn declare(
    engine: &mut Engine<SyntheticLookup>,
    blockers: &[(ObjectId, ObjectId)],
) -> Result<(), EngineError> {
    engine.apply(
        THEM,
        PlayerAction::DeclareBlockers {
            blockers: blockers.to_vec(),
        },
    )
}

/// The property the design rests on: every pair of the question's
/// `obeying` is one the question offers, and the declaration, handed back,
/// is accepted — so is the clock's answer, which is the same declaration.
fn obeying_is_accepted(mut engine: Engine<SyntheticLookup>, question: &Question) {
    for (blocker, attacker) in &question.obeying {
        assert!(
            question
                .options
                .iter()
                .any(|o| o.blocker == *blocker && o.attackers.contains(attacker)),
            "({blocker:?}, {attacker:?}) is not offered: {:?}",
            question.options
        );
    }
    let Some(PlayerAction::DeclareBlockers { blockers }) =
        crate::choice::timeout_answer(engine.pending())
    else {
        panic!("the clock answers a block question with blocks");
    };
    assert_eq!(
        blockers, question.obeying,
        "the clock's answer is `obeying`"
    );
    declare(&mut engine, &question.obeying).expect("the question's own declaration is legal");
}

fn refused(result: Result<(), EngineError>, why: &str) {
    match result {
        Err(EngineError::IllegalAction(message)) => assert_eq!(message, why),
        other => panic!("expected the refusal {why:?}, got {other:?}"),
    }
}

const MUST_BLOCK: &str = "a creature that must block if able does not";

// ----------------------------------------------------------------- lures

/// "All creatures able to block this creature do so": each creature that
/// may block it is named, and a declaration leaving either out is refused.
#[test]
fn every_creature_able_to_block_a_lure_must_block_it() {
    let mine = [LURING];
    let theirs = [BEAR, BEAR];
    let (me, them) = ids(&mine, &theirs);
    let (mut engine, question) = blocks(&mine, &theirs, &[me[0]], |_| {});
    assert_eq!(question.obeying, vec![(them[0], me[0]), (them[1], me[0])]);
    assert!(question.capacity.is_empty(), "each bear blocks one");
    refused(declare(&mut engine, &[]), MUST_BLOCK);
    refused(declare(&mut engine, &[(them[0], me[0])]), MUST_BLOCK);
    obeying_is_accepted(engine, &question);
}

/// A creature that may not block the lure is under no requirement: a
/// tapped one, and a ground creature facing a flier. The creature that
/// can still must.
#[test]
fn a_creature_that_cannot_block_the_lure_is_not_asked_to() {
    let mine = [LURING_FLIER];
    let theirs = [BEAR, FLIER, FLIER];
    let (me, them) = ids(&mine, &theirs);
    let tapped = them[2];
    let (mut engine, question) = blocks(&mine, &theirs, &[me[0]], |engine| {
        engine
            .state
            .object_mut(tapped)
            .unwrap()
            .status
            .insert(crate::object::Status::TAPPED);
        engine.state.board_state_changed();
    });
    assert_eq!(question.obeying, vec![(them[1], me[0])]);
    refused(declare(&mut engine, &[]), MUST_BLOCK);
    obeying_is_accepted(engine, &question);
}

/// A lure with menace facing one creature that may block it: no legal
/// declaration blocks it (CR 702.111b), so the most requirements that can
/// be obeyed is none, and not blocking at all is legal. Blocking it with
/// the one creature is still refused.
#[test]
fn a_menacing_lure_with_one_blocker_asks_nothing() {
    let mine = [MENACING_LURE];
    let theirs = [BEAR];
    let (me, them) = ids(&mine, &theirs);
    let (mut engine, question) = blocks(&mine, &theirs, &[me[0]], |_| {});
    assert!(question.obeying.is_empty());
    assert!(declare(&mut engine, &[(them[0], me[0])]).is_err());
    obeying_is_accepted(engine, &question);
}

/// With two creatures able to block it, both must.
#[test]
fn a_menacing_lure_with_two_blockers_takes_both() {
    let mine = [MENACING_LURE];
    let theirs = [BEAR, BEAR];
    let (me, them) = ids(&mine, &theirs);
    let (mut engine, question) = blocks(&mine, &theirs, &[me[0]], |_| {});
    assert_eq!(question.obeying, vec![(them[0], me[0]), (them[1], me[0])]);
    refused(declare(&mut engine, &[]), MUST_BLOCK);
    obeying_is_accepted(engine, &question);
}

/// Two lures and one creature that may block one attacker: it cannot
/// obey both, and blocking either obeys as many as can be obeyed, so both
/// declarations are legal and not blocking is not.
#[test]
fn one_blocker_between_two_lures_blocks_either() {
    let mine = [LURING, LURING];
    let theirs = [BEAR];
    let (me, them) = ids(&mine, &theirs);
    let (engine, question) = blocks(&mine, &theirs, &[me[0], me[1]], |_| {});
    assert_eq!(question.obeying.len(), 1);
    for lure in [me[0], me[1]] {
        let (mut engine, _) = blocks(&mine, &theirs, &[me[0], me[1]], |_| {});
        declare(&mut engine, &[(them[0], lure)]).expect("either lure is as good");
    }
    let (mut again, _) = blocks(&mine, &theirs, &[me[0], me[1]], |_| {});
    refused(declare(&mut again, &[]), MUST_BLOCK);
    refused(
        declare(&mut again, &[(them[0], me[0]), (them[0], me[1])]),
        "creature cannot block that many attackers",
    );
    obeying_is_accepted(engine, &question);
}

// ------------------------------------------------------------- capacity

/// "Can block an additional creature each combat": two of three attackers
/// and not the third, and one attacker named twice is refused. The
/// question says how many.
#[test]
fn a_creature_that_blocks_one_more_blocks_two_and_not_three() {
    let mine = [BEAR, BEAR, BEAR];
    let theirs = [GIANT];
    let (me, them) = ids(&mine, &theirs);
    let giant = them[0];
    let (mut engine, question) = blocks(&mine, &theirs, &me, |_| {});
    assert_eq!(
        question.capacity,
        vec![BlockCapacity {
            blocker: giant,
            most: Some(2)
        }]
    );
    assert!(question.obeying.is_empty(), "nothing must block");
    refused(
        declare(
            &mut engine,
            &[(giant, me[0]), (giant, me[1]), (giant, me[2])],
        ),
        "creature cannot block that many attackers",
    );
    refused(
        declare(&mut engine, &[(giant, me[0]), (giant, me[0])]),
        "one choice named twice",
    );
    declare(&mut engine, &[(giant, me[0]), (giant, me[1])]).unwrap();
    assert_eq!(
        engine.state().combat.blocked_by(giant),
        vec![me[0], me[1]],
        "it blocks both"
    );
    // Its 4 damage is divided between the two by its controller
    // (CR 510.1d).
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player,
                reason: crate::choice::NumberPrompt::CombatDamage { source, .. },
                ..
            } => {
                assert_eq!((player, source), (THEM, giant));
                return;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    panic!("nobody was asked to divide the giant's damage");
}

// ------------------------------------------------ blocks each attacker

/// "Can block any number of creatures; blocks each attacking creature if
/// able": every attacker it may block, and a menace attacker it may not
/// block alone is left out when nothing else can help.
#[test]
fn a_creature_that_blocks_each_attacker_blocks_every_one_it_may() {
    let mine = [BEAR, BEAR, MENACING];
    let theirs = [BLAZED];
    let (me, them) = ids(&mine, &theirs);
    let blazed = them[0];
    let (mut engine, question) = blocks(&mine, &theirs, &me, |_| {});
    assert_eq!(
        question.capacity,
        vec![BlockCapacity {
            blocker: blazed,
            most: None
        }]
    );
    assert_eq!(question.obeying, vec![(blazed, me[0]), (blazed, me[1])]);
    refused(declare(&mut engine, &[(blazed, me[0])]), MUST_BLOCK);
    obeying_is_accepted(engine, &question);
}

/// With a second creature on the table, the menace attacker can be
/// blocked, so blocking it too is part of the most that can be obeyed —
/// and the other creature has to help, or the declaration breaks menace.
#[test]
fn a_menace_attacker_it_must_block_takes_a_second_blocker() {
    let mine = [BEAR, BEAR, MENACING];
    let theirs = [BLAZED, BEAR];
    let (me, them) = ids(&mine, &theirs);
    let (blazed, helper) = (them[0], them[1]);
    let (mut engine, question) = blocks(&mine, &theirs, &me, |_| {});
    assert_eq!(
        question.obeying,
        vec![
            (blazed, me[0]),
            (blazed, me[1]),
            (blazed, me[2]),
            (helper, me[2])
        ]
    );
    refused(
        declare(&mut engine, &[(blazed, me[0]), (blazed, me[1])]),
        MUST_BLOCK,
    );
    refused(
        declare(
            &mut engine,
            &[(blazed, me[0]), (blazed, me[1]), (blazed, me[2])],
        ),
        "too few blockers for that attacker",
    );
    obeying_is_accepted(engine, &question);
}

/// A creature that must block each attacker and may block one: an
/// attacker without menace first, since blocking a menace attacker needs
/// a second creature that here has its own requirement to obey. Blocking
/// the plain attacker and the flier obeys two; blocking the menace
/// attacker obeys one at most, so a declaration obeying one is refused.
#[test]
fn a_blocker_that_may_block_one_obeys_where_nobody_else_is_needed() {
    let mine = [MENACING, BEAR, LURING_FLIER];
    let theirs = [DUTIFUL, FLIER];
    let (me, them) = ids(&mine, &theirs);
    let (menacing, bear, lure) = (me[0], me[1], me[2]);
    let (dutiful, flier) = (them[0], them[1]);
    let (mut engine, question) = blocks(&mine, &theirs, &me, |_| {});
    assert_eq!(question.obeying, vec![(dutiful, bear), (flier, lure)]);
    refused(declare(&mut engine, &[(flier, lure)]), MUST_BLOCK);
    refused(declare(&mut engine, &[(dutiful, bear)]), MUST_BLOCK);
    refused(
        declare(&mut engine, &[(dutiful, menacing), (flier, menacing)]),
        MUST_BLOCK,
    );
    obeying_is_accepted(engine, &question);
}
