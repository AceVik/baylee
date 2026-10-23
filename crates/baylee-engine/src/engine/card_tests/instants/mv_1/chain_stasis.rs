//! Chain Stasis — `{U}` Instant (Homelands)
//!
//! You may tap or untap target creature. Then that creature's controller may
//! pay `{2}{U}`. If the player does, they may copy this spell and may choose a
//! new target for that copy.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::{TargetPrompt, YesNoPrompt};
use crate::engine::card_tests::legends_kit as kit;
use crate::event::GameEvent;

fn chain_stasis() -> CardIndex {
    card_index("1ec365f8-bfb8-416f-b4db-868a8f5d042a")
}

fn copies(e: &Engine<RegistryLookup>) -> usize {
    e.journal()
        .entries()
        .iter()
        .filter(|entry| matches!(entry.event, GameEvent::SpellCopied { .. }))
        .count()
}

/// Stops at the question "pay `{2}{U}`?" and checks who is asked.
#[track_caller]
fn expect_payment(e: &mut Engine<RegistryLookup>, who: PlayerId) {
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
            if *player == who && *cost == baylee_core::mana!("{2}{U}")),
        "the creature's controller is asked to pay {{2}}{{U}}: {:?}",
        e.pending()
    );
}

/// The spell cast, its target named, and the "may tap or untap" question put
/// to its caster.
#[track_caller]
fn cast_at(e: &mut Engine<RegistryLookup>, target: ObjectId) {
    cast_with_floating(e, kit::P0, chain_stasis());
    kit::aim(e, &[target], &[]);
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == kit::P0),
        "the caster is asked whether to tap or untap: {:?}",
        e.pending()
    );
}

/// Answers whatever is left of the chain: every "may" yes (the first
/// alternative, tapping), every payment declined.
#[track_caller]
fn finish(e: &mut Engine<RegistryLookup>) {
    for _ in 0..80 {
        if stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. }) {
            return;
        }
        match e.pending().clone() {
            Pending::Priority { player, .. } => {
                e.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => e.apply(player, PlayerAction::YesNo(true)).unwrap(),
            Pending::YesNo { player, .. } => e.apply(player, PlayerAction::YesNo(false)).unwrap(),
            Pending::ChooseCastMode { player, .. } => {
                e.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            other => panic!("unexpected Chain Stasis choice: {other:?}"),
        }
    }
    panic!("Chain Stasis did not finish");
}

#[test]
fn chain_stasis_may_tap_or_untap_target_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[chain_stasis(), chain_stasis()])
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert!(!is_tapped(&engine, bears));

    // Cast the first Chain Stasis to tap Grizzly Bears (mode 0), declining to pay.
    tap_all_mana(&mut engine, p0);
    cast_at(&mut engine, bears);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(0)).unwrap();
    expect_payment(&mut engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, bears));

    // Cast the second to untap it (mode 1), declining to pay again.
    let second = in_hand(&engine, p0, chain_stasis()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    kit::aim(&mut engine, &[bears], &[]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    engine.apply(p0, PlayerAction::ChooseMode(1)).unwrap();
    expect_payment(&mut engine, p0);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(!is_tapped(&engine, bears));
}

/// "You may tap or untap": declining leaves the creature as it was, and the
/// creature's controller is still asked about paying.
#[test]
fn declining_to_tap_leaves_the_creature_alone_and_the_payment_is_still_asked() {
    let mut e = kit::game(7001, &[grizzly_bears()], &[], &[chain_stasis()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 1)]);
    let bears = kit::obj(&e, kit::P0, grizzly_bears());
    cast_at(&mut e, bears);
    e.apply(kit::P0, PlayerAction::YesNo(false)).unwrap();
    expect_payment(&mut e, kit::P0);
    e.apply(kit::P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(!kit::tapped(&e, bears), "declined: nothing was tapped");
}

/// The payment and the copy belong to the creature's controller: here the
/// opponent, whose three floating blue pay `{2}{U}`, who copies the spell,
/// aims the copy at the caster's creature, and whose copy then taps it. The
/// caster, the creature's controller for the copy, is asked in turn.
#[test]
fn the_creatures_controller_pays_copies_and_retargets_and_the_copy_asks_its_new_controller() {
    let mut e = kit::game(
        7002,
        &[grizzly_bears()],
        &[gray_ogre()],
        &[chain_stasis()],
        &[],
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Blue, 3)]);
    let ours = kit::obj(&e, kit::P0, grizzly_bears());
    let theirs = kit::obj(&e, kit::P1, gray_ogre());

    cast_at(&mut e, theirs);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(kit::P0, PlayerAction::ChooseMode(0)).unwrap();
    // The opponent owns the creature, so the opponent is the one asked.
    expect_payment(&mut e, kit::P1);
    assert!(
        kit::tapped(&e, theirs),
        "the tap happens before the payment question"
    );
    e.apply(kit::P1, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == kit::P1),
        "after paying, the opponent may copy: {:?}",
        e.pending()
    );
    e.apply(kit::P1, PlayerAction::YesNo(true)).unwrap();
    assert!(
        matches!(e.pending(), Pending::ChooseTargets { player, reason: TargetPrompt::Retarget { .. }, .. } if *player == kit::P1),
        "the copy's controller chooses its new target: {:?}",
        e.pending()
    );
    kit::aim(&mut e, &[ours], &[]);
    assert_eq!(
        kit::pool_total(&e, kit::P1),
        0,
        "{{2}}{{U}} came out of the opponent's pool"
    );
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(stack.len(), 1, "the original has finished; the copy waits");
    assert_eq!(e.state().object(stack[0]).unwrap().controller, kit::P1);
    assert_eq!(copies(&e), 1);
    assert_eq!(
        e.state().per_turn.spells_cast,
        vec![1, 0],
        "copying is not casting"
    );

    // The copy resolves under the opponent's control: they tap our creature,
    // and we are the ones asked about paying for another copy (we decline).
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    assert!(
        matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == kit::P1),
        "the copy's controller decides on the tap: {:?}",
        e.pending()
    );
    finish(&mut e);
    assert_eq!(copies(&e), 1, "the caster declined to copy the copy");
    assert!(
        kit::tapped(&e, theirs),
        "the original tapped the opponent's ogre"
    );
    assert!(kit::tapped(&e, ours), "the copy tapped the caster's bears");
}

/// A creature's controller who says yes but cannot find `{2}{U}` has not paid
/// and gets no copy; their mana is untouched.
#[test]
fn a_controller_who_cannot_pay_gets_no_copy() {
    let mut e = kit::game(7003, &[], &[gray_ogre()], &[chain_stasis()], &[]);
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 1)]);
    kit::float(&mut e, kit::P1, &[(ManaColor::Blue, 2)]);
    let theirs = kit::obj(&e, kit::P1, gray_ogre());

    cast_at(&mut e, theirs);
    e.apply(kit::P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
    });
    e.apply(kit::P0, PlayerAction::ChooseMode(0)).unwrap();
    expect_payment(&mut e, kit::P1);
    e.apply(kit::P1, PlayerAction::YesNo(true)).unwrap();

    // No payment means no "may copy" question: were one asked, the passing
    // helper would answer it yes and a copy would show in the journal.
    pass_until(&mut e, stack_is_empty);
    assert_eq!(copies(&e), 0, "no payment, no copy");
    assert_eq!(
        kit::pool_total(&e, kit::P1),
        2,
        "two blue cannot pay {{2}}{{U}} and none was spent"
    );
    assert!(kit::tapped(&e, theirs));
}
