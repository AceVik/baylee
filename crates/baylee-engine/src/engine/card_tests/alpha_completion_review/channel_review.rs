//! Independent Channel card behavior: CR 116.2c, 117.1d, 119.4,
//! 601.2g, 605.3a, and cleanup expiration.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::GrantedActionKind;
use baylee_core::generated::index;
use baylee_core::ids::GrantedActionId;
use baylee_core::mana::ManaColor;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == USER),
    );
}

fn setup(extra_board: &[CardIndex], extra_hand: &[CardIndex], life: i32) -> Engine<RegistryLookup> {
    let mut board = vec![forest(), forest()];
    board.extend_from_slice(extra_board);
    let mut hand = vec![index::CHANNEL];
    hand.extend_from_slice(extra_hand);
    let mut engine = Duel::new(SEED, forest())
        .life(0, life)
        .battlefield(0, &board)
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    for source in all_on_battlefield(&engine, USER, forest())
        .into_iter()
        .take(2)
    {
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, USER, index::CHANNEL);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine);
    assert!(in_graveyard(&engine, USER, index::CHANNEL).is_some());
    engine
}

fn offer(engine: &Engine<RegistryLookup>) -> GrantedActionId {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority offer expected")
    };
    legal
        .granted_actions
        .iter()
        .find(|offer| {
            matches!(
                offer.effect,
                GrantedActionKind::AddMana {
                    color: ManaColor::Colorless,
                    amount: 1
                }
            )
        })
        .unwrap()
        .id
}

fn pay(engine: &mut Engine<RegistryLookup>, id: GrantedActionId) {
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    let since = engine.state().journal.last_seq();
    engine
        .apply(USER, PlayerAction::TakeGrantedAction { id })
        .unwrap();
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack), &stack);
    assert!(
        engine
            .state()
            .journal
            .entries()
            .iter()
            .filter(|entry| entry.seq > since)
            .all(|entry| !matches!(
                entry.event,
                crate::event::GameEvent::AbilityTriggered { .. }
                    | crate::event::GameEvent::SpellCast { .. }
            )),
        "Channel is a special action, not casting or activating"
    );
}

#[test]
fn channel_review_repeated_priority_payments_are_immediate_and_expire() {
    let mut engine = setup(&[], &[], 20);
    let action = offer(&engine);
    for expected in 1..=3 {
        pay(&mut engine, action);
        assert_eq!(engine.state().players[0].life, 20 - expected);
        assert_eq!(
            i32::try_from(
                engine.state().players[0]
                    .mana_pool
                    .available(ManaColor::Colorless)
            )
            .unwrap(),
            expected
        );
        assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
        assert_eq!(
            offer(&engine),
            action,
            "the resolved permission is reusable"
        );
    }
    reach_their_main_phase(&mut engine, OTHER);
    priority(&mut engine);
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.granted_actions.is_empty());
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: action })
            .is_err()
    );
    assert_eq!(engine.state().players[0].life, 17);
}

#[test]
fn channel_review_can_pay_during_a_normal_permanent_cast_payment() {
    let mut engine = setup(&[], &[index::JUGGERNAUT], 20);
    let action = offer(&engine);
    cast_with_floating(&mut engine, USER, index::JUGGERNAUT);
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
    for _ in 0..4 {
        pay(&mut engine, action);
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let juggernaut = on_battlefield(&engine, USER, index::JUGGERNAUT).unwrap();
    assert_eq!(pt(&engine, juggernaut), (5, 3));
    assert_eq!(engine.state().players[0].life, 16);
}

#[test]
fn channel_review_can_pay_real_fireball_x_in_the_mana_window() {
    let mut engine = setup(&[mountain()], &[index::FIREBALL], 20);
    let action = offer(&engine);
    let red = on_battlefield(&engine, USER, mountain()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: red })
        .unwrap();
    cast_with_floating(&mut engine, USER, index::FIREBALL);
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(USER, PlayerAction::ChooseNumber(3)).unwrap();
    aim(&mut engine, vec![], vec![OTHER]);
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
    for _ in 0..3 {
        pay(&mut engine, action);
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn channel_review_can_pay_kessig_wolf_runs_x_in_the_mana_window() {
    let mut engine = setup(
        &[
            forest(),
            mountain(),
            index::KESSIG_WOLF_RUN,
            llanowar_elves(),
        ],
        &[],
        20,
    );
    let action = offer(&engine);
    let elf = on_battlefield(&engine, USER, llanowar_elves()).unwrap();
    for card in [forest(), mountain()] {
        let source = all_on_battlefield(&engine, USER, card)
            .into_iter()
            .find(|id| !is_tapped(&engine, *id))
            .unwrap();
        engine
            .apply(USER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    activate(&mut engine, USER, index::KESSIG_WOLF_RUN, 1);
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(USER, PlayerAction::ChooseNumber(3)).unwrap();
    aim(&mut engine, vec![elf], vec![]);
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
    for _ in 0..3 {
        pay(&mut engine, action);
    }
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (4, 1));
    assert!(keywords(&engine, elf).contains(KeywordSet::TRAMPLE));
    assert_eq!(engine.state().players[0].life, 17);
}

#[test]
fn channel_review_cannot_pay_life_after_real_everybody_lives() {
    let mut engine = setup(&[plains(), plains()], &[index::EVERYBODY_LIVES], 20);
    let action = offer(&engine);
    cast_from_hand(&mut engine, USER, index::EVERYBODY_LIVES);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine);
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: action })
            .is_err()
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0
    );
}

#[test]
fn channel_review_at_zero_life_cannot_pay_again_even_when_loss_is_forbidden() {
    let mut engine = setup(&[], &[], 1);
    let action = offer(&engine);
    // Permit the zero-life state without preventing the life payment itself.
    let modifier = baylee_cards_dsl::Modifier::PlayersCantLose;
    let timestamp = engine.state.next_timestamp();
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: USER,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier,
        });
    pay(&mut engine, action);
    assert_eq!(engine.state().players[0].life, 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: action })
            .is_err()
    );
    assert_eq!(engine.state().players[0].life, 0);
}
