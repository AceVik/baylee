//! Child of `casting::tests`, using that module's state/me/register helpers.
//! Narrow payment-boundary fixtures: no claim that a Forest has an Un printing
//! or that a real printed spell has the synthetic costs below.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::casting::source_cost::SourceAllowance;
use crate::mana_provenance::ManaProvenance;
use baylee_cards_dsl::{ManaRestriction, ManaUse};
use baylee_core::ids::DamageSourceRef;
use baylee_core::mana::{ManaFlags, RestrictionId};

fn boundary_un_card(state: &mut GameState) -> ObjectId {
    let id = state.zones.list(ZoneLocation::Hand(me()))[0];
    let index = state.object(id).unwrap().card.unwrap().index;
    state.object_mut(id).unwrap().physical_rules =
        Some(std::sync::Arc::new(baylee_core::card_metadata::RulePrint {
            card: index,
            watermarks: vec![None],
            is_un: true,
        }));
    id
}

fn source_white_unit(state: &mut GameState, card: ObjectId, snow: bool) -> RestrictedMana {
    let id = state.next_restriction_id;
    let provenance = ManaProvenance::capture(
        state,
        crate::text_changes::RuleContext {
            source: card,
            text: crate::text_changes::TextChangeMap::IDENTITY,
            resolved_name: None,
            reference: None,
        },
        Some(state.object(card).unwrap().version),
        me(),
        Some(ManaRestriction {
            filter: &Filter::UnCard,
            ability_filter: None,
            rider: SpendRider::None,
            uses: ManaUse::CostsFromMatchingSource,
            restricts: true,
            until_end_of_turn: false,
        }),
    );
    assert!(crate::mana_provenance::deposit(
        state,
        me(),
        ManaColor::White,
        1,
        snow,
        provenance,
    ));
    RestrictedMana {
        color: ManaColor::White,
        amount: 1,
        flags: if snow {
            ManaFlags::SNOW
        } else {
            ManaFlags::NONE
        },
        restriction: RestrictionId(id),
    }
}

#[test]
fn source_receipt_backtracks_when_snow_credit_does_not_satisfy_required_plain_credit() {
    let mut state = state();
    let card = boundary_un_card(&mut state);
    register(&mut state, me(), Modifier::ManaIsAnyColor);
    state.players[0].mana_pool = ManaPool::new();
    state.players[0].mana_pool.add_snow(ManaColor::Blue, 1);
    let required_plain = source_white_unit(&mut state, card, false);
    let _optional_snow = source_white_unit(&mut state, card, true);
    let mut required = ManaPool::new();
    required.add_restricted(required_plain);
    state
        .constrained_payments
        .push(crate::constrained_payment::ConstrainedPayment {
            player: me(),
            card: DamageSourceRef {
                object: card,
                version: state.object(card).unwrap().version,
            },
            required,
        });
    let before = state.players[0].mana_pool.clone();
    let allowance = SourceAllowance {
        constrained: true,
        generic: 0,
    };
    let (after, _) = plan_mana_scoped(
        &state,
        me(),
        SpendFor::Spell(card),
        &ManaCost::parse("{S}{U}"),
        None,
        allowance,
    )
    .expect("snow U pays S; required plain source W pays U under Lattice");
    let receipt = before.payment_receipt(&after).unwrap();
    let snow_spent = receipt.snow_available(ManaColor::Blue)
        + receipt
            .restricted()
            .iter()
            .filter(|mana| mana.flags.contains(ManaFlags::SNOW))
            .map(|mana| u32::from(mana.amount))
            .sum::<u32>();
    assert_eq!(snow_spent, 1);
    assert!(receipt.restricted().contains(&required_plain));
    assert_eq!(after.total(), 1);
    assert_eq!(state.players[0].mana_pool, before, "preview is pure");
    assert!(
        pay_mana_scoped(
            &mut state,
            me(),
            SpendFor::Spell(card),
            &ManaCost::parse("{S}{U}"),
            None,
            allowance,
        )
        .is_some()
    );
    assert_eq!(
        state.players[0].mana_pool, after,
        "actual receipt matches preview"
    );
}

#[test]
fn source_plan_does_not_lose_ordinary_mana_at_merged_counter_limit() {
    let mut state = state();
    let card = boundary_un_card(&mut state);
    state.players[0].mana_pool = ManaPool::new();
    state.players[0].mana_pool.add(ManaColor::White, u32::MAX);
    let credit = source_white_unit(&mut state, card, false);
    let before = state.players[0].mana_pool.clone();
    let cost = ManaCost::from_symbol_generic(u32::MAX);
    let allowance = SourceAllowance {
        constrained: true,
        generic: u32::MAX,
    };
    let planning = spendable_pool(&state, me(), SpendFor::Spell(card));
    let pool = planning.as_ref().unwrap_or(&state.players[0].mana_pool);
    assert!(
        source_plan(
            &state,
            SpendFor::Spell(card),
            pool,
            &cost,
            ManaSpending::default(),
            None,
            allowance,
        )
        .is_some(),
        "ordinary MAX pays ordinary floor; source credit remains separate"
    );
    let (after, _) = plan_mana_scoped(&state, me(), SpendFor::Spell(card), &cost, None, allowance)
        .expect("MAX ordinary units pay the whole MAX-sized cost");
    assert_eq!(after.available(ManaColor::White), 0);
    assert_eq!(after.restricted(), &[credit]);
    assert_eq!(
        before.payment_receipt(&after).unwrap().total(),
        u64::from(u32::MAX)
    );
    assert_eq!(state.players[0].mana_pool, before);
    assert!(
        pay_mana_scoped(
            &mut state,
            me(),
            SpendFor::Spell(card),
            &cost,
            None,
            allowance,
        )
        .is_some()
    );
    assert_eq!(state.players[0].mana_pool, after);
    assert!(state.numeric_failure.is_none());
}
