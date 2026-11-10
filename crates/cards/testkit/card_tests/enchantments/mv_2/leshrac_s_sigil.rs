//! `cards/enchantments/mv_2/leshrac_s_sigil.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn leshrac_s_sigil() -> CardIndex {
    card_index("98388eba-31ab-4302-be9e-ea67ea2b8c1a")
}

fn opponent_casts_green_with_a_card_left() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[leshrac_s_sigil(), swamp(), swamp()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, grizzly_bears());
    engine
}

#[track_caller]
fn wait_for_black_black(engine: &mut Engine<RegistryLookup>) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    else {
        unreachable!("pass_until stopped at the payment question")
    };
    assert_eq!(*player, P0, "the Sigil's controller chooses whether to pay");
    assert_eq!(*cost, baylee_core::mana!("{B}{B}"));
}

#[track_caller]
fn pay_and_wait_for_hand(engine: &mut Engine<RegistryLookup>) {
    engine.apply(P0, PlayerAction::YesNo(true)).unwrap();
    if matches!(engine.pending(), Pending::Priority { .. }) {
        tap_all_mana(engine, P0);
        engine.apply(P0, PlayerAction::PassPriority).unwrap();
    }
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::LookAtHand,
                ..
            }
        )
    });
}

/// Leshrac's Sigil (98388eba-31ab-4302-be9e-ea67ea2b8c1a) — {B}{B} Enchantment:
/// "{B}{B}: Return this enchantment to its owner's hand."
#[test]
fn leshrac_s_sigil_casts_and_returns_itself_to_owners_hand_for_two_black() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[leshrac_s_sigil()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, leshrac_s_sigil());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, leshrac_s_sigil()).is_some());

    activate(&mut engine, P0, leshrac_s_sigil(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, leshrac_s_sigil()).is_none()
            && in_hand(&engine, P0, leshrac_s_sigil()).is_some(),
        "activating {{B}}{{B}} returns Leshrac's Sigil to its owner's hand"
    );
}

#[test]
fn leshrac_s_sigil_pays_to_inspect_the_casters_hand_and_choose_its_discard() {
    let mut engine = opponent_casts_green_with_a_card_left();
    let discard = in_hand(&engine, P1, unsummon()).unwrap();
    wait_for_black_black(&mut engine);
    pay_and_wait_for_hand(&mut engine);

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped at the private hand choice")
    };
    assert_eq!(player, P0, "the controller, not the hand's owner, chooses");
    assert_eq!((min, max), (1, 1));
    assert_eq!(options.len(), 2, "the turn's draw is also in the hand");
    assert!(options.contains(&discard));
    assert!(on_stack(&engine, grizzly_bears()).is_some());
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![discard]
                }
            )
            .is_err(),
        "the hand's owner cannot make the controller's choice"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P1, unsummon()).is_none());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(P1))
            .contains(&discard),
        "the chosen card is discarded into its owner's graveyard"
    );
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn leshrac_s_sigil_declined_payment_does_not_inspect_or_discard() {
    let mut engine = opponent_casts_green_with_a_card_left();
    wait_for_black_black(&mut engine);
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, P1, unsummon()).is_some());
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn leshrac_s_sigil_ignores_its_controllers_green_spell_and_an_opponents_nongreen_spell() {
    let mut own_green = Duel::new(SEED, forest())
        .battlefield(0, &[leshrac_s_sigil(), forest(), forest()])
        .hand(0, &[grizzly_bears(), unsummon()])
        .start();
    keep_mulligans(&mut own_green);
    reach_main_phase(&mut own_green, P0);
    cast_from_hand(&mut own_green, P0, grizzly_bears());
    pass_until(&mut own_green, stack_is_empty);
    assert!(on_battlefield(&own_green, P0, grizzly_bears()).is_some());
    assert!(in_hand(&own_green, P0, unsummon()).is_some());

    let mut enemy_blue = Duel::new(SEED, forest())
        .battlefield(0, &[leshrac_s_sigil()])
        .battlefield(1, &[grizzly_bears(), island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut enemy_blue);
    reach_their_main_phase(&mut enemy_blue, P1);
    let bears = on_battlefield(&enemy_blue, P1, grizzly_bears()).unwrap();
    cast_from_hand(&mut enemy_blue, P1, unsummon());
    aim_at(&mut enemy_blue, P1, bears);
    pass_until(&mut enemy_blue, stack_is_empty);
    assert!(in_hand(&enemy_blue, P1, grizzly_bears()).is_some());
}

#[test]
fn leshrac_s_sigil_trigger_keeps_the_caster_after_its_source_returns_to_hand() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[leshrac_s_sigil(), swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears(), unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);
    cast_from_hand(&mut engine, P1, grizzly_bears());
    pass_until(&mut engine, |e| {
        on_battlefield(e, P0, leshrac_s_sigil()).is_some_and(|sigil| ability_offered(e, sigil, 1))
    });
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, leshrac_s_sigil(), 1);
    pass_until(&mut engine, |e| in_hand(e, P0, leshrac_s_sigil()).is_some());
    assert!(on_stack(&engine, grizzly_bears()).is_some());

    let discard = in_hand(&engine, P1, unsummon()).unwrap();
    wait_for_black_black(&mut engine);
    pay_and_wait_for_hand(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P1, unsummon()).is_none());
    assert!(on_battlefield(&engine, P1, grizzly_bears()).is_some());
}
