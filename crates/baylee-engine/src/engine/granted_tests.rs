//! Temporary special actions share payments, but never pay life implicitly.
use super::synthetic::{SyntheticLookup, creature, forest, preset, walk_past};
use super::*;
use baylee_cards_dsl::prelude::*;
use baylee_core::ids::GrantedActionId;
use baylee_core::mana::ManaColor;

const USER: PlayerId = PlayerId::new(0);
const BODY: u32 = 992_000;
static ABILITIES: &[AbilityDef] = &[
    activated!(
        Cost::FREE,
        &[Effect::GrantSpecialActionUntilEndOfTurn {
            timing: SpecialActionTiming::ManaAbility,
            cost: SpecialActionCost::Life(1),
            effect: SpecialActionEffect::AddMana {
                color: ManaColor::Colorless,
                amount: 1
            },
        }]
    ),
    activated!(
        Cost::FREE,
        &[Effect::GrantSpecialActionUntilEndOfTurn {
            timing: SpecialActionTiming::Priority,
            cost: SpecialActionCost::Mana(mana!("{1}")),
            effect: SpecialActionEffect::PreventNextDamage {
                target: TargetSpec::ThisObject,
                amount: Amount::Fixed(1)
            },
        }]
    ),
    activated!(
        cost!("{4}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
            controlled_by: None
        }]
    ),
    mana_ability!(cost!("{2}"), &[Effect::mana(ManaColor::Green, 3)]),
    activated!(
        cost!("{G}"),
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]
    ),
];
fn fixture() -> (Engine<SyntheticLookup>, ObjectId) {
    fixture_seats(2)
}
fn fixture_seats(seats: usize) -> (Engine<SyntheticLookup>, ObjectId) {
    fixture_on(seats, &[BODY])
}
fn fixture_on(seats: usize, battlefield: &[u32]) -> (Engine<SyntheticLookup>, ObjectId) {
    let def = Box::leak(Box::new(CardDef {
        abilities: ABILITIES,
        ..*creature(BODY, "Permission subject", 2, 2, &[])
    }));
    let mut settings = preset(91_116, battlefield);
    while settings.seats.len() < seats {
        settings.seats.push(settings.seats[1].clone());
    }
    let mut engine = Engine::new(&settings, SyntheticLookup::new(vec![def])).unwrap();
    while let Pending::Mulligan { player, .. } = engine.pending().clone() {
        engine.apply(player, PlayerAction::MulliganKeep).unwrap();
    }
    while engine.state.turn.phase != Phase::FirstMain {
        let pending = engine.pending().clone();
        assert!(walk_past(&mut engine, &pending));
    }
    let body = engine
        .state
        .battlefield_seen()
        .find(|id| engine.state.object(*id).unwrap().card.unwrap().index.get() == BODY)
        .unwrap();
    engine.state.players[0].life = 20;
    (engine, body)
}
fn activate(engine: &mut Engine<SyntheticLookup>, source: ObjectId, ability_index: u32) {
    engine
        .apply(
            USER,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
}
fn settle(engine: &mut Engine<SyntheticLookup>) {
    while !engine.state.zones.list(ZoneLocation::Stack).is_empty() {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("unexpected resolution choice")
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
    }
}
fn grant(engine: &mut Engine<SyntheticLookup>, body: ObjectId, index: u32) -> GrantedActionId {
    activate(engine, body, index);
    settle(engine);
    engine.state.granted_actions.last().unwrap().offer.id
}
fn take(engine: &mut Engine<SyntheticLookup>, id: GrantedActionId) {
    engine
        .apply(USER, PlayerAction::TakeGrantedAction { id })
        .unwrap();
}
#[test]
fn granted_nested_activation_payment_keeps_outer_debt_and_explicit_life() {
    let (mut engine, body) = fixture();
    let id = grant(&mut engine, body, 0);
    activate(&mut engine, body, 2);
    assert_eq!(engine.state.players[0].life, 20);
    assert_eq!(
        engine.payment_window().unwrap().1,
        baylee_core::mana::ManaPayment::Fixed(mana!("{4}"))
    );
    activate(&mut engine, body, 3);
    assert_eq!(
        engine.payment_window().unwrap().1,
        baylee_core::mana::ManaPayment::Fixed(mana!("{2}"))
    );
    take(&mut engine, id);
    take(&mut engine, id);
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        engine.payment_window().unwrap().1,
        baylee_core::mana::ManaPayment::Fixed(mana!("{4}"))
    );
    assert_eq!(
        engine.state.players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );
    take(&mut engine, id);
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert!(engine.payment_window().is_none());
    settle(&mut engine);
    assert_eq!(
        engine.state.object(body).unwrap().characteristics().power,
        Some(3)
    );
    assert_eq!(engine.state.players[0].life, 17);
}
#[test]
fn granted_priority_permission_can_pay_using_mana_only_permission() {
    let (mut engine, body) = fixture();
    let mana = grant(&mut engine, body, 0);
    let shield = grant(&mut engine, body, 1);
    take(&mut engine, shield);
    assert!(engine.payment_window().is_some());
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: shield })
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    take(&mut engine, mana);
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.state.players[0].life, 19);
    assert!(!engine.state.shields.is_empty());
}
#[test]
fn granted_colorless_never_substitutes_missing_colored_mana() {
    let (mut engine, body) = fixture();
    let id = grant(&mut engine, body, 0);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!()
    };
    assert!(!legal.abilities.contains(&(body, 4)));
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: body,
                    ability_index: 4
                }
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    take(&mut engine, id);
    assert_eq!(engine.state.players[0].life, 19);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!()
    };
    assert!(!legal.abilities.contains(&(body, 4)));
}
#[test]
fn granted_overflow_wrong_seat_and_forged_identity_refuse_atomically() {
    let (mut engine, body) = fixture();
    let id = grant(&mut engine, body, 0);
    for (player, id) in [
        (PlayerId::new(1), id),
        (USER, GrantedActionId::new(id.get() + 1)),
    ] {
        let before = engine.fingerprint();
        assert!(
            engine
                .apply(player, PlayerAction::TakeGrantedAction { id })
                .is_err()
        );
        assert_eq!(before, engine.fingerprint());
    }
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, u32::MAX);
    engine.refresh_offer();
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id })
            .is_err()
    );
    assert_eq!(before, engine.fingerprint());
    assert_eq!(engine.state.players[0].life, 20);
    assert_eq!(
        engine.state.players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        u32::MAX
    );
}
#[test]
fn granted_replay_and_continuation_hash_include_nested_payment() {
    let (mut first, body) = fixture();
    let (mut second, other_body) = fixture();
    assert_eq!(body, other_body);
    let id = grant(&mut first, body, 0);
    assert_eq!(id, grant(&mut second, body, 0));
    for action in [
        PlayerAction::ActivateAbility {
            source: body,
            ability_index: 2,
        },
        PlayerAction::ActivateAbility {
            source: body,
            ability_index: 3,
        },
        PlayerAction::TakeGrantedAction { id },
    ] {
        first.apply(USER, action.clone()).unwrap();
        second.apply(USER, action).unwrap();
        assert_eq!(first.snapshot_hash(), second.snapshot_hash());
        assert_eq!(first.fingerprint(), second.fingerprint());
    }
    let before = first.snapshot_hash();
    if let PaymentContinuation::Activation(payment) =
        &mut first.mana_window.as_mut().unwrap().suspended
    {
        payment.cost.mana = mana!("{3}");
    }
    assert_ne!(first.snapshot_hash(), before);
}

#[test]
fn activation_payment_hash_only_extends_for_imposed_cost_parts() {
    let (mut engine, body) = fixture();
    grant(&mut engine, body, 0);
    activate(&mut engine, body, 2);
    let before = engine.snapshot_hash();
    {
        let Some(PaymentWindow {
            suspended: PaymentContinuation::Activation(payment),
            ..
        }) = engine.mana_window.as_mut()
        else {
            panic!("activation must open a payment window")
        };
        assert_eq!(payment.cost.imposed, 0);
        assert_eq!(
            crate::state::structural_fingerprint(&payment.cost),
            crate::state::structural_fingerprint(&(
                payment.cost.mana,
                &payment.cost.parts[..],
                payment.cost.allowance,
            )),
            "counter continuations must retain the old Cost and allowance encoding"
        );
        payment.cost.parts.push(CostPart::PayLife(1));
    }
    assert_eq!(
        engine.snapshot_hash(),
        before,
        "the legacy encoding must ignore every printed cost part"
    );
    {
        let Some(PaymentWindow {
            suspended: PaymentContinuation::Activation(payment),
            ..
        }) = engine.mana_window.as_mut()
        else {
            unreachable!()
        };
        let price_before = crate::state::structural_fingerprint(&payment.cost);
        payment.cost.imposed = payment.cost.parts.len();
        assert_ne!(
            crate::state::structural_fingerprint(&payment.cost),
            price_before,
            "counter continuations must distinguish imposed costs"
        );
    }
    assert_ne!(
        engine.snapshot_hash(),
        before,
        "an imposed cost part must join the replay digest"
    );
}

#[test]
fn granted_permission_retains_history_without_making_it_a_damage_source() {
    let (mut engine, body) = fixture();
    let old = engine.state.source_identity(body).unwrap();
    grant(&mut engine, body, 1);
    engine
        .state
        .move_object(
            body,
            ZoneLocation::Hand(USER),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.state.prune_damage_sources();
    assert!(engine.state.source_object(old).is_some());
    assert!(!engine.state.eligible_damage_sources().contains(&old));
    engine.state.granted_actions.clear();
    engine.state.prune_damage_sources();
    assert!(engine.state.source_object(old).is_none());
}

#[test]
fn granted_zero_life_is_checked_after_finishing_a_payment_not_midway() {
    let (mut engine, body) = fixture();
    let id = grant(&mut engine, body, 0);
    engine.state.players[0].life = 4;
    engine.refresh_offer();
    activate(&mut engine, body, 2);
    for _ in 0..4 {
        take(&mut engine, id);
    }
    assert_eq!(engine.state.players[0].life, 0);
    assert!(!engine.state.has_left(USER));
    assert!(engine.payment_window().is_some());
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id })
            .is_err()
    );
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert!(engine.state.has_left(USER));
    assert!(engine.state.granted_actions.is_empty());
}

#[test]
fn granted_concession_discards_permission_and_nested_payment_in_multiplayer() {
    let (mut engine, body) = fixture_seats(3);
    let id = grant(&mut engine, body, 0);
    activate(&mut engine, body, 2);
    activate(&mut engine, body, 3);
    take(&mut engine, id);
    engine.apply(USER, PlayerAction::Concede).unwrap();
    assert!(engine.state.has_left(USER));
    assert!(engine.state.granted_actions.is_empty());
    assert!(engine.payment_window().is_none());
    let Pending::Priority { player, .. } = engine.pending() else {
        panic!("survivor must continue")
    };
    let survivor = *player;
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(survivor, PlayerAction::TakeGrantedAction { id })
            .is_err()
    );
    assert_eq!(before, engine.fingerprint());
    engine.apply(survivor, PlayerAction::PassPriority).unwrap();
}

fn offered_grants(engine: &Engine<SyntheticLookup>) -> Vec<GrantedActionId> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    legal.granted_actions.iter().map(|offer| offer.id).collect()
}

/// Nafs Asp's "pay {1}" at deal 359 of the v6 priority probe, reduced: no
/// land to tap, an activation the seat can press for nothing, one it cannot
/// pay ({4}), and a granted action that costs {1}. Nothing makes the {1}, so
/// the action is not offered. It was, because the offer asked
/// `has_mana_source` while `abilities` still held every activation; the
/// payment window it opened then found nothing to pay with and refused it.
#[test]
fn a_granted_action_nothing_can_pay_for_is_not_offered() {
    let (mut engine, body) = fixture();
    let shield = grant(&mut engine, body, 1);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    assert!(legal.abilities.contains(&(body, 0)), "{legal:?}");
    assert!(
        legal
            .unpaid_abilities
            .iter()
            .any(|&(source, index, _)| (source, index) == (body, 2)),
        "{legal:?}"
    );
    assert!(legal.mana_abilities.is_empty(), "{legal:?}");
    assert!(!offered_grants(&engine).contains(&shield));
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: shield })
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);

    // As it was: offered, and the press refused.
    let (offered, refused) = as_before(|| {
        engine.refresh_offer();
        let offered = offered_grants(&engine).contains(&shield);
        let before = engine.fingerprint();
        let refused = engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: shield })
            .is_err();
        assert_eq!(engine.fingerprint(), before);
        (offered, refused)
    });
    assert!(offered && refused, "offered {offered}, refused {refused}");
    engine.refresh_offer();
    assert!(!offered_grants(&engine).contains(&shield));
}

/// What pays a granted action keeps it on offer: mana in the pool pays it
/// at once, and an untapped land pays it in the window it opens. (A granted
/// mana permission does too: `granted_priority_permission_can_pay_using_
/// mana_only_permission`.)
#[test]
fn a_granted_action_mana_can_pay_for_stays_offered() {
    let (mut engine, body) = fixture();
    let shield = grant(&mut engine, body, 1);
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    engine.refresh_offer();
    assert!(offered_grants(&engine).contains(&shield));
    take(&mut engine, shield);
    assert!(engine.payment_window().is_none());
    assert!(!engine.state.shields.is_empty());

    let (mut engine, body) = fixture_on(2, &[BODY, forest()]);
    let land = engine
        .state
        .battlefield_seen()
        .find(|id| engine.state.object(*id).unwrap().card.unwrap().index.get() == forest())
        .unwrap();
    let shield = grant(&mut engine, body, 1);
    assert!(offered_grants(&engine).contains(&shield));
    take(&mut engine, shield);
    assert!(engine.payment_window().is_some());
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: land })
        .unwrap();
    if engine.payment_window().is_some() {
        engine.apply(USER, PlayerAction::PassPriority).unwrap();
    }
    assert!(engine.payment_window().is_none());
    assert!(!engine.state.shields.is_empty());
}

/// A free cast counts on mana to come only from what a payment window
/// would offer: an untapped land, not an activation that makes none. As it
/// was, the free-cast offers asked the list priority shows, where the
/// seat's free activations read as mana sources.
#[test]
fn a_free_cast_counts_on_mana_only_a_window_could_make() {
    let (engine, _) = fixture();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority")
    };
    assert!(!legal.abilities.is_empty() && legal.mana_abilities.is_empty());
    assert!(!engine.free_cast_makes_mana(USER));
    assert!(as_before(|| engine.free_cast_makes_mana(USER)));
    let (engine, _) = fixture_on(2, &[BODY, forest()]);
    assert!(engine.free_cast_makes_mana(USER));
    assert!(as_before(|| engine.free_cast_makes_mana(USER)));
}

/// Runs `f` with "can this seat make mana?" asked as it was.
fn as_before<T>(f: impl FnOnce() -> T) -> T {
    super::granted::ASK_MANA_AS_BEFORE.with(|before| before.set(true));
    let out = f();
    super::granted::ASK_MANA_AS_BEFORE.with(|before| before.set(false));
    out
}
