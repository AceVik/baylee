//! Grab the Reins — {3}{R} — Instant
//! Oracle: Choose one —
//! Oracle: • Until end of turn, you gain control of target creature and it gains haste.
//! Oracle: • Sacrifice a creature. Grab the Reins deals damage equal to that creature's power to any target.
//! Oracle: Entwine {2}{R} (Choose both if you pay the entwine cost.)
//! Set: C16 #126 — Commander 2016 | Scryfall ID: d74d181c-6783-4472-a3b9-2f6780e2b89b | Oracle ID: 3ceda405-8175-416a-beda-9a95f5acc60c
use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRAB_THE_REINS,
    oracle_id = "3ceda405-8175-416a-beda-9a95f5acc60c",
    scryfall_id = "d74d181c-6783-4472-a3b9-2f6780e2b89b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Grab the Reins",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::INSTANT,
        entwine = Some(cost!("{2}{R}")),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[
                    Effect::ModifyTargets {
                        modifier: Modifier::GainControl,
                        duration: Duration::UntilEndOfTurn,
                    },
                    Effect::ModifyTargets {
                        modifier: Modifier::AddKeyword(KeywordSet::HASTE),
                        duration: Duration::UntilEndOfTurn,
                    },
                ],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
            mode!(
                &[Effect::SacrificeAndIf {
                    filter: &Filter::CREATURE,
                    check: &Filter::Any,
                    effect: &Effect::DealDamage {
                        amount: Amount::SacrificedPower,
                        target: TargetSpec::AnyTarget,
                    },
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
            ),
        ],
    }],
);
