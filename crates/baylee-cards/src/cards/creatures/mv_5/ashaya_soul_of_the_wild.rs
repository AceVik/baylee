//! Ashaya, Soul of the Wild — {3}{G}{G} — Legendary Creature — Elemental
//! Oracle: Ashaya's power and toughness are each equal to the number of lands you control.
//! Oracle: Nontoken creatures you control are Forest lands in addition to their other types. (They're still affected by summoning sickness.)
//! Set: DSC #170 — Duskmourn: House of Horror Commander | Scryfall ID: 0a74b4e6-f6c9-4fef-a83c-a285a541e720 | Oracle ID: 162572f2-1757-42e9-bd97-e6bd9a762c0e
// PARTIAL — a +1/+1-per-land body on top of the printed 0/0, and every
// nontoken creature you control is a Forest land in addition to its other
// types. The change is a layer-4 type change, so the Forest type supplies
// "{T}: Add {G}" by CR 305.6 — the type does the work, no ability is
// granted — and the reminder text holds: they are still creatures, so
// summoning sickness still applies to them.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Nontoken creatures you control" — the group both halves of the second
/// ability name, so it is written once instead of twice (and the adjective
/// order is a load-bearing part of the filter, not a style).
static NONTOKEN_CREATURES_YOU_CONTROL: Filter = f!(your nontoken CREATURE);

card!(
    index = index::ASHAYA_SOUL_OF_THE_WILD,
    oracle_id = "162572f2-1757-42e9-bd97-e6bd9a762c0e",
    scryfall_id = "0a74b4e6-f6c9-4fef-a83c-a285a541e720",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ashaya, Soul of the Wild",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(0),
        toughness = Some(0),
    ),],
    coverage = Coverage::Partial(
        "\"Ashaya's power and toughness are each equal to the number of lands you control\" is a \
         layer-7a characteristic-defining ability, which no `Modifier` sets; it is spelled as the \
         layer-7c `Modifier::ModifyPTPerCount` over the 0/0 printed body, which is the same number \
         until a layer-7b effect sets power and toughness, and different after one"
    ),
    abilities = &[
        // "…are each equal to the number of lands you control": the printed
        // body is 0/0, so one +1/+1 per land is that number. Read at layer 7c,
        // after the type change below, so the creatures it makes into lands
        // count too — Ashaya itself included.
        static_ability!(
            Filter::This,
            Modifier::ModifyPTPerCount {
                filter: &Filter::YOUR_LAND,
                p: 1,
                t: 1,
            }
        ),
        // "…are Forest lands": the land type and the Forest land type are two
        // modifiers, and both are additive, so the creature keeps its types.
        static_ability!(
            NONTOKEN_CREATURES_YOU_CONTROL,
            Modifier::AddType(TypeSet::LAND)
        ),
        static_ability!(
            NONTOKEN_CREATURES_YOU_CONTROL,
            Modifier::AddSubtype(subtypes::land::FOREST)
        ),
    ],
);
