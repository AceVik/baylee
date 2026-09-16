//! Mana Vault — {1} — Artifact
//! Oracle: This artifact doesn't untap during your untap step.
//! Oracle: At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact.
//! Oracle: At the beginning of your draw step, if this artifact is tapped, it deals 1 damage to you.
//! Oracle: {T}: Add {C}{C}{C}.
//! Set: 2X2 #308 — Double Masters 2022 | Scryfall ID: c1a31d52-a407-4ded-bfca-cc812f11afa0 | Oracle ID: 736892cb-a34b-4bb9-b56c-e26e3db207a2
// PARTIAL — doesn't untap, taps for {C}{C}{C}, and deals 1 damage to you on draw step if tapped; upkeep {4} untap payment is not in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MANA_VAULT,
    oracle_id = "736892cb-a34b-4bb9-b56c-e26e3db207a2",
    scryfall_id = "c1a31d52-a407-4ded-bfca-cc812f11afa0",
    coverage = Coverage::Partial("upkeep optional mana untap payment is not supported"),
    faces = &[face!(
        name = "Mana Vault",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 3)]),
        // NOT SUPPORTED: At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact.
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Draw,
                whose: PlayerRel::You,
            },
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::Player(PlayerRel::You),
            }],
            condition = Some(Condition::SourceMatches(&Filter::Tapped)),
        ),
    ],
);
