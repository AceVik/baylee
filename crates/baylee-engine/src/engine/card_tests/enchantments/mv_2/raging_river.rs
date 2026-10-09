//! `cards/enchantments/mv_2/raging_river.rs`, played.
//!
//! "Whenever one or more creatures you control attack, each defending player
//! divides all creatures without flying they control into a 'left' pile and
//! a 'right' pile. Then, for each attacking creature you control, choose
//! 'left' or 'right.' That creature can't be blocked this combat except by
//! creatures with flying and creatures in a pile with the chosen label."
//!
//! Every question is answered by the test itself: a defender divides with
//! `ChoicePrompt::LeftPile` (`ChooseObjects` names the left pile), the
//! controller labels each attacker through `Pending::ChoosePile` with
//! `label: Some(attacker)` (`ChooseMode(0)` is left, `1` right). The limits
//! sit in `CombatState::pile_limits`, which `can_block` reads, so the proof
//! of a label is the `ChooseBlockers` offer and a refused declaration.
//! The mechanism's own cases are in `engine::pile_block_tests`.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::ChoicePrompt;
use baylee_core::generated::index;
use baylee_core::ids::Defender;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
const P2: PlayerId = PlayerId::new(2);

/// Raging River resolves onto the battlefield and asks nothing until its
/// controller attacks.
#[test]
fn raging_river_is_cast_and_waits_for_an_attack() {
    let raging_river = card_index("a2310312-6e1e-4e34-a351-9aef499a810f");
    assert_eq!(
        cast_saying_nothing(raging_river, mountain(), 2),
        Zone::Battlefield,
        "Raging River"
    );
}

/// Passes priority, and only priority: a question the test did not expect
/// panics instead of being answered on the test's behalf.
#[track_caller]
fn pass_to(engine: &mut Engine<RegistryLookup>, done: impl Fn(&Engine<RegistryLookup>) -> bool) {
    for _ in 0..40 {
        if done(engine) {
            return;
        }
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("a question nobody expected: {:?}", engine.pending());
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
    panic!("the game never got there: {:?}", engine.pending());
}

fn asking_blockers(e: &Engine<RegistryLookup>) -> bool {
    matches!(e.pending(), Pending::ChooseBlockers { .. })
}

fn asking_something(e: &Engine<RegistryLookup>) -> bool {
    !matches!(e.pending(), Pending::Priority { .. })
}

fn asking_attackers(e: &Engine<RegistryLookup>) -> bool {
    matches!(e.pending(), Pending::ChooseAttackers { .. })
}

/// P1 holds priority with nothing on the stack: the River's trigger is
/// resolved and the attack has not moved on.
fn p1_may_act_after_the_trigger(e: &Engine<RegistryLookup>) -> bool {
    stack_is_empty(e) && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
}

fn stack_len(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .len()
}

fn sorted(mut v: Vec<ObjectId>) -> Vec<ObjectId> {
    v.sort();
    v
}

/// A duel where P0 (the River's side) is in its first main phase.
fn river_duel(
    seed: u64,
    p0_board: &[CardIndex],
    p1_board: &[CardIndex],
    p1_hand: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, index::MOUNTAIN)
        .battlefield(0, p0_board)
        .battlefield(1, p1_board)
        .hand(1, p1_hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

#[track_caller]
fn card(engine: &Engine<RegistryLookup>, seat: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(engine, seat, c).expect("the card is on the battlefield")
}

/// P0 declares `attacking` (creature, the player it attacks), in that
/// order, and the attack is left where it stops: with the trigger on the
/// stack.
#[track_caller]
fn declare(engine: &mut Engine<RegistryLookup>, attacking: &[(ObjectId, PlayerId)]) {
    pass_to(engine, asking_attackers);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: attacking
                    .iter()
                    .map(|&(a, at)| (a, Defender::Player(at)))
                    .collect(),
            },
        )
        .unwrap();
}

/// `declare`, then the trigger resolves up to its first question.
#[track_caller]
fn attack(engine: &mut Engine<RegistryLookup>, attacking: &[(ObjectId, PlayerId)]) {
    declare(engine, attacking);
    pass_to(engine, asking_something);
}

/// `who` is asked to divide, offered exactly `grounded`, and names `left`.
#[track_caller]
fn divide(
    engine: &mut Engine<RegistryLookup>,
    who: PlayerId,
    grounded: &[ObjectId],
    left: &[ObjectId],
) {
    let Pending::ChooseCards {
        player,
        options,
        prompt: ChoicePrompt::LeftPile,
        min: 0,
        ..
    } = engine.pending().clone()
    else {
        panic!("a division is asked: {:?}", engine.pending());
    };
    assert_eq!(player, who, "who divides");
    assert_eq!(
        sorted(options),
        sorted(grounded.to_vec()),
        "the creatures without flying, and no other"
    );
    engine
        .apply(
            who,
            PlayerAction::ChooseObjects {
                objects: left.to_vec(),
            },
        )
        .unwrap();
}

/// P0 is asked for `attacker`'s label over exactly these piles, and picks
/// one (0 left, 1 right).
#[track_caller]
fn label(
    engine: &mut Engine<RegistryLookup>,
    attacker: ObjectId,
    left: &[ObjectId],
    right: &[ObjectId],
    pick: usize,
) {
    let Pending::ChoosePile {
        player,
        piles,
        label,
    } = engine.pending().clone()
    else {
        panic!("a label is asked: {:?}", engine.pending());
    };
    assert_eq!((player, label), (P0, Some(attacker)), "whose label");
    assert_eq!(piles.len(), 2, "left and right");
    assert_eq!(sorted(piles[0].clone()), sorted(left.to_vec()), "left pile");
    assert_eq!(
        sorted(piles[1].clone()),
        sorted(right.to_vec()),
        "right pile"
    );
    engine.apply(P0, PlayerAction::ChooseMode(pick)).unwrap();
}

/// Whom the blocking player may have `blocker` block, from the offer.
#[track_caller]
fn may_block(engine: &Engine<RegistryLookup>, blocker: ObjectId) -> Vec<ObjectId> {
    let Pending::ChooseBlockers { blockers, .. } = engine.pending() else {
        panic!("blockers are asked: {:?}", engine.pending());
    };
    sorted(
        blockers
            .iter()
            .find(|b| b.blocker == blocker)
            .map(|b| b.attackers.clone())
            .unwrap_or_default(),
    )
}

/// `player` declares `blocks` and is refused, and is still asked.
#[track_caller]
fn refused(engine: &mut Engine<RegistryLookup>, player: PlayerId, blocks: &[(ObjectId, ObjectId)]) {
    assert!(
        engine
            .apply(
                player,
                PlayerAction::DeclareBlockers {
                    blockers: blocks.to_vec(),
                },
            )
            .is_err(),
        "{blocks:?} is refused"
    );
    assert!(
        asking_blockers(engine),
        "still asking: {:?}",
        engine.pending()
    );
}

#[track_caller]
fn blocks(engine: &mut Engine<RegistryLookup>, player: PlayerId, pairs: &[(ObjectId, ObjectId)]) {
    engine
        .apply(
            player,
            PlayerAction::DeclareBlockers {
                blockers: pairs.to_vec(),
            },
        )
        .unwrap();
    for &(blocker, attacker) in pairs {
        assert!(engine.state().combat.is_blocking(blocker, attacker));
    }
}

/// P1, holding priority, casts a Benalish Knight (flash) from hand and lets
/// it resolve; the game is left at the declaration of blockers.
#[track_caller]
fn flash_in_the_knight(engine: &mut Engine<RegistryLookup>) -> ObjectId {
    cast_from_hand(engine, P1, index::BENALISH_KNIGHT);
    pass_to(engine, asking_blockers);
    card(engine, P1, index::BENALISH_KNIGHT)
}

/// 1. Only creatures without flying are divided: a flier is never offered,
///    is in neither pile, and still blocks either label's attacker.
#[test]
fn a_flier_is_never_divided_and_blocks_whatever_it_likes() {
    let mut engine = river_duel(
        8101,
        &[index::RAGING_RIVER, index::HILL_GIANT, index::GRAY_OGRE],
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::SERRA_ANGEL,
            index::BIRDS_OF_PARADISE,
        ],
        &[],
    );
    let (giant, ogre) = (
        card(&engine, P0, index::HILL_GIANT),
        card(&engine, P0, index::GRAY_OGRE),
    );
    let (bears, elves) = (
        card(&engine, P1, index::GRIZZLY_BEARS),
        card(&engine, P1, index::LLANOWAR_ELVES),
    );
    let (angel, birds) = (
        card(&engine, P1, index::SERRA_ANGEL),
        card(&engine, P1, index::BIRDS_OF_PARADISE),
    );
    attack(&mut engine, &[(giant, P1), (ogre, P1)]);

    // A flier named among the left pile is no answer.
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![angel]
                }
            )
            .is_err(),
        "a creature with flying is not in the division"
    );
    divide(&mut engine, P1, &[bears, elves], &[bears]);
    label(&mut engine, giant, &[bears], &[elves], 0);
    label(&mut engine, ogre, &[bears], &[elves], 1);

    pass_to(&mut engine, asking_blockers);
    assert_eq!(may_block(&engine, bears), vec![giant], "left, the Giant's");
    assert_eq!(may_block(&engine, elves), vec![ogre], "right, the Ogre's");
    let both = sorted(vec![giant, ogre]);
    assert_eq!(may_block(&engine, angel), both, "flying is never held back");
    assert_eq!(may_block(&engine, birds), both, "nor the other flier");

    refused(&mut engine, P1, &[(bears, ogre)]);
    blocks(&mut engine, P1, &[(birds, giant), (angel, ogre)]);
}

/// 2. A labelled attacker is blocked by its pile and by fliers alone, and a
///    block across the river is refused. Labelled "right", so a reading that
///    always meant the first pile cannot pass.
#[test]
fn a_block_across_the_river_is_refused() {
    let mut engine = river_duel(
        8102,
        &[index::RAGING_RIVER, index::HILL_GIANT],
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::SERRA_ANGEL,
        ],
        &[],
    );
    let giant = card(&engine, P0, index::HILL_GIANT);
    let (bears, elves) = (
        card(&engine, P1, index::GRIZZLY_BEARS),
        card(&engine, P1, index::LLANOWAR_ELVES),
    );
    let angel = card(&engine, P1, index::SERRA_ANGEL);
    attack(&mut engine, &[(giant, P1)]);
    divide(&mut engine, P1, &[bears, elves], &[bears]);
    label(&mut engine, giant, &[bears], &[elves], 1);

    pass_to(&mut engine, asking_blockers);
    assert_eq!(may_block(&engine, bears), vec![], "left is the far bank");
    assert_eq!(may_block(&engine, elves), vec![giant]);
    assert_eq!(may_block(&engine, angel), vec![giant]);
    refused(&mut engine, P1, &[(bears, giant)]);
    refused(&mut engine, P1, &[(bears, giant), (elves, giant)]);
    blocks(&mut engine, P1, &[(elves, giant), (angel, giant)]);
}

/// 3. Two attackers are one trigger: one division, then a label for each
///    attacker in the order they were declared (not battlefield order).
#[test]
fn two_attackers_trigger_once_and_are_labelled_in_declaration_order() {
    let mut engine = river_duel(
        8103,
        &[index::RAGING_RIVER, index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[index::GRAY_OGRE, index::LLANOWAR_ELVES],
        &[],
    );
    let (bears, giant) = (
        card(&engine, P0, index::GRIZZLY_BEARS),
        card(&engine, P0, index::HILL_GIANT),
    );
    let (ogre, elves) = (
        card(&engine, P1, index::GRAY_OGRE),
        card(&engine, P1, index::LLANOWAR_ELVES),
    );
    declare(&mut engine, &[(giant, P1), (bears, P1)]);
    assert_eq!(stack_len(&engine), 1, "one trigger for the attack");
    pass_to(&mut engine, asking_something);

    divide(&mut engine, P1, &[ogre, elves], &[ogre]);
    label(&mut engine, giant, &[ogre], &[elves], 0);
    label(&mut engine, bears, &[ogre], &[elves], 1);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no second division, no third label: {:?}",
        engine.pending()
    );

    pass_to(&mut engine, asking_blockers);
    assert_eq!(may_block(&engine, ogre), vec![giant]);
    assert_eq!(may_block(&engine, elves), vec![bears]);
}

/// 4. The River is "creatures you control attack": when its controller is
///    the defender nothing is asked and the blocks are free.
#[test]
fn the_defenders_river_asks_nothing_of_the_opponents_attack() {
    let mut engine = river_duel(
        8104,
        &[
            index::RAGING_RIVER,
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
        ],
        &[index::HILL_GIANT],
        &[],
    );
    let (bears, elves) = (
        card(&engine, P0, index::GRIZZLY_BEARS),
        card(&engine, P0, index::LLANOWAR_ELVES),
    );
    let giant = card(&engine, P1, index::HILL_GIANT);
    reach_their_main_phase(&mut engine, P1);
    pass_to(&mut engine, asking_attackers);
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    // `pass_to` panics on any question that is not priority, which is the
    // assertion: no division and no label before the blockers.
    pass_to(&mut engine, asking_blockers);
    assert_eq!(may_block(&engine, bears), vec![giant]);
    assert_eq!(may_block(&engine, elves), vec![giant]);
    blocks(&mut engine, P0, &[(bears, giant), (elves, giant)]);
}

/// 5. A creature that arrives after the division is in no pile: it may not
///    block the labelled attacker (a flash creature, cast once the trigger
///    has resolved).
#[test]
fn a_creature_that_entered_after_the_division_is_in_no_pile() {
    let mut engine = river_duel(
        8105,
        &[index::RAGING_RIVER, index::HILL_GIANT],
        &[
            index::GRIZZLY_BEARS,
            index::PLAINS,
            index::PLAINS,
            index::PLAINS,
        ],
        &[index::BENALISH_KNIGHT],
    );
    let giant = card(&engine, P0, index::HILL_GIANT);
    let bears = card(&engine, P1, index::GRIZZLY_BEARS);
    attack(&mut engine, &[(giant, P1)]);
    divide(&mut engine, P1, &[bears], &[bears]);
    label(&mut engine, giant, &[bears], &[], 0);
    pass_to(&mut engine, p1_may_act_after_the_trigger);

    let knight = flash_in_the_knight(&mut engine);
    assert_eq!(may_block(&engine, bears), vec![giant], "the pile");
    assert_eq!(
        may_block(&engine, knight),
        vec![],
        "the newcomer is no pile"
    );
    refused(&mut engine, P1, &[(knight, giant)]);
    blocks(&mut engine, P1, &[(bears, giant)]);
}

/// 6. Every defending creature flies: no division, no label, and only fliers
///    may block. A creature that arrives afterwards (flash) is no flier and
///    so may not block at all.
#[test]
fn with_only_fliers_to_divide_nothing_is_asked_and_only_fliers_block() {
    let mut engine = river_duel(
        8106,
        &[index::RAGING_RIVER, index::HILL_GIANT],
        &[
            index::SERRA_ANGEL,
            index::AIR_ELEMENTAL,
            index::PLAINS,
            index::PLAINS,
            index::PLAINS,
        ],
        &[index::BENALISH_KNIGHT],
    );
    let giant = card(&engine, P0, index::HILL_GIANT);
    let (angel, elemental) = (
        card(&engine, P1, index::SERRA_ANGEL),
        card(&engine, P1, index::AIR_ELEMENTAL),
    );
    declare(&mut engine, &[(giant, P1)]);
    assert_eq!(stack_len(&engine), 1, "the trigger is on the stack");
    // `pass_to` panics on a question: the trigger resolves, asks nothing,
    // and priority comes round to P1.
    pass_to(&mut engine, p1_may_act_after_the_trigger);

    let knight = flash_in_the_knight(&mut engine);
    assert_eq!(may_block(&engine, angel), vec![giant]);
    assert_eq!(may_block(&engine, elemental), vec![giant]);
    assert_eq!(may_block(&engine, knight), vec![], "only fliers");
    refused(&mut engine, P1, &[(knight, giant)]);
    blocks(&mut engine, P1, &[(angel, giant), (elemental, giant)]);
}

/// 7. At a table of three P1 divides, then P2, and the label P0 chooses is
///    asked over both players' piles together: left is P1's left and P2's
///    left. An attacker on each player shows each side of the river.
#[test]
fn a_label_is_asked_over_both_players_piles() {
    let mut engine = Duel::table(8107, index::MOUNTAIN, 3)
        .battlefield(
            0,
            &[
                index::RAGING_RIVER,
                index::GRAY_OGRE,
                index::PEARLED_UNICORN,
            ],
        )
        .battlefield(1, &[index::GRIZZLY_BEARS, index::HILL_GIANT])
        .battlefield(2, &[index::LLANOWAR_ELVES, index::CRAW_WURM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let (ogre, unicorn) = (
        card(&engine, P0, index::GRAY_OGRE),
        card(&engine, P0, index::PEARLED_UNICORN),
    );
    let (bears, giant) = (
        card(&engine, P1, index::GRIZZLY_BEARS),
        card(&engine, P1, index::HILL_GIANT),
    );
    let (elves, wurm) = (
        card(&engine, P2, index::LLANOWAR_ELVES),
        card(&engine, P2, index::CRAW_WURM),
    );
    attack(&mut engine, &[(ogre, P1), (unicorn, P2)]);

    divide(&mut engine, P1, &[bears, giant], &[bears]);
    divide(&mut engine, P2, &[elves, wurm], &[elves]);
    label(&mut engine, ogre, &[bears, elves], &[giant, wurm], 0);
    label(&mut engine, unicorn, &[bears, elves], &[giant, wurm], 1);

    pass_to(&mut engine, asking_blockers);
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P1);
    assert_eq!(may_block(&engine, bears), vec![ogre], "P1's left bank");
    assert_eq!(may_block(&engine, giant), vec![], "P1's right bank");
    refused(&mut engine, P1, &[(giant, ogre)]);
    blocks(&mut engine, P1, &[(bears, ogre)]);

    pass_to(&mut engine, asking_blockers);
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!("P2 is asked next: {:?}", engine.pending())
    };
    assert_eq!(player, P2);
    assert_eq!(
        may_block(&engine, elves),
        vec![],
        "P2's left, the Unicorn is right"
    );
    assert_eq!(may_block(&engine, wurm), vec![unicorn], "P2's right bank");
    refused(&mut engine, P2, &[(elves, unicorn)]);
    blocks(&mut engine, P2, &[(wurm, unicorn)]);
}

/// 8. Two Rivers are two triggers, each with its own division and label, and
///    both limits bind: the Ogre may be blocked only by a creature in the
///    chosen pile of each. Either limit alone would let a second creature
///    through (the Elves past the second, the Giant past the first).
#[test]
fn two_rivers_bind_the_attacker_twice() {
    let mut engine = river_duel(
        8108,
        &[index::RAGING_RIVER, index::RAGING_RIVER, index::GRAY_OGRE],
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::HILL_GIANT,
        ],
        &[],
    );
    let ogre = card(&engine, P0, index::GRAY_OGRE);
    let (bears, elves, giant) = (
        card(&engine, P1, index::GRIZZLY_BEARS),
        card(&engine, P1, index::LLANOWAR_ELVES),
        card(&engine, P1, index::HILL_GIANT),
    );
    declare(&mut engine, &[(ogre, P1)]);
    assert_eq!(stack_len(&engine), 2, "one trigger per River");
    pass_to(&mut engine, asking_something);

    let all = [bears, elves, giant];
    divide(&mut engine, P1, &all, &[bears, elves]);
    label(&mut engine, ogre, &[bears, elves], &[giant], 0);
    pass_to(&mut engine, asking_something);
    divide(&mut engine, P1, &all, &[bears, giant]);
    label(&mut engine, ogre, &[bears, giant], &[elves], 0);

    pass_to(&mut engine, asking_blockers);
    assert_eq!(may_block(&engine, bears), vec![ogre], "left of both");
    assert_eq!(may_block(&engine, elves), vec![], "right of the second");
    assert_eq!(may_block(&engine, giant), vec![], "right of the first");
    refused(&mut engine, P1, &[(elves, ogre)]);
    refused(&mut engine, P1, &[(giant, ogre)]);
    blocks(&mut engine, P1, &[(bears, ogre)]);
}
