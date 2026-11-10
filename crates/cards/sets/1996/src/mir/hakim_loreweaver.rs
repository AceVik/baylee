//! Hakim, Loreweaver — {3}{U}{U} — Legendary Creature — Human Wizard
//! Oracle: Flying
//! Oracle: {U}{U}: Return target Aura card from your graveyard to the battlefield attached to Hakim. Activate only during your upkeep and only if Hakim isn't enchanted.
//! Oracle: {U}{U}, {T}: Destroy all Auras attached to Hakim.
//! Set: MIR #68 — Mirage | Scryfall ID: 8192bca7-03e5-4ea1-ae77-8bc811c19417 | Oracle ID: 43e99cba-a6e3-4abc-a92b-aaf5839c72d2
// NOT SUPPORTED: "{U}{U}: Return target Aura card from your graveyard to the battlefield attached to Hakim" — no effect returns a graveyard card to the battlefield attached to the source; returning it to your hand instead is not printed, so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static AURA_ATTACHED_TO_SOURCE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedToSource,
]);

card!(
    index = index::HAKIM_LOREWEAVER,
    oracle_id = "43e99cba-a6e3-4abc-a92b-aaf5839c72d2",
    scryfall_id = "8192bca7-03e5-4ea1-ae77-8bc811c19417",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "has flying and destroys all Auras attached to Hakim for {U}{U}, {T}; returning an Aura card from your graveyard to the battlefield attached to Hakim during your upkeep is left off because no effect attaches a returned card to the source (returning it to your hand is not printed)",
    ),
    faces = &[face!(
        name = "Hakim, Loreweaver",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(4),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[
        // NOT SUPPORTED: {U}{U}: Return target Aura card from your graveyard to the battlefield attached to Hakim. Activate only during your upkeep and only if Hakim isn't enchanted.
        activated!(
            cost!("{U}{U}", TapSelf),
            &[Effect::DestroyAll {
                filter: &AURA_ATTACHED_TO_SOURCE,
                no_regen: false,
            }],
        ),
    ],
);
