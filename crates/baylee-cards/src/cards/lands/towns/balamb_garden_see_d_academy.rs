//! Balamb Garden, SeeD Academy // Balamb Garden, Airborne — (no cost) — Land — Town // Legendary Artifact — Vehicle
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {U}.
//! Oracle: {5}{G}{U}, {T}: Transform this land. This ability costs {1} less to activate for each other Town you control.
//! Oracle: Flying
//! Oracle: Whenever Balamb Garden attacks, draw a card.
//! Oracle: Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)
//! Set: FIN #272 — Final Fantasy | Scryfall ID: 001e9f20-5b15-41cb-bf82-46172decc235 | Oracle ID: 8b84fec5-617c-4088-8250-2ba1f1f9479a
//! Face: Balamb Garden, SeeD Academy —  — Land — Town
//! Face: Balamb Garden, Airborne —  — Legendary Artifact — Vehicle
// PARTIAL — the land enters tapped and taps for {G} or {U}, and the Vehicle
// face carries flying plus its attack trigger. The transform, the per-Town
// cost reduction on it and Crew 1 have no vocabulary (see the NOT SUPPORTED
// comments).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Balamb Garden, Airborne's abilities. Only the front face's transform
/// reaches this face, and that transform is not expressible below.
static BACK_ABILITIES: &[AbilityDef] = &[
    // NOT SUPPORTED: "Crew 1 (Tap any number of creatures you control with
    // total power 1 or more: This Vehicle becomes an artifact creature until
    // end of turn.)" — crew is no keyword bit the engine reads, and no
    // CostPart names "tap any number of creatures with total power N or more".
    triggered!(Trigger::Attacks(&Filter::This), &[Effect::draw(1)]),
];

card!(
    index = index::BALAMB_GARDEN_SEE_D_ACADEMY,
    oracle_id = "8b84fec5-617c-4088-8250-2ba1f1f9479a",
    scryfall_id = "001e9f20-5b15-41cb-bf82-46172decc235",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    faces = &[
        face!(
            name = "Balamb Garden, SeeD Academy",
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::TOWN],
            enter_modifiers = &[EnterModifier::Tapped],
        ),
        face!(
            name = "Balamb Garden, Airborne",
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::artifact::VEHICLE],
            power = Some(5),
            toughness = Some(4),
            keywords = KeywordSet::FLYING,
            abilities = BACK_ABILITIES,
            castable_from_hand = false,
        ),
    ],
    coverage = Coverage::Partial(
        "\"{5}{G}{U}, {T}: Transform this land\" — the DSL has no transform \
         effect, and \"costs {1} less … for each other Town you control\" has \
         no cost-reduction variant; \"Crew 1\" — not a keyword the engine \
         reads, and its cost is no CostPart"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Green, ManaColor::Blue])]),
        // NOT SUPPORTED: "{5}{G}{U}, {T}: Transform this land. This ability
        // costs {1} less to activate for each other Town you control." — a
        // transform is not `Effect::ExileSelfReturnAsFace` (that exiles and
        // returns, which a transform does not do), and neither `Cost` nor
        // `AbilityDef` carries a reduction that counts other permanents.
    ],
);
