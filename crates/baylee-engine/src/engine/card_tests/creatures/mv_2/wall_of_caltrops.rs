//! `cards/creatures/mv_2/wall_of_caltrops.rs`, played.
//!
//! Written from the Oracle text and the Comprehensive Rules alone (the card
//! has no Scryfall rulings): CR 603.4 (the intervening "if" is checked when
//! the trigger event happens *and* again as the ability resolves), CR 702.22j
//! (the defending player divides the damage of an attacker blocked by a
//! creature with banding), CR 510.1c (otherwise the attacker's controller
//! does), CR 514.2 ("until end of turn" ends in the cleanup step).
//!
//! Banding is read where a player would meet it: in the projected keywords of
//! the Wall once the trigger has left the stack, and in who is asked to
//! divide the attacker's damage.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::NumberPrompt;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::KeywordSet;
use baylee_core::generated::index;

// oracle_id = "20fa17d4-9c3e-470b-9993-a4a2799e33d1"
fn wall_of_caltrops() -> CardIndex {
    card_index("20fa17d4-9c3e-470b-9993-a4a2799e33d1")
}

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Whether `object` has banding right now, as the layers project it.
fn banded(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    keywords(engine, object).contains(KeywordSet::BANDING)
}

/// Every permanent `seat` controls, in play order.
fn permanents_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
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
}

/// Seat 0 attacks with a Hill Giant (3/3) into `theirs`, every creature seat
/// 1 has blocks it, and the stack is left to empty: what comes back is the
/// declare-blockers step with nothing waiting on the stack.
///
/// Nothing has banding before the block; that is checked here, so each
/// caller's later "has banding" is the block's doing.
#[track_caller]
fn the_giant_is_blocked_by(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = lk::game(SEED, &[index::HILL_GIANT], theirs, &[], &[]);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let blockers = permanents_of(&engine, P1);
    assert_eq!(
        blockers.len(),
        theirs.len(),
        "seat 1 has what the scene names"
    );
    for blocker in &blockers {
        assert!(
            !banded(&engine, *blocker),
            "nothing has banding before the block: {blocker:?}"
        );
    }
    let offer = lk::attack(&mut engine, &[giant]);
    for blocker in &blockers {
        assert!(
            lk::may_block(&offer, *blocker, giant),
            "{blocker:?} may block the Giant: {offer:?}"
        );
    }
    let pairs: Vec<(ObjectId, ObjectId)> = blockers.iter().map(|b| (*b, giant)).collect();
    lk::block(&mut engine, &pairs);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().turn.step,
        Step::DeclareBlockers,
        "the trigger (if any) resolved inside the declare-blockers step"
    );
    (engine, giant)
}

/// Passes priority to the combat damage step and returns the first question
/// about dividing the Giant's damage: who is asked, and over whom.
#[track_caller]
fn first_division(engine: &mut Engine<RegistryLookup>) -> (PlayerId, ObjectId, ObjectId) {
    for _ in 0..30 {
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player,
                reason:
                    NumberPrompt::CombatDamage {
                        source, recipient, ..
                    },
                ..
            } => return (player, source, recipient),
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected on the way to combat damage: {other:?}"),
        }
    }
    panic!("combat damage never asked how to divide it");
}

/// Plays combat out to the second main phase, answering every damage
/// division by giving `spare` nothing and the other blocker everything.
#[track_caller]
fn finish_combat_sparing(engine: &mut Engine<RegistryLookup>, spare: ObjectId) {
    for _ in 0..60 {
        if engine.state().turn.phase == Phase::SecondMain {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber {
                player,
                max,
                reason: NumberPrompt::CombatDamage { recipient, .. },
                ..
            } => {
                let share = if recipient == spare { 0 } else { max };
                engine
                    .apply(player, PlayerAction::ChooseNumber(share))
                    .unwrap();
            }
            other => panic!("unexpected in combat: {other:?}"),
        }
    }
    panic!("combat never ended");
}

/// "Whenever this creature blocks a creature, if at least one other Wall
/// creature is blocking that creature and no non-Wall creatures are blocking
/// that creature, this creature gains banding until end of turn."
///
/// Caltrops and a Wall of Wood block a Hill Giant. When the trigger is gone
/// the Caltrops has banding (the Wood has not gained any: the sentence says
/// "this creature"), and in the damage step the *defending* player is asked
/// to divide the Giant's three damage (CR 702.22j), not its controller. Seat 1
/// puts all of it on the Wood: the Wood dies, the Caltrops (toughness one)
/// takes none, and its own two damage still reaches the Giant.
#[test]
fn caltrops_blocking_beside_another_wall_gains_banding_and_the_defender_divides() {
    let (mut engine, giant) = the_giant_is_blocked_by(&[wall_of_caltrops(), index::WALL_OF_WOOD]);
    let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);

    assert!(
        banded(&engine, caltrops),
        "the Caltrops blocks beside another Wall and no non-Wall: banding"
    );
    assert!(
        !banded(&engine, wood),
        "only the Caltrops gains it: the Wood is a vanilla Wall"
    );

    let (asked, source, _) = first_division(&mut engine);
    assert_eq!(source, giant, "it is the Giant's damage that is divided");
    assert_eq!(
        asked, P1,
        "CR 702.22j: the defending player divides it, as a creature with banding blocks"
    );
    finish_combat_sparing(&mut engine, caltrops);

    assert!(
        on_battlefield(&engine, P0, index::HILL_GIANT).is_some(),
        "the Giant, a 3/3 that took two, is still there"
    );
    assert_eq!(
        lk::damage_on(&engine, giant),
        2,
        "the Caltrops' two damage (the Wood has none) reached the Giant"
    );
    assert!(
        on_battlefield(&engine, P1, wall_of_caltrops()).is_some(),
        "the Caltrops, a 2/1, was given none of the Giant's damage and lives"
    );
    assert!(
        in_graveyard(&engine, P1, index::WALL_OF_WOOD).is_some(),
        "the Wood took all three and died"
    );
}

/// "… if at least one other Wall creature is blocking that creature": a
/// second Caltrops is such a Wall. Two Caltrops block the Giant; each one
/// has the other as its "other Wall", so both trigger and both gain banding.
#[test]
fn two_caltrops_blocking_together_both_gain_banding() {
    let (mut engine, giant) = the_giant_is_blocked_by(&[wall_of_caltrops(), wall_of_caltrops()]);
    let both = permanents_of(&engine, P1);
    assert_eq!(both.len(), 2, "two Caltrops are out");
    for caltrops in &both {
        assert!(
            banded(&engine, *caltrops),
            "{caltrops:?} blocks beside the other Caltrops, a Wall: banding"
        );
    }
    let (asked, source, _) = first_division(&mut engine);
    assert_eq!((asked, source), (P1, giant), "the defender divides");
}

/// The other side of the condition: "at least one *other* Wall creature".
/// Blocking alone, the Caltrops has none and does not trigger. The control is
/// the same Caltrops with a Wall of Wood beside it, which does get banding.
#[test]
fn a_lone_caltrops_does_not_gain_banding() {
    let (mut engine, giant) = the_giant_is_blocked_by(&[wall_of_caltrops()]);
    let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
    assert!(
        !banded(&engine, caltrops),
        "blocking alone there is no other Wall: no banding"
    );
    finish_combat_sparing(&mut engine, caltrops);
    assert_eq!(
        lk::damage_on(&engine, giant),
        2,
        "it fought as an ordinary 2/1: its two damage reached the Giant"
    );
    assert!(
        in_graveyard(&engine, P1, wall_of_caltrops()).is_some(),
        "and the Giant's three killed it, with no one dividing anything"
    );

    let (control, _) = the_giant_is_blocked_by(&[wall_of_caltrops(), index::WALL_OF_WOOD]);
    let control_caltrops = lk::obj(&control, P1, wall_of_caltrops());
    assert!(
        banded(&control, control_caltrops),
        "control: beside a Wall of Wood, the same Caltrops has banding"
    );
}

/// "… and no non-Wall creatures are blocking that creature". A Grizzly Bears
/// blocking the Giant with the Caltrops and a Wall of Wood spoils it: no
/// banding, and (CR 510.1c) the Giant's controller, not the defender, divides
/// its damage. Likewise a Bears beside a Caltrops that has no other Wall.
/// The control is the same block without the Bears.
#[test]
fn a_non_wall_blocking_the_same_creature_stops_the_banding() {
    let (mut engine, giant) = the_giant_is_blocked_by(&[
        wall_of_caltrops(),
        index::WALL_OF_WOOD,
        index::GRIZZLY_BEARS,
    ]);
    let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
    assert!(
        !banded(&engine, caltrops),
        "a Bear is blocking the Giant too: no banding"
    );
    let (asked, source, _) = first_division(&mut engine);
    assert_eq!(
        source, giant,
        "three blockers: the Giant's damage is divided"
    );
    assert_eq!(
        asked, P0,
        "CR 510.1c: with no banding blocker the attacker's controller divides"
    );

    // A Wall must be the "other" one: a Bear alone beside the Caltrops is no help.
    let (bear_only, _) = the_giant_is_blocked_by(&[wall_of_caltrops(), index::GRIZZLY_BEARS]);
    assert!(
        !banded(&bear_only, lk::obj(&bear_only, P1, wall_of_caltrops())),
        "a non-Wall is not an \"other Wall creature\""
    );

    let (control, _) = the_giant_is_blocked_by(&[wall_of_caltrops(), index::WALL_OF_WOOD]);
    assert!(
        banded(&control, lk::obj(&control, P1, wall_of_caltrops())),
        "control: with only Walls, the Caltrops has banding"
    );
}

/// "… blocking *that creature*": the other Wall counts only when it blocks the
/// same attacker. Caltrops blocks the Giant, a Grizzly Bears blocks the Ogre
/// and the Wall of Wood blocks either one. With the Wood on the Giant the
/// Caltrops has banding (the Bears block a different creature, so they do not
/// spoil it); with the Wood on the Ogre the Caltrops stands alone against the
/// Giant and has none.
#[test]
fn only_blockers_of_the_same_attacker_count() {
    fn caltrops_banded_when(wood_blocks_the_giant: bool) -> bool {
        let mut engine = lk::game(
            SEED,
            &[index::HILL_GIANT, index::GRAY_OGRE],
            &[
                wall_of_caltrops(),
                index::WALL_OF_WOOD,
                index::GRIZZLY_BEARS,
            ],
            &[],
            &[],
        );
        let giant = lk::obj(&engine, P0, index::HILL_GIANT);
        let ogre = lk::obj(&engine, P0, index::GRAY_OGRE);
        let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
        let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        lk::attack(&mut engine, &[giant, ogre]);
        let wood_target = if wood_blocks_the_giant { giant } else { ogre };
        lk::block(
            &mut engine,
            &[(caltrops, giant), (wood, wood_target), (bears, ogre)],
        );
        pass_until(&mut engine, stack_is_empty);
        banded(&engine, caltrops)
    }

    assert!(
        !caltrops_banded_when(false),
        "the Wood blocks the Ogre: no other Wall blocks the Giant, so no banding"
    );
    assert!(
        caltrops_banded_when(true),
        "control: the Wood joins the Caltrops on the Giant, the Bears block the Ogre: banding"
    );
}

/// CR 603.4: the intervening "if" is checked again as the trigger resolves.
/// With the trigger on the stack seat 0 burns the Wall of Wood (a Lightning
/// Bolt: three damage on a 0/3) in response. The condition is false by the
/// time the ability would resolve, so the Caltrops gains nothing. The control
/// plays the same turn without the Bolt and has banding.
#[test]
fn caltrops_checks_its_condition_again_as_the_trigger_resolves() {
    /// Both Walls block the Giant; seat 0 holds priority; with `bolt_the_wood`
    /// it burns the Wood now, whatever is on the stack. Returns the board once
    /// the stack has emptied, the two Walls, and whether anything was waiting
    /// on the stack when seat 0 got priority.
    fn caltrops_after(bolt_the_wood: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId, bool) {
        let mut engine = lk::game(
            SEED,
            &[index::HILL_GIANT],
            &[wall_of_caltrops(), index::WALL_OF_WOOD],
            &[index::LIGHTNING_BOLT],
            &[],
        );
        let giant = lk::obj(&engine, P0, index::HILL_GIANT);
        let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
        let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
        lk::attack(&mut engine, &[giant]);
        lk::block(&mut engine, &[(caltrops, giant), (wood, giant)]);
        assert!(
            matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
            "seat 0 has priority after the blocks: {:?}",
            engine.pending()
        );
        let trigger_waiting = !stack_is_empty(&engine);
        if bolt_the_wood {
            lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
            cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
            lk::aim(&mut engine, &[wood], &[]);
        }
        lk::settle(&mut engine);
        (engine, caltrops, wood, trigger_waiting)
    }

    let (engine, caltrops, wood, trigger_waiting) = caltrops_after(true);
    assert!(
        in_graveyard(&engine, P1, index::WALL_OF_WOOD).is_some(),
        "the Bolt killed the Wood before the trigger resolved ({wood:?})"
    );
    assert!(
        on_battlefield(&engine, P1, wall_of_caltrops()).is_some(),
        "the Caltrops is still blocking"
    );
    assert!(
        !banded(&engine, caltrops),
        "no other Wall is blocking when the trigger resolves: it does nothing"
    );
    assert!(
        trigger_waiting,
        "both Walls block the Giant: the Caltrops' trigger was waiting on the stack to be responded to"
    );

    let (control, caltrops, _, _) = caltrops_after(false);
    assert!(
        banded(&control, caltrops),
        "control: left alone, the trigger gives the Caltrops banding"
    );
}

/// "… gains banding until end of turn." Banding is still there in the end
/// step, after combat, and gone once the turn is over (CR 514.2): in the
/// next turn's upkeep the Caltrops has none.
#[test]
fn the_banding_lasts_until_end_of_turn_and_not_beyond() {
    let (mut engine, _giant) = the_giant_is_blocked_by(&[wall_of_caltrops(), index::WALL_OF_WOOD]);
    let caltrops = lk::obj(&engine, P1, wall_of_caltrops());
    let wood = lk::obj(&engine, P1, index::WALL_OF_WOOD);
    assert!(
        banded(&engine, caltrops),
        "banding once the trigger resolves"
    );
    finish_combat_sparing(&mut engine, caltrops);
    assert!(
        banded(&engine, caltrops),
        "combat is over and it still has banding: the duration is the turn"
    );
    lk::to_step(&mut engine, P0, Step::End);
    assert!(banded(&engine, caltrops), "the end step is still this turn");
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    assert!(
        !banded(&engine, caltrops),
        "seat 1's own turn has begun: the effect ended with the last one"
    );
    assert!(
        !banded(&engine, wood) && on_battlefield(&engine, P1, wall_of_caltrops()).is_some(),
        "the Caltrops itself is untouched by the end of the effect"
    );
}

/// Printed as a Defender: it blocks, it does not attack. Seat 0's Caltrops
/// is not among the attackers it is offered; the Gray Ogre beside it is,
/// which keeps the offer from being empty for some other reason.
#[test]
fn the_caltrops_is_a_defender_and_is_never_offered_as_an_attacker() {
    let mut engine = lk::game(SEED, &[wall_of_caltrops(), index::GRAY_OGRE], &[], &[], &[]);
    let caltrops = lk::obj(&engine, P0, wall_of_caltrops());
    let ogre = lk::obj(&engine, P0, index::GRAY_OGRE);
    let able = lk::attackers_offered(&mut engine);
    assert!(able.contains(&ogre), "the Ogre may attack: {able:?}");
    assert!(
        !able.contains(&caltrops),
        "the Caltrops, a defender, may not: {able:?}"
    );
}
