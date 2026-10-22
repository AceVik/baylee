//! Hakim, Loreweaver — {3}{U}{U} — Legendary Creature — Human Wizard
//! Oracle: Flying
//! Oracle: {U}{U}: Return target Aura card from your graveyard to the battlefield attached to Hakim. Activate only during your upkeep and only if Hakim isn't enchanted.
//! Oracle: {U}{U}, {T}: Destroy all Auras attached to Hakim.
//! Set: MIR #68 — Mirage | Scryfall ID: 8192bca7-03e5-4ea1-ae77-8bc811c19417 | Oracle ID: 43e99cba-a6e3-4abc-a92b-aaf5839c72d2
// NOT SUPPORTED: returning the target Aura card directly to the battlefield attached to Hakim; returns it to your hand instead.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static AURA_CARD: Filter = Filter::HasSubtype(subtypes::enchantment::AURA);
static AURA_ATTACHED_TO_SOURCE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedToSource,
]);
static UPKEEP_AND_UNENCHANTED: [Condition; 3] = [
    Condition::YourTurn,
    Condition::DuringStep(StepKind::Upkeep),
    Condition::Not(&Condition::SourceMatches(&Filter::Enchanted)),
];

card!(
    index = index::HAKIM_LOREWEAVER,
    oracle_id = "43e99cba-a6e3-4abc-a92b-aaf5839c72d2",
    scryfall_id = "8192bca7-03e5-4ea1-ae77-8bc811c19417",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "has flying, returns target Aura card from your graveyard to your hand during your upkeep if Hakim isn't enchanted, and destroys all Auras attached to Hakim for {U}{U}, {T}; returning the Aura directly to the battlefield attached to Hakim is not in Effect",
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
        activated!(
            cost!("{U}{U}"),
            &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&AURA_CARD, PlayerRel::You),
            }],
            target = Some(TargetSpec::CardInGraveyard(&AURA_CARD, PlayerRel::You)),
            condition = Some(Condition::All(&UPKEEP_AND_UNENCHANTED)),
        ),
        activated!(
            cost!("{U}{U}", TapSelf),
            &[Effect::DestroyAll {
                filter: &AURA_ATTACHED_TO_SOURCE,
                no_regen: false,
            }],
        ),
    ],
);
