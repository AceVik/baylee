//! Independent Power Leak behavior; registered after implementation.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;

fn power_leak() -> CardIndex {
    card_index("dc2f0000-870b-487f-9623-618fc8eb9765")
}

fn leak_host() -> CardIndex {
    card_index("a2310312-6e1e-4e34-a351-9aef499a810f")
}

fn attach_leak(engine: &mut Engine<RegistryLookup>) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    reach_main_phase(engine, p0);
    let host = on_battlefield(engine, p1, leak_host()).unwrap();
    cast_from_hand(engine, p0, power_leak());
    aim(engine, vec![host], vec![]);
    pass_until(engine, stack_is_empty);
}

fn leak_walk<L: CardLookup>(engine: &mut Engine<L>, pred: impl Fn(&Engine<L>) -> bool) {
    for _ in 0..100 {
        if pred(engine) {
            return;
        }
        let pending = engine.pending().clone();
        assert!(
            super::super::super::synthetic::walk_past(engine, &pending),
            "unexpected pending: {pending:?}"
        );
    }
    panic!("Power Leak scenario did not reach its decision");
}

fn leak_payment_window<L: CardLookup>(engine: &mut Engine<L>, payer: PlayerId) {
    leak_walk(engine, |e| e.payment_window().is_some());
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == payer));
    assert_eq!(engine.state().turn.active, payer);
}

fn pay_leak<L: CardLookup>(engine: &mut Engine<L>, payer: PlayerId, amount: u32) {
    engine.apply(payer, PlayerAction::PassPriority).unwrap();
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "Power Leak offers an amount after making mana: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, payer);
    assert_eq!(min, 0);
    assert!(
        amount <= max,
        "any amount actually available must be offered"
    );
    engine
        .apply(payer, PlayerAction::ChooseNumber(amount))
        .unwrap();
    leak_walk(engine, |e| e.state().zones.stack_is_empty());
}

#[test]
fn power_leak_independent_empty_pool_opens_mana_window_and_accepts_more_than_two() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[power_leak()])
        .battlefield(1, &[leak_host(), mountain(), mountain(), mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    attach_leak(&mut engine);
    leak_payment_window(&mut engine, p1);
    assert_eq!(engine.state().players[1].mana_pool.total(), 0);
    assert_eq!(tap_all_mana(&mut engine, p1), 3);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("making mana must keep the payment window open");
    };
    assert!(
        legal.castable.is_empty(),
        "even a now-affordable Bolt cannot be cast during resolution"
    );
    pay_leak(&mut engine, p1, 3);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        0,
        "all three mana are actually paid"
    );
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[0].life, 20);
    assert!(on_battlefield(&engine, p0, power_leak()).is_some());
}

#[test]
fn power_leak_independent_zero_one_and_two_payments_prevent_only_paid_damage() {
    let p1 = PlayerId::new(1);
    for amount in 0..=2 {
        let mut engine = Duel::new(SEED, island())
            .battlefield(0, &[island(), island()])
            .hand(0, &[power_leak()])
            .battlefield(1, &[leak_host(), mountain(), mountain(), mountain()])
            .start();
        keep_mulligans(&mut engine);
        attach_leak(&mut engine);
        leak_payment_window(&mut engine, p1);
        tap_all_mana(&mut engine, p1);
        pay_leak(&mut engine, p1, amount);
        assert_eq!(engine.state().players[1].mana_pool.total(), 3 - amount);
        assert_eq!(
            engine.state().players[1].life,
            18 + i32::try_from(amount).unwrap()
        );
        assert_eq!(engine.state().players[0].life, 20);
    }
}

#[test]
fn power_leak_independent_excess_payment_does_not_shield_later_damage_or_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), mountain()])
        .hand(0, &[power_leak(), lightning_bolt()])
        .battlefield(1, &[leak_host(), mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    attach_leak(&mut engine);
    leak_payment_window(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    pay_leak(&mut engine, p1, 3);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim(&mut engine, vec![], vec![p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "unused prevention is not a general shield"
    );
    leak_payment_window(&mut engine, p1);
    pay_leak(&mut engine, p1, 0);
    assert_eq!(
        engine.state().players[1].life,
        15,
        "last upkeep's overpayment cannot prevent this event"
    );
}

#[test]
fn power_leak_independent_source_or_attachment_removed_preserves_trigger_player_and_payment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for bounce_aura in [false, true] {
        let mut engine = Duel::new(SEED, island())
            .battlefield(0, &[island(), island()])
            .hand(0, &[power_leak()])
            .battlefield(
                1,
                &[
                    leak_host(),
                    island(),
                    island(),
                    island(),
                    island(),
                    island(),
                ],
            )
            .hand(1, &[boomerang()])
            .start();
        keep_mulligans(&mut engine);
        attach_leak(&mut engine);
        let host = on_battlefield(&engine, p1, leak_host()).unwrap();
        let aura = on_battlefield(&engine, p0, power_leak()).unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && !e.state().zones.stack_is_empty()
        });
        assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1));
        cast_from_hand(&mut engine, p1, boomerang());
        aim(
            &mut engine,
            vec![if bounce_aura { aura } else { host }],
            vec![],
        );
        leak_payment_window(&mut engine, p1);
        if bounce_aura {
            assert!(in_hand(&engine, p0, power_leak()).is_some());
        } else {
            assert!(in_hand(&engine, p1, leak_host()).is_some());
            assert!(in_graveyard(&engine, p0, power_leak()).is_some());
        }
        pay_leak(&mut engine, p1, 1);
        assert_eq!(
            engine.state().players[1].life,
            19,
            "the removed Aura's trigger still deals damage and permits payment"
        );
        assert_eq!(engine.state().players[0].life, 20);
    }
}

use super::super::super::synthetic::{self, SyntheticLookup};
use baylee_cards_dsl::{
    AbilityDef, Cost, Duration, Effect, Filter, Modifier, TargetSpec, activated, static_ability,
};

const LEAK_GUARD: u32 = 4_000_030;
const LEAK_THIEF: u32 = 4_000_031;

#[test]
fn power_leak_independent_creature_only_mana_cannot_pay_its_resolution_payment() {
    let p1 = PlayerId::new(1);
    let ziggurat = card_index("0baabe39-72ae-47bd-a095-cbf7eb8a6361");
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[power_leak()])
        .battlefield(1, &[leak_host(), ziggurat])
        .start();
    keep_mulligans(&mut engine);
    attach_leak(&mut engine);
    leak_payment_window(&mut engine, p1);
    activate(&mut engine, p1, ziggurat, 0);
    engine
        .apply(p1, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseNumber { min: 0, max: 0, .. }
        ),
        "mana restricted to creature spells is unavailable to this payment"
    );
    assert!(engine.apply(p1, PlayerAction::ChooseNumber(1)).is_err());
    engine.apply(p1, PlayerAction::ChooseNumber(0)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[1].mana_pool.total(), 1);
}

static LEAK_GUARD_ABILITIES: &[AbilityDef] = &[
    static_ability!(
        Filter::This,
        Modifier::RedirectDamageToYou(&Filter::ENCHANTMENT)
    ),
    // A prohibition on combat prevention must not suppress this noncombat
    // prevention, even when it applies to the real Aura source.
    static_ability!(Filter::ENCHANTMENT, Modifier::CombatDamageCantBePrevented),
];
static LEAK_THIEF_ABILITIES: &[AbilityDef] = &[activated!(
    Cost::FREE,
    &[Effect::continuous(
        &Filter::This,
        Modifier::GainControl,
        Duration::Indefinitely
    )],
    target = Some(TargetSpec::Object(&Filter::ENCHANTMENT))
)];

fn leak_fixture(
    index: u32,
    abilities: &'static [AbilityDef],
) -> &'static baylee_cards_dsl::CardDef {
    Box::leak(Box::new(baylee_cards_dsl::CardDef {
        abilities,
        ..*synthetic::creature(index, "Power Leak interaction fixture", 0, 5, &[])
    }))
}

fn synthetic_leak(index: u32, abilities: &'static [AbilityDef]) -> Engine<SyntheticLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let fixture_seat0 = if index == LEAK_THIEF {
        vec![island().get(), island().get(), index]
    } else {
        vec![island().get(), island().get()]
    };
    let mut fixture_seat1 = vec![
        leak_host().get(),
        mountain().get(),
        mountain().get(),
        mountain().get(),
    ];
    if index == LEAK_GUARD {
        fixture_seat1.push(index);
    }
    let mut preset = synthetic::preset_both(SEED, &fixture_seat0, &fixture_seat1);
    preset.seats[0].starting_hand = Some(vec![baylee_core::preset::DeckEntry {
        card: power_leak(),
        print: baylee_core::ids::PrintRef::new(0),
    }]);
    let mut engine = Engine::new(
        &preset,
        SyntheticLookup::new(vec![leak_fixture(index, abilities)]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut engine);
    leak_walk(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    for source in synthetic::permanents(&engine, island().get()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let host = synthetic::permanents(&engine, leak_host().get())[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![host],
                players: vec![],
            },
        )
        .unwrap();
    leak_walk(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(engine.state().object(host).unwrap().controller, p1);
    engine
}

#[test]
fn power_leak_independent_paid_prevention_tracks_its_damage_through_redirection() {
    let p1 = PlayerId::new(1);
    for paid in 0..=2 {
        let mut engine = synthetic_leak(LEAK_GUARD, LEAK_GUARD_ABILITIES);
        let guard = synthetic::permanents(&engine, LEAK_GUARD)[0];
        leak_payment_window(&mut engine, p1);
        synthetic::tap_every_land(&mut engine, p1);
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
        engine.apply(p1, PlayerAction::ChooseNumber(paid)).unwrap();
        if paid > 0 {
            let Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } = engine.pending().clone()
            else {
                panic!("the player chooses between paid prevention and redirection");
            };
            assert_eq!(player, p1);
            let effect = options
                .iter()
                .find(|option| {
                    matches!(
                        option.kind,
                        crate::choice::DamageEffectKind::Redirect { .. }
                    )
                })
                .unwrap()
                .id;
            engine
                .apply(player, PlayerAction::ChooseDamageEffect { choice, effect })
                .unwrap();
        }
        leak_walk(&mut engine, |e| e.state().zones.stack_is_empty());
        // CR 614.9 calls redirection the same damage to another recipient.
        // "That damage" remains the Aura's event after its recipient changes.
        assert_eq!(
            u32::from(engine.state().object(guard).unwrap().damage),
            2 - paid
        );
        assert_eq!(engine.state().players[1].life, 20);
        assert_eq!(engine.state().players[1].mana_pool.total(), 3 - paid);
    }
}

#[test]
fn power_leak_independent_host_control_change_in_response_keeps_triggered_upkeep_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = synthetic_leak(LEAK_THIEF, LEAK_THIEF_ABILITIES);
    let thief = synthetic::permanents(&engine, LEAK_THIEF)[0];
    let host = synthetic::permanents(&engine, leak_host().get())[0];
    leak_walk(&mut engine, |e| {
        e.state().turn.active == p1 && !e.state().zones.stack_is_empty()
    });
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: thief,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![host],
                players: vec![],
            },
        )
        .unwrap();
    leak_payment_window(&mut engine, p1);
    assert_eq!(engine.state().object(host).unwrap().controller, p0);
    synthetic::tap_every_land(&mut engine, p1);
    pay_leak(&mut engine, p1, 1);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "that player is the one whose upkeep triggered the Aura"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

/// CR 616.1 permits the affected player to choose among applicable prevention
/// effects. With one paid Power Leak prevention and Reverse Damage's shield,
/// Leak-first yields one life gained; Reverse-first yields two. These
/// choices are also covered by `damage_order_review`'s real-card order cases.
#[test]
fn power_leak_independent_reverse_damage_requires_a_prevention_order_choice() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let reverse = card_index("eaaf7c30-f463-4115-a40e-7dc717063413");
    for paid_first in [true, false] {
        let mut engine = Duel::new(SEED, island())
            .battlefield(0, &[island(), island()])
            .hand(0, &[power_leak()])
            .battlefield(1, &[leak_host(), plains(), plains(), plains(), plains()])
            .hand(1, &[reverse])
            .start();
        keep_mulligans(&mut engine);
        attach_leak(&mut engine);
        let aura = on_battlefield(&engine, p0, power_leak()).unwrap();
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && !e.state().zones.stack_is_empty()
        });
        cast_from_hand(&mut engine, p1, reverse);
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseDamageSource { .. })
        });
        let Pending::ChooseDamageSource {
            choice, options, ..
        } = engine.pending().clone()
        else {
            unreachable!("source prompt")
        };
        let selected = baylee_core::ids::DamageSourceRef {
            object: aura,
            version: engine.state().object(aura).expect("source exists").version,
        };
        assert!(options.contains(&selected));
        engine
            .apply(
                p1,
                PlayerAction::ChooseDamageSource {
                    choice,
                    source: selected,
                },
            )
            .unwrap();
        leak_payment_window(&mut engine, p1);
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
        engine.apply(p1, PlayerAction::ChooseNumber(1)).unwrap();
        assert_eq!(
            engine.state().players[1].life,
            20,
            "the player must choose prevention order before lifegain is decided"
        );
        let Pending::ChooseDamageEffect {
            player,
            choice,
            options,
            ..
        } = engine.pending().clone()
        else {
            panic!("a meaningful replacement/prevention decision remains");
        };
        assert_eq!(player, p1);
        let effect = options
            .iter()
            .find(|option| {
                if paid_first {
                    matches!(
                        option.kind,
                        crate::choice::DamageEffectKind::PreventThisEvent { .. }
                    )
                } else {
                    matches!(
                        option.kind,
                        crate::choice::DamageEffectKind::PreventFromSource {
                            gain_life: true,
                            ..
                        }
                    )
                }
            })
            .unwrap()
            .id;
        engine
            .apply(player, PlayerAction::ChooseDamageEffect { choice, effect })
            .unwrap();
        pass_until(&mut engine, |e| e.state().zones.stack_is_empty());
        assert_eq!(
            engine.state().players[1].life,
            if paid_first { 21 } else { 22 }
        );
    }
}
