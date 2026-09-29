//! Ashaya, Soul of the Wild — {3}{G}{G} — Legendary Creature — Elemental
//! Oracle: Ashaya's power and toughness are each equal to the number of lands you control.
//! Oracle: Nontoken creatures you control are Forest lands in addition to their other types. (They're still affected by summoning sickness.)
//! Set: DSC #170 — Duskmourn: House of Horror Commander | Scryfall ID: 0a74b4e6-f6c9-4fef-a83c-a285a541e720 | Oracle ID: 162572f2-1757-42e9-bd97-e6bd9a762c0e
// PARTIAL — on the battlefield both abilities are the card: power and
// toughness are defined by the count at layer 7a (CR 613.4a), and every
// nontoken creature you control is a Forest land in addition to its other
// types. The change is a layer-4 type change, so the Forest type supplies
// "{T}: Add {G}" by CR 305.6 — the type does the work, no ability is
// granted — and the reminder text holds: they are still creatures, so
// summoning sickness still applies to them. What is missing is the
// characteristic-defining ability in every other zone (CR 604.3).

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
         characteristic-defining ability, which works in every zone (CR 604.3); \
         `Modifier::DefinePTByCount` is registered only while Ashaya is on the battlefield, so in \
         a library or a graveyard it is its 0/0 base, which Recruiter of the Guard's toughness \
         and Reveillark's power read"
    ),
    abilities = &[
        // "…are each equal to the number of lands you control": layer 7a,
        // after the type change below, so the creatures it makes into lands
        // count too — Ashaya itself included.
        // NOT SUPPORTED: the same ability off the battlefield (CR 604.3) —
        // statics are registered while their source is on the battlefield.
        static_ability!(Filter::This, Modifier::DefinePTByCount(&Filter::YOUR_LAND)),
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
