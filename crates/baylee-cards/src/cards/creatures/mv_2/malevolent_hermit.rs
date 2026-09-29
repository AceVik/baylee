//! Malevolent Hermit // Benevolent Geist — {1}{U} — Creature — Human Wizard // Creature — Spirit Wizard
//! Oracle: {U}, Sacrifice this creature: Counter target noncreature spell unless its controller pays {3}.
//! Oracle: Disturb {2}{U} (You may cast this card from your graveyard transformed for its disturb cost.)
//! Oracle: Flying
//! Oracle: Noncreature spells you control can't be countered.
//! Oracle: If Benevolent Geist would be put into a graveyard from anywhere, exile it instead.
//! Set: MID #61 — Innistrad: Midnight Hunt | Scryfall ID: e79269af-63eb-43d2-afee-c38fa14a0c5b | Oracle ID: 51233ade-70cd-4539-9f41-5ffab761da54
//! Face: Malevolent Hermit — {1}{U} — Creature — Human Wizard
//! Face: Benevolent Geist —  — Creature — Spirit Wizard
// IMPLEMENTED — the {U}, sacrifice soft counter (`PlayerMayPayOr` on the
// targeted spell's controller); disturb for {2}{U}; Benevolent Geist's flying,
// its shield over your noncreature spells, and its exile instead of any
// graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BENEVOLENT_GEIST: &[AbilityDef] = &[
    // "Noncreature spells you control can't be countered." A filter that
    // reaches the stack has every object projected while it stands, your
    // abilities there included. A spell always has a card type and an
    // ability has none (its face is blank), so asking for a noncreature
    // card type keeps the shield off abilities, which ward or a
    // counter-target-ability effect may still counter.
    static_ability!(
        Filter::And(&[
            Filter::ControlledByYou,
            Filter::InZone(ZoneRef::Stack),
            Filter::HasType(
                TypeSet::ARTIFACT
                    .union(TypeSet::ENCHANTMENT)
                    .union(TypeSet::INSTANT)
                    .union(TypeSet::SORCERY)
                    .union(TypeSet::PLANESWALKER)
                    .union(TypeSet::BATTLE)
                    .union(TypeSet::KINDRED)
            ),
            Filter::NONCREATURE,
        ]),
        Modifier::AddKeyword(KeywordSet::UNCOUNTERABLE)
    ),
    // "If Benevolent Geist would be put into a graveyard from anywhere,
    // exile it instead."
    AbilityDef::Replacement(ReplacementRule::ExileSelfInsteadOfGraveyard),
];

card!(
    index = index::MALEVOLENT_HERMIT,
    oracle_id = "51233ade-70cd-4539-9f41-5ffab761da54",
    scryfall_id = "e79269af-63eb-43d2-afee-c38fa14a0c5b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Malevolent Hermit",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
            power = Some(2),
            toughness = Some(1),
        ),
        // "Disturb {2}{U}": the back face's `mana_cost` is the disturb cost.
        face!(
            name = "Benevolent Geist",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::SPIRIT, subtypes::creature::WIZARD],
            power = Some(2),
            toughness = Some(2),
            castable_from_hand = false,
            disturb = true,
            keywords = KeywordSet::FLYING,
            abilities = BENEVOLENT_GEIST,
        ),
    ],
    abilities = &[activated!(
        cost!("{U}", SacrificeSelf),
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(3),
            effect: &Effect::CounterTargetSpell,
        }],
        target = Some(TargetSpec::Spell(&Filter::NONCREATURE)),
    ),],
);
