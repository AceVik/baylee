//! Veiled Serpent — {2}{U} — Enchantment
//! Oracle: When an opponent casts a spell, if this permanent is an enchantment, it becomes a 4/4 Serpent creature with "This creature can't attack unless defending player controls an Island."
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: USG #110 — Urza's Saga | Scryfall ID: 25193fd6-3156-48ff-90fa-71328ee7adf5 | Oracle ID: ff59a95c-28de-44cf-abbe-772417851ffa
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::ISLAND)]);

card!(
    index = index::VEILED_SERPENT,
    oracle_id = "ff59a95c-28de-44cf-abbe-772417851ffa",
    scryfall_id = "25193fd6-3156-48ff-90fa-71328ee7adf5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Veiled Serpent",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::SpellCast(&Filter::ControlledByOpponent),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::BecomeType {
                        types: TypeSet::CREATURE,
                        subtype: subtypes::creature::SERPENT,
                    },
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely),
                Effect::continuous(
                    &Filter::This,
                    Modifier::CantAttackUnlessDefenderControls(&ISLAND),
                    Duration::Indefinitely,
                ),
            ],
            condition = Some(Condition::SourceMatches(&Filter::ENCHANTMENT)),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
