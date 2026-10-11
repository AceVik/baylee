//! Powerstone restrictions apply to spells, not to other payments.
use super::*;
use crate::casting::{SpendFor, pay_mana, pay_mana_for};
use baylee_cards_dsl::{Filter, ManaColor, ManaUse, SpendRider, activated, cost, mana_ability};

const SOURCE: u32 = 7500;
const ARTIFACT: u32 = 7501;
const CREATURE: u32 = 7502;
const USER: u32 = 7503;
static MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)
    .restricted_for(
        &Filter::ARTIFACT,
        SpendRider::None,
        ManaUse::NonSpellPaymentsOrMatchingSpells
    )])];
static USE: &[AbilityDef] = &[activated!(cost!("{1}"), &[Effect::gain_life(1)])];

fn table() -> (Bench, ObjectId) {
    let mut e = bench(
        7500,
        vec![
            super::super::synthetic::land(SOURCE, "Powerstone restriction", MANA),
            card(
                ARTIFACT,
                face("Artifact", "{1}", TypeSet::ARTIFACT),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                CREATURE,
                creature_face("Creature", "{1}", 1, 1),
                KeywordSet::EMPTY,
                &[],
            ),
            card(
                USER,
                creature_face("Mana user", "{0}", 1, 1),
                KeywordSet::EMPTY,
                USE,
            ),
        ],
        [
            Seat::with(&[SOURCE, USER]).holding(&[ARTIFACT, CREATURE]),
            Seat::default(),
        ],
    );
    to_main(&mut e, me());
    let source = the(&e, ZoneLocation::Battlefield, SOURCE);
    activate(&mut e, me(), source, 0);
    (e, source)
}

#[test]
fn powerstone_mana_casts_artifacts_but_never_nonartifact_spells_even_after_its_source_leaves() {
    let (mut e, source) = table();
    e.dev_state_mut(me())
        .unwrap()
        .move_object(
            source,
            ZoneLocation::Graveyard(me()),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    e.refresh_offer();
    let creature = the(&e, ZoneLocation::Hand(me()), CREATURE);
    let artifact = the(&e, ZoneLocation::Hand(me()), ARTIFACT);
    let legal = offer(&e).1;
    assert!(!legal.castable.contains(&creature));
    assert!(legal.castable.contains(&artifact));
    assert!(
        e.apply(me(), PlayerAction::CastSpell { card: creature })
            .is_err()
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
    e.apply(me(), PlayerAction::CastSpell { card: artifact })
        .unwrap();
    settle(&mut e, me());
    assert_eq!(
        e.state().object(artifact).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn powerstone_mana_pays_an_ability_of_a_nonartifact_creature() {
    let (mut e, _) = table();
    let user = the(&e, ZoneLocation::Battlefield, USER);
    let before = life(&e, me());
    activate(&mut e, me(), user, 0);
    settle(&mut e, me());
    assert_eq!(life(&e, me()), before + 1);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn unrelated_payments_spend_only_unreserved_units_without_shrinking_the_obligation() {
    for reserve_restricted in [false, true] {
        let (mut e, _) = table();
        let card = the(&e, ZoneLocation::Hand(me()), ARTIFACT);
        let state = e.dev_state_mut(me()).unwrap();
        let mut required = if reserve_restricted {
            state.players[0].mana_pool.clone()
        } else {
            baylee_core::mana::ManaPool::default()
        };
        if !reserve_restricted {
            required.add(ManaColor::Colorless, 1);
        }
        state.players[0].mana_pool.add(ManaColor::Colorless, 2);
        state
            .constrained_payments
            .push(crate::constrained_payment::ConstrainedPayment {
                player: me(),
                card: state.source_identity(card).unwrap(),
                required: required.clone(),
            });
        // Both a plain and a restricted source can back the remaining unit.
        for remaining in (1..=2).rev() {
            assert_eq!(
                crate::casting::spendable_units(state, me(), SpendFor::Other),
                remaining
            );
            assert!(pay_mana(state, me(), &ManaCost::parse("{1}")));
            assert_eq!(state.constrained_payment(me()).unwrap().required, required);
        }
        assert_eq!(
            crate::casting::spendable_units(state, me(), SpendFor::Other),
            0
        );
        assert!(!pay_mana(state, me(), &ManaCost::parse("{1}")));
        assert_eq!(state.players[0].mana_pool, required);
        assert!(
            pay_mana_for(state, me(), SpendFor::Spell(card), &ManaCost::parse("{1}")).is_some()
        );
        assert_eq!(state.players[0].mana_pool.total(), 0);
        assert_eq!(state.constrained_payment(me()).unwrap().required.total(), 0);
    }
}

#[test]
fn reserved_mana_can_still_pay_an_eligible_land_mana_ability() {
    let (mut e, source) = table();
    let card = the(&e, ZoneLocation::Hand(me()), ARTIFACT);
    let state = e.dev_state_mut(me()).unwrap();
    state
        .constrained_payments
        .push(crate::constrained_payment::ConstrainedPayment {
            player: me(),
            card: state.source_identity(card).unwrap(),
            required: state.players[0].mana_pool.clone(),
        });
    assert!(
        pay_mana_for(
            state,
            me(),
            SpendFor::Ability(source),
            &ManaCost::parse("{1}")
        )
        .is_some()
    );
    assert_eq!(state.players[0].mana_pool.total(), 0);
    assert_eq!(state.constrained_payment(me()).unwrap().required.total(), 0);
}

#[test]
fn powerstone_mana_is_offered_and_paid_for_an_unrelated_tax() {
    let (mut e, _) = table();
    let cost = ManaCost::parse("{1}");
    assert!(e.pool_pays_tax(me(), &cost));
    assert!(pay_mana(e.dev_state_mut(me()).unwrap(), me(), &cost));
    assert!(!e.pool_pays_tax(me(), &cost));
}

#[test]
fn fragmented_reserved_entries_never_duplicate_the_remaining_spendable_mana() {
    for cost in [0, 1, 50, 100, 101] {
        let (mut e, _) = table();
        let card = the(&e, ZoneLocation::Hand(me()), ARTIFACT);
        let state = e.dev_state_mut(me()).unwrap();
        let mut entry = state.players[0].mana_pool.restricted()[0];
        entry.amount = u16::MAX;
        let mut required = baylee_core::mana::ManaPool::default();
        required.add_restricted(entry);
        required.add_restricted(entry);
        state.players[0].mana_pool = required.clone();
        entry.amount = 100;
        state.players[0].mana_pool.add_restricted(entry);
        state
            .constrained_payments
            .push(crate::constrained_payment::ConstrainedPayment {
                player: me(),
                card: state.source_identity(card).unwrap(),
                required: required.clone(),
            });
        assert_eq!(
            crate::casting::spendable_units(state, me(), SpendFor::Other),
            100
        );
        assert_eq!(
            pay_mana(state, me(), &ManaCost::from_symbol_generic(cost)),
            cost <= 100
        );
        assert_eq!(state.constrained_payment(me()).unwrap().required, required);
        assert_eq!(
            state.players[0].mana_pool.total(),
            131_170 - u64::from(if cost <= 100 { cost } else { 0 })
        );
    }
}
