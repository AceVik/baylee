//! Archdruid's Charm — {G}{G}{G} — Instant
//! Oracle: Choose one —
//! Oracle: • Search your library for a creature or land card and reveal it. Put it onto the battlefield tapped if it's a land card. Otherwise, put it into your hand. Then shuffle.
//! Oracle: • Put a +1/+1 counter on target creature you control. It deals damage equal to its power to target creature you don't control.
//! Oracle: • Exile target artifact or enchantment.
//! Set: MKM #151 — Murders at Karlov Manor | Scryfall ID: 5caae5ae-845f-42c2-b1ae-956df2739433 | Oracle ID: 3c1ef404-e2c6-486d-a5a2-d5779c71d498
// IMPLEMENTED — three modes: a search whose find forks on the card found
// (a land enters tapped, a creature goes to hand, revealed either way), a
// counter and a bite through the mode's second target, and an exile.

use baylee_cards_dsl::prelude::*;

static CREATURE_OR_LAND: Filter = Filter::Or(&[Filter::CREATURE, Filter::LAND]);
static CREATURE_YOU_DONT_CONTROL: Filter = f!(not_yours CREATURE);

card!(
    index = index::ARCHDRUID_S_CHARM,
    oracle_id = "3c1ef404-e2c6-486d-a5a2-d5779c71d498",
    scryfall_id = "5caae5ae-845f-42c2-b1ae-956df2739433",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Archdruid's Charm",
        mana_cost = mana!("{G}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(&[Effect::SearchLibrary {
                filter: &CREATURE_OR_LAND,
                finds: &[Find::HAND.when_matching(&Filter::LAND, &Find::BATTLEFIELD_TAPPED)],
                optional: false,
            }]),
            mode!(
                &[
                    Effect::AddCounter {
                        kind: CounterKind::P1P1,
                        amount: Amount::Fixed(1),
                    },
                    Effect::DamageEqualToPower {
                        dealer: TargetSlot::First,
                        to: TargetSlot::Second,
                    },
                ],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE))),
                second_targets = Some(TargetReq::one(TargetSpec::Object(
                    &CREATURE_YOU_DONT_CONTROL
                ))),
            ),
            mode!(
                &[Effect::exile(TargetSpec::Object(
                    &Filter::ARTIFACT_OR_ENCHANTMENT
                ))],
                targets = Some(TargetReq::one(TargetSpec::Object(
                    &Filter::ARTIFACT_OR_ENCHANTMENT
                )))
            ),
        ],
    }],
);
