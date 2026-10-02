//! Resolution payment and event-scoped prevention, independent of printed cards.

use super::*;
use crate::engine::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{AbilityDef, Amount, Effect, PlayerRel, TargetSpec};
use baylee_core::mana::{ManaColor, ManaFlags, ManaPayment, RestrictedMana, RestrictionId};

fn payment_probe() -> Engine<SyntheticLookup> {
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!("", TapSelf),
        &[
            Effect::PayManaToPreventDamage {
                player: PlayerRel::You,
                amount: Amount::Fixed(2)
            },
            Effect::DealDamage {
                amount: Amount::Fixed(3),
                target: TargetSpec::Player(PlayerRel::You)
            },
        ]
    )];
    let definition = synthetic::land(98_010, "Optional payment probe", ABILITIES);
    let preset = synthetic::preset(71, &[98_010]);
    let mut engine = Engine::new(&preset, SyntheticLookup::new(vec![definition])).unwrap();
    synthetic::keep_mulligans(&mut engine);
    walk_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if player.get() == 0),
    );
    engine
}

fn walk_until(
    engine: &mut Engine<SyntheticLookup>,
    ready: impl Fn(&Engine<SyntheticLookup>) -> bool,
) {
    for _ in 0..50 {
        if ready(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            synthetic::walk_past(engine, &pending),
            "unexpected choice: {pending:?}"
        );
    }
    panic!("condition not reached: {:?}", engine.pending());
}

fn open_payment(engine: &mut Engine<SyntheticLookup>) {
    let player = PlayerId::new(0);
    engine.refresh_offer();
    let source = synthetic::permanents(engine, 98_010)[0];
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source,
                ability_index: 0,
            },
        )
        .unwrap();
    walk_until(engine, |e| e.payment_window().is_some());
    assert_eq!(
        engine.payment_window(),
        Some((
            player,
            ManaPayment::AnyAmount {
                preventable_damage: 2
            }
        ))
    );
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

#[test]
fn an_optional_payment_spends_more_than_u16_and_cannot_use_restricted_mana() {
    let mut engine = payment_probe();
    let player = PlayerId::new(0);
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Black, 40_000);
    engine.state.players[0]
        .mana_pool
        .add(ManaColor::Red, 40_000);
    let restricted = RestrictedMana {
        color: ManaColor::Green,
        amount: 7,
        flags: ManaFlags::default(),
        restriction: RestrictionId(1),
    };
    engine.state.players[0].mana_pool.add_restricted(restricted);
    open_payment(&mut engine);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 80_000,
            reason: crate::choice::NumberPrompt::ManaPayment {
                preventable_damage: 2
            },
            ..
        }
    ));
    let before = engine.state.snapshot_hash();
    assert!(
        engine
            .apply(player, PlayerAction::ChooseNumber(80_001))
            .is_err()
    );
    assert_eq!(engine.state.snapshot_hash(), before);
    engine
        .apply(player, PlayerAction::ChooseNumber(70_000))
        .unwrap();
    walk_until(&mut engine, |e| e.state.zones.stack_is_empty());
    assert_eq!(
        engine.state.players[0].mana_pool.total(),
        10_007,
        "the whole chosen amount is paid, with restricted units untouched"
    );
    assert_eq!(
        engine.state.players[0].mana_pool.restricted(),
        &[restricted]
    );
    assert_eq!(
        engine.state.players[0].life, 17,
        "unused prevention cannot shield the next instruction's damage"
    );
    assert!(engine.state.shields.is_empty());
}

#[test]
fn optional_payment_zero_partial_and_excess_only_prevent_the_named_event() {
    for (paid, life) in [(0, 15), (1, 16), (2, 17), (5, 17)] {
        let mut engine = payment_probe();
        engine.state.players[0]
            .mana_pool
            .add(ManaColor::Colorless, 5);
        open_payment(&mut engine);
        engine
            .apply(PlayerId::new(0), PlayerAction::ChooseNumber(paid))
            .unwrap();
        walk_until(&mut engine, |e| e.state.zones.stack_is_empty());
        assert_eq!(engine.state.players[0].life, life);
        assert_eq!(engine.state.players[0].mana_pool.total(), 5 - paid);
    }
}
