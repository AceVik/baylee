//! `cards/instants/mv_1/camouflage.rs`, played.
//!
//! "Cast this spell only during your declare attackers step. This turn,
//! instead of declaring blockers, each defending player chooses any number of
//! creatures they control and divides them into a number of piles equal to the
//! number of attacking creatures for whom that player is the defending player.
//! Creatures those players control that can block additional creatures may
//! likewise be put into additional piles. Assign each pile to a different one
//! of those attacking creatures at random. Each creature in a pile that can
//! block the creature that pile is assigned to does so. (Piles can be empty.)"
//!
//! Every question is driven by hand: the pile questions are
//! `ChoicePrompt::CamouflagePile` on `Pending::ChooseCards`, which
//! `pass_until` has no answer for (and a blind answer would prove nothing).
//! The blocks are read off the combat state once the last pile is named, the
//! way `make_blocks` left them.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

fn camouflage() -> CardIndex {
    card_index("9cf44db4-627a-4197-9588-6da72e41f03d")
}

/// Keeps every opening hand.
fn start(duel: Duel) -> Engine<RegistryLookup> {
    let mut engine = duel.start();
    keep_mulligans(&mut engine);
    engine
}

/// Passes to the question that asks `seat` for attackers.
#[track_caller]
fn walk_to_attack(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
}

/// `who` declares `attacks` (attacker, defending player); a band question, if
/// the attackers raise one, is answered with `band`. Leaves `who` holding
/// priority in the declare attackers step.
#[track_caller]
fn declare(
    engine: &mut Engine<RegistryLookup>,
    who: PlayerId,
    attacks: &[(ObjectId, PlayerId)],
    band: &[ObjectId],
) {
    engine
        .apply(
            who,
            PlayerAction::DeclareAttackers {
                attackers: attacks
                    .iter()
                    .map(|&(a, d)| (a, Defender::Player(d)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
    if let Pending::ChooseCards {
        player,
        prompt: ChoicePrompt::Band { .. },
        ..
    } = engine.pending().clone()
    {
        engine
            .apply(
                player,
                PlayerAction::ChooseObjects {
                    objects: band.to_vec(),
                },
            )
            .expect("the band answer is legal");
    }
    assert_eq!(engine.state().turn.step, Step::DeclareAttackers);
}

/// `who` casts Camouflage out of hand, and the spell resolves.
#[track_caller]
fn cast(engine: &mut Engine<RegistryLookup>, who: PlayerId) {
    cast_from_hand(engine, who, camouflage());
    assert!(!stack_is_empty(engine), "the spell is on the stack");
    pass_until(engine, stack_is_empty);
    assert!(
        engine.state().per_turn.camouflage,
        "resolved: blocks are made by piles this turn"
    );
}

/// Passes priority out of the declare attackers step: what comes next is the
/// first pile question, or, when nobody has anything to put in a pile, the
/// declare blockers step with its priority.
#[track_caller]
fn to_blocks(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| e.state().turn.step != Step::DeclareAttackers);
}

/// The pile question standing: who is asked, which pile, of how many, and
/// what may go in it (sorted).
fn question(engine: &Engine<RegistryLookup>) -> Option<(PlayerId, u8, u8, Vec<ObjectId>)> {
    match engine.pending() {
        Pending::ChooseCards {
            player,
            options,
            min: 0,
            prompt: ChoicePrompt::CamouflagePile { pile, of },
            ..
        } => {
            let mut options = options.clone();
            options.sort();
            Some((*player, *pile, *of, options))
        }
        _ => None,
    }
}

/// The standing question, which must be pile `pile` of `of` for `seat`.
#[track_caller]
fn asked(engine: &Engine<RegistryLookup>, seat: PlayerId, pile: u8, of: u8) -> Vec<ObjectId> {
    let Some((player, n, total, options)) = question(engine) else {
        panic!("expected a pile question, got {:?}", engine.pending())
    };
    assert_eq!(
        (player, n, total),
        (seat, pile, of),
        "who is asked which pile"
    );
    options
}

/// Names a pile.
#[track_caller]
fn name_pile(engine: &mut Engine<RegistryLookup>, seat: PlayerId, pile: &[ObjectId]) {
    engine
        .apply(
            seat,
            PlayerAction::ChooseObjects {
                objects: pile.to_vec(),
            },
        )
        .expect("the pile is legal");
}

/// Every block now standing as (blocker, attacker), attacker by attacker in
/// the order given.
fn blocks_on(engine: &Engine<RegistryLookup>, attackers: &[ObjectId]) -> Vec<(ObjectId, ObjectId)> {
    attackers
        .iter()
        .flat_map(|&attacker| {
            engine
                .state()
                .combat
                .blockers_of(attacker)
                .into_iter()
                .map(move |blocker| (blocker, attacker))
        })
        .collect()
}

fn sorted(mut objects: Vec<ObjectId>) -> Vec<ObjectId> {
    objects.sort();
    objects
}

/// Refuses Camouflage at `engine`'s current priority window: with every
/// Forest tapped for its mana (so the cost is never the reason) the spell is
/// not offered, the cast is an error, and nothing was put on the stack.
#[track_caller]
fn refused(engine: &mut Engine<RegistryLookup>, seat: PlayerId, when: &str) {
    assert!(
        tap_all_mana(engine, seat) > 0,
        "{when}: the Forest is untapped, so the mana is there"
    );
    let spell = in_hand(engine, seat, camouflage()).expect("Camouflage is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("{when}: expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&spell),
        "{when}: Camouflage is offered"
    );
    assert!(
        engine
            .apply(seat, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "{when}: the cast is refused"
    );
    assert!(stack_is_empty(engine), "{when}: nothing was cast");
    assert!(
        !engine.state().per_turn.camouflage,
        "{when}: and nothing is on"
    );
}

/// Camouflage is written: coverage, and one ability that is its spell.
#[test]
fn camouflage_is_written() {
    let camouflage = camouflage();
    let def = baylee_cards::by_index(camouflage).expect("in the pool");
    assert_eq!(def.coverage, baylee_cards_dsl::Coverage::Implemented);
    assert!(!def.abilities.is_empty());
}

/// "Cast this spell only during your declare attackers step": offered and cast
/// there, and nowhere else: not in the controller's main phase, not at the
/// beginning of its combat, not in its declare blockers step, and not in the
/// opponent's declare attackers step (the controller does hold priority
/// there, with the mana and the card — only "your" is missing).
///
/// Every refusal taps the Forest first in the same window, so the cost is
/// paid and the timing is the only thing left that can say no; the offered
/// case on the same board is the control.
#[test]
#[allow(clippy::too_many_lines)] // four wrong windows and the right one, each on its own board
fn camouflage_is_cast_only_during_your_own_declare_attackers_step() {
    // A fresh board for each window: a Forest tapped in one is still tapped in
    // the next, and a refusal that had no mana to blame would prove nothing.
    let board = |seed| {
        let mut engine = start(
            Duel::new(seed, index::FOREST)
                .battlefield(0, &[index::FOREST, index::HILL_GIANT])
                .hand(0, &[camouflage()])
                .battlefield(1, &[index::GRIZZLY_BEARS]),
        );
        reach_main_phase(&mut engine, P0);
        engine
    };

    let mut engine = board(9101);
    refused(&mut engine, P0, "my main phase");

    let mut engine = board(9104);
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    refused(&mut engine, P0, "my beginning of combat");

    // The control: the same board, the right step.
    let mut engine = board(9105);
    walk_to_attack(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    declare(&mut engine, P0, &[(giant, P1)], &[]);
    assert!(tap_all_mana(&mut engine, P0) > 0);
    let spell = in_hand(&engine, P0, camouflage()).unwrap();
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "my declare attackers step: offered"
    );
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .expect("my declare attackers step: cast");
    assert!(!stack_is_empty(&engine));

    // My declare blockers step: P1 declares an ordinary block, and I get
    // priority after it.
    let mut engine = start(
        Duel::new(9102, index::FOREST)
            .battlefield(0, &[index::FOREST, index::HILL_GIANT])
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::GRIZZLY_BEARS]),
    );
    reach_main_phase(&mut engine, P0);
    walk_to_attack(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    declare(&mut engine, P0, &[(giant, P1)], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, giant)],
            },
        )
        .unwrap();
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player, .. } if *player == P0
    ));
    refused(&mut engine, P0, "my declare blockers step");

    // The opponent's declare attackers step: the same step, not my turn.
    let mut engine = start(
        Duel::new(9103, index::FOREST)
            .battlefield(0, &[index::FOREST])
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::HILL_GIANT]),
    );
    walk_to_attack(&mut engine, P1);
    let theirs = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(theirs, Defender::Player(P0))],
            },
        )
        .unwrap();
    assert_eq!(engine.state().turn.step, Step::DeclareAttackers);
    assert_eq!(engine.state().turn.active, P1);
    // The active player holds priority first; when it passes, I do.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state().turn.step, Step::DeclareAttackers);
    assert!(matches!(
        engine.pending(),
        Pending::Priority { player, .. } if *player == P0
    ));
    refused(&mut engine, P0, "the opponent's declare attackers step");
}

/// One pile per attacker, each creature in one pile only.
///
/// Three attackers, four potential blockers. Pile 1 takes two of them, so
/// pile 2 offers the other two and not those; pile 3 offers what is left, and
/// is named empty ("piles can be empty"). The creatures of one pile all block
/// the one attacker that pile drew, the second pile's creature a different
/// one, and the third attacker is not blocked.
#[test]
fn there_are_as_many_piles_as_attackers_and_a_creature_is_in_one() {
    for seed in 9201..9209 {
        let mut engine = start(
            Duel::new(seed, index::FOREST)
                .battlefield(
                    0,
                    &[
                        index::FOREST,
                        index::LLANOWAR_ELVES,
                        index::LLANOWAR_ELVES,
                        index::LLANOWAR_ELVES,
                    ],
                )
                .hand(0, &[camouflage()])
                .battlefield(
                    1,
                    &[
                        index::GRIZZLY_BEARS,
                        index::HILL_GIANT,
                        index::GRAY_OGRE,
                        index::CRAW_WURM,
                    ],
                ),
        );
        reach_main_phase(&mut engine, P0);
        let elves = all_on_battlefield(&engine, P0, index::LLANOWAR_ELVES);
        assert_eq!(elves.len(), 3);
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
        let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
        let wurm = on_battlefield(&engine, P1, index::CRAW_WURM).unwrap();

        walk_to_attack(&mut engine, P0);
        declare(
            &mut engine,
            P0,
            &[(elves[0], P1), (elves[1], P1), (elves[2], P1)],
            &[],
        );
        cast(&mut engine, P0);
        to_blocks(&mut engine);

        assert_eq!(
            asked(&engine, P1, 1, 3),
            sorted(vec![bears, giant, ogre, wurm]),
            "three attackers, three piles; every creature may go in the first"
        );
        name_pile(&mut engine, P1, &[bears, giant]);
        assert_eq!(
            asked(&engine, P1, 2, 3),
            sorted(vec![ogre, wurm]),
            "the creatures already in a pile are in no other"
        );
        name_pile(&mut engine, P1, &[ogre]);
        assert_eq!(
            asked(&engine, P1, 3, 3),
            vec![wurm],
            "and the third pile is offered what is left"
        );
        name_pile(&mut engine, P1, &[]);
        assert!(
            question(&engine).is_none(),
            "three piles, no fourth question"
        );

        let made = blocks_on(&engine, &elves);
        assert_eq!(
            made.len(),
            3,
            "two in the first pile, one in the second: {made:?}"
        );
        let target = |blocker| made.iter().find(|b| b.0 == blocker).map(|b| b.1);
        assert_eq!(target(bears), target(giant), "one pile, one attacker");
        assert!(target(bears).is_some());
        assert!(target(ogre).is_some());
        assert_ne!(target(bears), target(ogre), "two piles, two attackers");
        assert_eq!(target(wurm), None, "left in no pile, it blocks nothing");
        assert_eq!(
            elves
                .iter()
                .filter(|e| engine.state().combat.is_blocked(**e))
                .count(),
            2,
            "the empty pile's attacker is not blocked"
        );
    }
}

/// When the creatures run out before the piles do, the rest are empty and
/// nobody is asked for them: three attackers, two creatures, both in the
/// first pile.
#[test]
fn piles_left_when_the_creatures_run_out_are_empty_and_not_asked() {
    let mut engine = start(
        Duel::new(9210, index::FOREST)
            .battlefield(
                0,
                &[
                    index::FOREST,
                    index::LLANOWAR_ELVES,
                    index::LLANOWAR_ELVES,
                    index::LLANOWAR_ELVES,
                ],
            )
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT]),
    );
    reach_main_phase(&mut engine, P0);
    let elves = all_on_battlefield(&engine, P0, index::LLANOWAR_ELVES);
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    walk_to_attack(&mut engine, P0);
    declare(
        &mut engine,
        P0,
        &[(elves[0], P1), (elves[1], P1), (elves[2], P1)],
        &[],
    );
    cast(&mut engine, P0);
    to_blocks(&mut engine);
    asked(&engine, P1, 1, 3);
    name_pile(&mut engine, P1, &[bears, giant]);
    assert!(
        question(&engine).is_none(),
        "no creature is left for a second pile: it is not asked"
    );
    let made = blocks_on(&engine, &elves);
    assert_eq!(made.len(), 2, "both in the one pile: {made:?}");
    assert_eq!(made[0].1, made[1].1, "and both block the same attacker");
}

/// "Creatures … that can block additional creatures may likewise be put into
/// additional piles": the Two-Headed Giant of Foriys (it can block an
/// additional creature) goes in two piles, an Ogre in one.
///
/// Three attackers. The Giant is named in pile 1, and pile 2 still offers it
/// (used once of its two); named there too, pile 3 no longer does. It then
/// blocks two different attackers, one per pile it is in.
#[test]
fn a_creature_that_can_block_additional_creatures_goes_in_additional_piles() {
    let mut engine = start(
        Duel::new(9301, index::FOREST)
            .battlefield(
                0,
                &[
                    index::FOREST,
                    index::LLANOWAR_ELVES,
                    index::LLANOWAR_ELVES,
                    index::LLANOWAR_ELVES,
                ],
            )
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::TWO_HEADED_GIANT_OF_FORIYS, index::GRAY_OGRE]),
    );
    reach_main_phase(&mut engine, P0);
    let elves = all_on_battlefield(&engine, P0, index::LLANOWAR_ELVES);
    let two_headed = on_battlefield(&engine, P1, index::TWO_HEADED_GIANT_OF_FORIYS).unwrap();
    let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();

    walk_to_attack(&mut engine, P0);
    declare(
        &mut engine,
        P0,
        &[(elves[0], P1), (elves[1], P1), (elves[2], P1)],
        &[],
    );
    cast(&mut engine, P0);
    to_blocks(&mut engine);

    assert_eq!(asked(&engine, P1, 1, 3), sorted(vec![two_headed, ogre]));
    name_pile(&mut engine, P1, &[two_headed]);
    assert_eq!(
        asked(&engine, P1, 2, 3),
        sorted(vec![two_headed, ogre]),
        "the Giant, which can block an additional creature, may go in a second pile; the Ogre too, it is in none yet"
    );
    name_pile(&mut engine, P1, &[two_headed]);
    assert_eq!(
        asked(&engine, P1, 3, 3),
        vec![ogre],
        "two piles is all the Giant can block: the third offers only the Ogre"
    );
    name_pile(&mut engine, P1, &[]);

    let made = blocks_on(&engine, &elves);
    assert_eq!(made.len(), 2, "the Giant blocks twice: {made:?}");
    assert!(made.iter().all(|b| b.0 == two_headed));
    assert_ne!(made[0].1, made[1].1, "two different attackers");
    assert_eq!(
        engine.state().combat.blocked_by(two_headed).len(),
        2,
        "it is blocking both"
    );
    assert_eq!(
        elves
            .iter()
            .filter(|e| engine.state().combat.is_blocked(**e))
            .count(),
        2
    );
}

/// The same seed deals the same piles to the same attackers, and across seeds
/// the draw is a draw: a permutation of three attackers by three single-
/// creature piles, which is `3! = 6` deals, most of which occur over a score
/// of seeds.
#[test]
fn the_draw_is_seeded_the_same_seed_deals_the_same_and_seeds_differ() {
    let play = |seed: u64| -> Vec<usize> {
        let mut engine = start(
            Duel::new(seed, index::FOREST)
                .battlefield(
                    0,
                    &[
                        index::FOREST,
                        index::HILL_GIANT,
                        index::GRIZZLY_BEARS,
                        index::HURLOON_MINOTAUR,
                    ],
                )
                .hand(0, &[camouflage()])
                .battlefield(
                    1,
                    &[index::GRAY_OGRE, index::CRAW_WURM, index::LLANOWAR_ELVES],
                ),
        );
        reach_main_phase(&mut engine, P0);
        let attackers = [
            on_battlefield(&engine, P0, index::HILL_GIANT).unwrap(),
            on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap(),
            on_battlefield(&engine, P0, index::HURLOON_MINOTAUR).unwrap(),
        ];
        let defenders = [
            on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap(),
            on_battlefield(&engine, P1, index::CRAW_WURM).unwrap(),
            on_battlefield(&engine, P1, index::LLANOWAR_ELVES).unwrap(),
        ];
        walk_to_attack(&mut engine, P0);
        declare(&mut engine, P0, &attackers.map(|a| (a, P1)), &[]);
        cast(&mut engine, P0);
        to_blocks(&mut engine);
        for (n, defender) in defenders.iter().enumerate() {
            asked(&engine, P1, u8::try_from(n + 1).unwrap(), 3);
            name_pile(&mut engine, P1, &[*defender]);
        }
        let made = blocks_on(&engine, &attackers);
        assert_eq!(made.len(), 3, "every creature can block: {made:?}");
        defenders
            .iter()
            .map(|d| {
                let hit = made.iter().find(|b| b.0 == *d).expect("it blocks").1;
                attackers.iter().position(|a| *a == hit).unwrap()
            })
            .collect()
    };

    let mut seen = std::collections::BTreeSet::new();
    for seed in 9401..9425 {
        let first = play(seed);
        let mut distinct = first.clone();
        distinct.sort_unstable();
        assert_eq!(
            distinct,
            vec![0, 1, 2],
            "each pile went to a different attacker: {first:?}"
        );
        assert_eq!(first, play(seed), "seed {seed}: the same deal again");
        seen.insert(first);
    }
    assert!(
        seen.len() >= 4,
        "across seeds, most of the six deals occur, got {seen:?}"
    );
}

/// "Each creature in a pile that can block the creature that pile is assigned
/// to does so": a Grizzly Bears cannot block a flier.
///
/// Attackers: a Serra Angel (flying) and a Hill Giant. P1's one pile holds
/// Bears and its own Serra Angel (which can block a flier). The pile goes to
/// either attacker. Dealt the Angel, only the Angel blocks and the Bears
/// don't; dealt the Giant, both block. The Bears never block the flier, and
/// both deals occur.
#[test]
fn a_creature_that_cannot_block_its_piles_attacker_does_not_block_it() {
    let (mut to_flier, mut to_giant) = (0, 0);
    for seed in 9501..9517 {
        let mut engine = start(
            Duel::new(seed, index::FOREST)
                .battlefield(0, &[index::FOREST, index::SERRA_ANGEL, index::HILL_GIANT])
                .hand(0, &[camouflage()])
                .battlefield(1, &[index::GRIZZLY_BEARS, index::SERRA_ANGEL]),
        );
        reach_main_phase(&mut engine, P0);
        let flier = on_battlefield(&engine, P0, index::SERRA_ANGEL).unwrap();
        let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        let guard = on_battlefield(&engine, P1, index::SERRA_ANGEL).unwrap();
        walk_to_attack(&mut engine, P0);
        declare(&mut engine, P0, &[(flier, P1), (giant, P1)], &[]);
        cast(&mut engine, P0);
        to_blocks(&mut engine);

        asked(&engine, P1, 1, 2);
        name_pile(&mut engine, P1, &[bears, guard]);
        assert!(
            question(&engine).is_none(),
            "both creatures are in the first pile; the second is empty and not asked"
        );
        let made = blocks_on(&engine, &[flier, giant]);
        assert!(
            !made.contains(&(bears, flier)),
            "the Bears cannot block a flier: {made:?}"
        );
        if engine.state().combat.is_blocked(flier) {
            to_flier += 1;
            assert_eq!(
                made,
                vec![(guard, flier)],
                "dealt the flier, the Angel blocks it alone, the Giant is unblocked"
            );
        } else {
            to_giant += 1;
            assert_eq!(
                sorted(made.iter().map(|b| b.0).collect()),
                sorted(vec![bears, guard]),
                "dealt the Giant, both block it"
            );
            assert!(made.iter().all(|b| b.1 == giant));
        }
    }
    assert!(to_flier > 0 && to_giant > 0, "{to_flier} / {to_giant}");
}

/// A menace attacker is blocked only if two or more of its pile can block it.
///
/// Attackers: Fearful Villager (menace) and a Hill Giant. With two piles of
/// one creature each, whichever pile draws the Villager is a lone blocker and
/// blocks nothing, and the other blocks the Giant: the Villager is never
/// blocked, in every seed. With one pile of two and one empty, a pile drawn by
/// the Villager blocks it, both together, and one drawn by the Giant blocks
/// the Giant; both draws occur.
#[test]
#[allow(clippy::too_many_lines)] // the lone pile and the pair, both on the same board
fn a_menace_attacker_is_blocked_only_by_a_pile_of_two() {
    let board = |seed| {
        start(
            Duel::new(seed, index::FOREST)
                .battlefield(
                    0,
                    &[index::FOREST, index::FEARFUL_VILLAGER, index::HILL_GIANT],
                )
                .hand(0, &[camouflage()])
                .battlefield(1, &[index::GRIZZLY_BEARS, index::GRAY_OGRE]),
        )
    };

    let mut blocked_giant_by = std::collections::BTreeSet::new();
    for seed in 9601..9613 {
        let mut engine = board(seed);
        reach_main_phase(&mut engine, P0);
        let villager = on_battlefield(&engine, P0, index::FEARFUL_VILLAGER).unwrap();
        let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
        assert!(keywords(&engine, villager).contains(baylee_cards_dsl::KeywordSet::MENACE));
        walk_to_attack(&mut engine, P0);
        declare(&mut engine, P0, &[(villager, P1), (giant, P1)], &[]);
        cast(&mut engine, P0);
        to_blocks(&mut engine);
        asked(&engine, P1, 1, 2);
        name_pile(&mut engine, P1, &[bears]);
        asked(&engine, P1, 2, 2);
        name_pile(&mut engine, P1, &[ogre]);

        assert!(
            !engine.state().combat.is_blocked(villager),
            "a lone creature cannot block a menace attacker, whichever pile it drew"
        );
        let made = blocks_on(&engine, &[villager, giant]);
        assert_eq!(made.len(), 1, "the other pile blocks the Giant: {made:?}");
        assert_eq!(made[0].1, giant);
        blocked_giant_by.insert(made[0].0 == bears);
    }
    assert_eq!(
        blocked_giant_by.len(),
        2,
        "the pile that went to the Giant was the Bears' in some seeds, the Ogre's in others"
    );

    let (mut to_villager, mut to_giant) = (0, 0);
    for seed in 9621..9637 {
        let mut engine = board(seed);
        reach_main_phase(&mut engine, P0);
        let villager = on_battlefield(&engine, P0, index::FEARFUL_VILLAGER).unwrap();
        let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
        walk_to_attack(&mut engine, P0);
        declare(&mut engine, P0, &[(villager, P1), (giant, P1)], &[]);
        cast(&mut engine, P0);
        to_blocks(&mut engine);
        asked(&engine, P1, 1, 2);
        name_pile(&mut engine, P1, &[bears, ogre]);
        assert!(
            question(&engine).is_none(),
            "nothing left for a second pile"
        );

        let made = blocks_on(&engine, &[villager, giant]);
        assert_eq!(made.len(), 2, "the pair block together: {made:?}");
        assert_eq!(made[0].1, made[1].1);
        assert_eq!(
            sorted(made.iter().map(|b| b.0).collect()),
            sorted(vec![bears, ogre])
        );
        if made[0].1 == villager {
            to_villager += 1;
        } else {
            to_giant += 1;
        }
    }
    assert!(
        to_villager > 0 && to_giant > 0,
        "{to_villager} / {to_giant}"
    );
}

/// Block triggers fire for blocks made by piles. Aisling Leprechaun turns the
/// creature it blocks, or is blocked by, green.
///
/// Blocking: a pile of the Leprechaun and the Bears blocks a red Hill Giant,
/// which is green once the trigger resolves; with the Leprechaun left out of
/// the pile it is not. Being blocked: an attacking Leprechaun, blocked by a
/// pile of Hill Giant, turns that Hill Giant green.
#[test]
fn blocks_made_by_piles_trigger_what_a_block_triggers() {
    let green = ColorSet::from_slice(&[Color::Green]);
    let colors = |e: &Engine<RegistryLookup>, id: ObjectId| {
        e.state().object(id).unwrap().characteristics().colors
    };

    for in_pile in [true, false] {
        let mut engine = start(
            Duel::new(9701, index::FOREST)
                .battlefield(0, &[index::FOREST, index::HILL_GIANT])
                .hand(0, &[camouflage()])
                .battlefield(1, &[index::AISLING_LEPRECHAUN, index::GRIZZLY_BEARS]),
        );
        reach_main_phase(&mut engine, P0);
        let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
        let leprechaun = on_battlefield(&engine, P1, index::AISLING_LEPRECHAUN).unwrap();
        let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
        assert_ne!(colors(&engine, giant), green, "red to begin with");
        walk_to_attack(&mut engine, P0);
        declare(&mut engine, P0, &[(giant, P1)], &[]);
        cast(&mut engine, P0);
        to_blocks(&mut engine);
        asked(&engine, P1, 1, 1);
        name_pile(
            &mut engine,
            P1,
            &if in_pile {
                vec![leprechaun, bears]
            } else {
                vec![bears]
            },
        );
        assert_eq!(
            sorted(engine.state().combat.blockers_of(giant)),
            sorted(if in_pile {
                vec![leprechaun, bears]
            } else {
                vec![bears]
            })
        );
        assert_eq!(
            stack_is_empty(&engine),
            !in_pile,
            "the Leprechaun's trigger is on the stack only when it blocked"
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            colors(&engine, giant) == green,
            in_pile,
            "it blocked the Giant, so the Giant is green; left out, it stayed red"
        );
    }

    let mut engine = start(
        Duel::new(9702, index::FOREST)
            .battlefield(0, &[index::FOREST, index::AISLING_LEPRECHAUN])
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::HILL_GIANT]),
    );
    reach_main_phase(&mut engine, P0);
    let leprechaun = on_battlefield(&engine, P0, index::AISLING_LEPRECHAUN).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    assert_ne!(colors(&engine, giant), green);
    walk_to_attack(&mut engine, P0);
    declare(&mut engine, P0, &[(leprechaun, P1)], &[]);
    cast(&mut engine, P0);
    to_blocks(&mut engine);
    asked(&engine, P1, 1, 1);
    name_pile(&mut engine, P1, &[giant]);
    assert!(!stack_is_empty(&engine), "becoming blocked triggers it too");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(colors(&engine, giant), green, "the blocker is turned green");
}

/// "Bands are blocked as a group": a block made by a pile on one member of a
/// band blocks the whole band.
///
/// Benalish Hero (banding) and a Hill Giant attack in a band, and P1's one
/// pile holds the Bears. It draws one of the two attackers, in whatever seed,
/// and both are blocked by the Bears. The control, the same attack with no
/// band declared, has exactly one of the two blocked.
#[test]
fn banding_spreads_a_block_made_by_a_pile_to_the_whole_band() {
    for banded in [true, false] {
        for seed in 9801..9809 {
            let mut engine = start(
                Duel::new(seed, index::FOREST)
                    .battlefield(0, &[index::FOREST, index::BENALISH_HERO, index::HILL_GIANT])
                    .hand(0, &[camouflage()])
                    .battlefield(1, &[index::GRIZZLY_BEARS]),
            );
            reach_main_phase(&mut engine, P0);
            let hero = on_battlefield(&engine, P0, index::BENALISH_HERO).unwrap();
            let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
            let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
            walk_to_attack(&mut engine, P0);
            engine
                .apply(
                    P0,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![
                            (hero, Defender::Player(P1)),
                            (giant, Defender::Player(P1)),
                        ],
                    },
                )
                .unwrap();
            let Pending::ChooseCards {
                prompt: ChoicePrompt::Band { with },
                options,
                ..
            } = engine.pending().clone()
            else {
                panic!("expected the band question, got {:?}", engine.pending())
            };
            assert_eq!((with, options), (hero, vec![giant]));
            engine
                .apply(
                    P0,
                    PlayerAction::ChooseObjects {
                        objects: if banded { vec![giant] } else { vec![] },
                    },
                )
                .unwrap();
            cast(&mut engine, P0);
            to_blocks(&mut engine);
            asked(&engine, P1, 1, 2);
            name_pile(&mut engine, P1, &[bears]);
            assert!(question(&engine).is_none());

            let made = blocks_on(&engine, &[hero, giant]);
            if banded {
                assert_eq!(
                    made,
                    vec![(bears, hero), (bears, giant)],
                    "seed {seed}: one pile blocks the band as a group"
                );
            } else {
                assert_eq!(
                    made.len(),
                    1,
                    "seed {seed}: no band, one attacker: {made:?}"
                );
            }
        }
    }
}

/// A defender with no creatures is asked nothing: the declare blockers step
/// comes with its priority and no question, nothing is blocked, and the
/// attacker's damage goes through.
#[test]
fn a_defender_with_no_creatures_is_asked_nothing() {
    let mut engine = start(
        Duel::new(9901, index::FOREST)
            .battlefield(0, &[index::FOREST, index::HILL_GIANT])
            .hand(0, &[camouflage()]),
    );
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    walk_to_attack(&mut engine, P0);
    declare(&mut engine, P0, &[(giant, P1)], &[]);
    cast(&mut engine, P0);
    to_blocks(&mut engine);

    assert!(question(&engine).is_none(), "asked: {:?}", engine.pending());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the game goes on at once: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().turn.step, Step::DeclareBlockers);
    assert!(!engine.state().combat.is_blocked(giant));
    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    assert_eq!(
        engine.state().players[1].life,
        20 - 3,
        "unblocked, the Giant hit"
    );
}

/// "On the next turn": the effect is of this turn only. Blockers are declared
/// normally again, in the opponent's turn (P0 is asked `ChooseBlockers`) and in
/// P0's next turn (P1 is).
#[test]
fn on_the_next_turn_blockers_are_declared_normally_again() {
    let mut engine = start(
        Duel::new(9951, index::FOREST)
            .battlefield(0, &[index::FOREST, index::GRIZZLY_BEARS, index::HILL_GIANT])
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::CRAW_WURM, index::GRAY_OGRE]),
    );
    reach_main_phase(&mut engine, P0);
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let wurm = on_battlefield(&engine, P1, index::CRAW_WURM).unwrap();
    let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
    walk_to_attack(&mut engine, P0);
    declare(&mut engine, P0, &[(bears, P1)], &[]);
    cast(&mut engine, P0);
    to_blocks(&mut engine);
    asked(&engine, P1, 1, 1);
    name_pile(&mut engine, P1, &[wurm]);
    assert_eq!(engine.state().combat.blockers_of(bears), vec![wurm]);

    // The opponent's turn: P1 attacks (the Wurm stays home), and P0 is asked to declare blockers.
    walk_to_attack(&mut engine, P1);
    assert!(
        !engine.state().per_turn.camouflage,
        "the effect ended with the turn"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ogre, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P0, "the defender declares, as usual");
    engine
        .apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // My next turn: I attack again, and P1 declares a block of its own.
    walk_to_attack(&mut engine, P0);
    assert!(!engine.state().per_turn.camouflage);
    declare(&mut engine, P0, &[(giant, P1)], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wurm, giant)],
            },
        )
        .expect("a declared block, as before");
    assert_eq!(engine.state().combat.blockers_of(giant), vec![wurm]);
}

/// Three seats: each defending player makes their piles, in turn order, with
/// a pile count of their own attackers.
///
/// P0 attacks P1 with two creatures and P2 with one: P1 is asked first, for
/// two piles, and its blocks are made before P2 is asked for one.
#[test]
fn with_three_seats_the_defenders_make_their_piles_in_turn_order() {
    let mut engine = start(
        Duel::table(9961, index::FOREST, 3)
            .battlefield(
                0,
                &[
                    index::FOREST,
                    index::HILL_GIANT,
                    index::HURLOON_MINOTAUR,
                    index::CRAW_WURM,
                ],
            )
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::GRIZZLY_BEARS, index::GRAY_OGRE])
            .battlefield(2, &[index::LLANOWAR_ELVES, index::SERRA_ANGEL]),
    );
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let minotaur = on_battlefield(&engine, P0, index::HURLOON_MINOTAUR).unwrap();
    let wurm = on_battlefield(&engine, P0, index::CRAW_WURM).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let ogre = on_battlefield(&engine, P1, index::GRAY_OGRE).unwrap();
    let elves = on_battlefield(&engine, P2, index::LLANOWAR_ELVES).unwrap();
    let angel = on_battlefield(&engine, P2, index::SERRA_ANGEL).unwrap();

    walk_to_attack(&mut engine, P0);
    declare(
        &mut engine,
        P0,
        &[(giant, P1), (minotaur, P1), (wurm, P2)],
        &[],
    );
    cast(&mut engine, P0);
    to_blocks(&mut engine);

    assert_eq!(
        asked(&engine, P1, 1, 2),
        sorted(vec![bears, ogre]),
        "P1 first, two piles for two attackers, only its own creatures"
    );
    name_pile(&mut engine, P1, &[bears]);
    asked(&engine, P1, 2, 2);
    name_pile(&mut engine, P1, &[ogre]);

    assert_eq!(
        blocks_on(&engine, &[giant, minotaur]).len(),
        2,
        "P1's blocks stand before P2 is asked"
    );
    assert!(!engine.state().combat.is_blocked(wurm));
    assert_eq!(
        asked(&engine, P2, 1, 1),
        sorted(vec![elves, angel]),
        "then P2, one pile for its one attacker"
    );
    name_pile(&mut engine, P2, &[elves, angel]);
    assert!(question(&engine).is_none(), "and that is all of them");
    let made = blocks_on(&engine, &[wurm]);
    assert_eq!(
        sorted(made.iter().map(|b| b.0).collect()),
        sorted(vec![elves, angel]),
        "the whole pile blocks the Wurm: {made:?}"
    );
}

/// Turn order is the order from the active player, not the seat numbers: with
/// P1 active at three seats, P2 is asked before P0.
#[test]
fn the_defenders_are_asked_from_the_active_player_not_from_seat_zero() {
    let mut engine = start(
        Duel::table(9962, index::FOREST, 3)
            .battlefield(0, &[index::GRAY_OGRE])
            .battlefield(
                1,
                &[index::FOREST, index::HILL_GIANT, index::HURLOON_MINOTAUR],
            )
            .hand(1, &[camouflage()])
            .battlefield(2, &[index::GRIZZLY_BEARS]),
    );
    walk_to_attack(&mut engine, P1);
    assert_eq!(engine.state().turn.active, P1);
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let minotaur = on_battlefield(&engine, P1, index::HURLOON_MINOTAUR).unwrap();
    let ogre = on_battlefield(&engine, P0, index::GRAY_OGRE).unwrap();
    let bears = on_battlefield(&engine, P2, index::GRIZZLY_BEARS).unwrap();
    // The Minotaur at P0 first, the Giant at P2 second: declaration order is
    // the reverse of the order they are asked in.
    declare(&mut engine, P1, &[(minotaur, P0), (giant, P2)], &[]);
    cast(&mut engine, P1);
    to_blocks(&mut engine);

    assert_eq!(asked(&engine, P2, 1, 1), vec![bears], "P2 follows P1");
    name_pile(&mut engine, P2, &[bears]);
    assert_eq!(engine.state().combat.blockers_of(giant), vec![bears]);
    assert_eq!(asked(&engine, P0, 1, 1), vec![ogre], "then P0");
    name_pile(&mut engine, P0, &[ogre]);
    assert_eq!(engine.state().combat.blockers_of(minotaur), vec![ogre]);
    assert!(question(&engine).is_none());
}

/// Seats that are not asked: one with no creatures, one nobody attacked.
///
/// P0 attacks P1 and P2. P1 has no creatures, so the first and only question
/// is P2's. And when P0 attacks only P2, P1, which has creatures, is not asked
/// either.
#[test]
fn only_a_defending_player_with_creatures_is_asked() {
    let mut engine = start(
        Duel::table(9971, index::FOREST, 3)
            .battlefield(
                0,
                &[index::FOREST, index::HILL_GIANT, index::HURLOON_MINOTAUR],
            )
            .hand(0, &[camouflage()])
            .battlefield(2, &[index::GRIZZLY_BEARS]),
    );
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let minotaur = on_battlefield(&engine, P0, index::HURLOON_MINOTAUR).unwrap();
    let bears = on_battlefield(&engine, P2, index::GRIZZLY_BEARS).unwrap();
    walk_to_attack(&mut engine, P0);
    declare(&mut engine, P0, &[(giant, P1), (minotaur, P2)], &[]);
    cast(&mut engine, P0);
    to_blocks(&mut engine);
    assert_eq!(
        asked(&engine, P2, 1, 1),
        vec![bears],
        "P1 has nothing to put in a pile: P2 is the first asked"
    );
    name_pile(&mut engine, P2, &[bears]);
    assert!(question(&engine).is_none());
    assert!(!engine.state().combat.is_blocked(giant));
    assert_eq!(engine.state().combat.blockers_of(minotaur), vec![bears]);

    let mut engine = start(
        Duel::table(9972, index::FOREST, 3)
            .battlefield(0, &[index::FOREST, index::HILL_GIANT])
            .hand(0, &[camouflage()])
            .battlefield(1, &[index::GRAY_OGRE])
            .battlefield(2, &[index::GRIZZLY_BEARS]),
    );
    reach_main_phase(&mut engine, P0);
    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let bears = on_battlefield(&engine, P2, index::GRIZZLY_BEARS).unwrap();
    walk_to_attack(&mut engine, P0);
    declare(&mut engine, P0, &[(giant, P2)], &[]);
    cast(&mut engine, P0);
    to_blocks(&mut engine);
    assert_eq!(
        asked(&engine, P2, 1, 1),
        vec![bears],
        "P1 was not attacked: it is not a defending player"
    );
    name_pile(&mut engine, P2, &[bears]);
    assert!(question(&engine).is_none());
    assert_eq!(engine.state().combat.blockers_of(giant), vec![bears]);
}
