//! Icy Prison — {U}{U} — Enchantment
//! Oracle: When this enchantment enters, exile target creature.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless any player pays {3}.
//! Oracle: When this enchantment leaves the battlefield, return the exiled card to the battlefield under its owner's control.
//! Set: ME2 #50 — Masters Edition II | Scryfall ID: 5f3a535e-2efa-4893-a9ee-f42caf482e5f | Oracle ID: d4e2cc1a-4212-4284-9ebe-d3c40931de50
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "unless any player pays {3}" — modeled as controller may pay {3} or sacrifice.
card!(
    index = index::ICY_PRISON,
    oracle_id = "d4e2cc1a-4212-4284-9ebe-d3c40931de50",
    scryfall_id = "5f3a535e-2efa-4893-a9ee-f42caf482e5f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage =
        Coverage::Partial("upkeep mana payment offered to controller rather than any player"),
    faces = &[face!(
        name = "Icy Prison",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE))],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::You,
                mana: Amount::Fixed(3),
                effect: &Effect::SacrificeSelf,
            }]
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::ReturnLinkedToBattlefield]
        ),
    ],
);
