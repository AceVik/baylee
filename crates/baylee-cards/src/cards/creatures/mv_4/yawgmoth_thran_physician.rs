//! Yawgmoth, Thran Physician — {2}{B}{B} — Legendary Creature — Human Cleric
//! Oracle: Protection from Humans
//! Oracle: Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.
//! Oracle: {B}{B}, Discard a card: Proliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)
//! Set: DMR #110 — Dominaria Remastered | Scryfall ID: b5a79f5d-d0df-4799-ac3a-84305e3af0c9 | Oracle ID: a1e232c0-dc38-47be-a5a0-f68bc1d86a29
// PARTIAL — protection from Humans; Pay 1 life, Sacrifice another creature
// to put a -1/-1 counter on target creature and draw a card; proliferate is
// not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::YAWGMOTH_THRAN_PHYSICIAN,
    oracle_id = "a1e232c0-dc38-47be-a5a0-f68bc1d86a29",
    scryfall_id = "b5a79f5d-d0df-4799-ac3a-84305e3af0c9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "Proliferate has no Effect variant, and the sacrifice activation's \
         target cannot be optional",
    ),
    faces = &[face!(
        name = "Yawgmoth, Thran Physician",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasSubtype(subtypes::creature::HUMAN)),
        ),
        activated!(
            cost!(PayLife(1), Sacrifice(&Filter::ANOTHER_CREATURE_YOU_CONTROL)),
            &[
                Effect::AddCounter {
                    kind: CounterKind::M1M1,
                    amount: Amount::Fixed(1),
                },
                Effect::draw(1),
            ],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
        // NOT SUPPORTED: "{B}{B}, Discard a card: Proliferate." — Proliferate
        // has no Effect variant.
    ],
);
