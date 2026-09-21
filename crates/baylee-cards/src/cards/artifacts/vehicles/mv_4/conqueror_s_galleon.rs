//! Conqueror's Galleon // Conqueror's Foothold — {4} — Artifact — Vehicle // Land
//! Oracle: When this Vehicle attacks, exile it at end of combat, then return it to the battlefield transformed under your control.
//! Oracle: Crew 4 (Tap any number of creatures you control with total power 4 or more: This Vehicle becomes an artifact creature until end of turn.)
//! Oracle: (Transforms from Conqueror's Galleon.)
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Draw a card, then discard a card.
//! Oracle: {4}, {T}: Draw a card.
//! Oracle: {6}, {T}: Return target card from your graveyard to your hand.
//! Set: XLN #234 — Ixalan | Scryfall ID: 02bb4b2a-3c9f-48ab-b2a5-ae31f06b82d9 | Oracle ID: 88b18901-50cd-461c-b1bc-be900210be8e
//! Face: Conqueror's Galleon — {4} — Artifact — Vehicle
//! Face: Conqueror's Foothold —  — Land
// PARTIAL — Conqueror's Foothold's four activated abilities are built; the
// Galleon's two sentences are not expressible, so the card never transforms.
//
// NOT SUPPORTED: "When this Vehicle attacks, exile it at end of combat, then
// return it to the battlefield transformed under your control" — nothing in
// the DSL schedules a transform for the end of combat.
// `Effect::ExileSelfReturnAsFace` transforms on resolution, which would stop
// this being a creature at declare attackers rather than after combat, and
// `Effect::ExileAndReturnAtEndStep` waits for the end step and hands the
// permanent back as the face it exiled.
// NOT SUPPORTED: "Crew 4" — no crew keyword bit, and no cost that taps any
// number of creatures for their total power: `CostPart::TapOther` names
// exactly one permanent. The "becomes an artifact creature until end of turn"
// half alone would be
// `Effect::continuous(&Filter::This, Modifier::AddType(TypeSet::CREATURE),
// Duration::UntilEndOfTurn)`, but the cost it hangs off cannot be said.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BACK_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
    activated!(
        cost!("{2}", TapSelf),
        &[
            Effect::draw(1),
            Effect::DiscardForPlayers {
                who: PlayerRel::You,
                count: 1,
            },
        ]
    ),
    activated!(cost!("{4}", TapSelf), &[Effect::draw(1)]),
    activated!(
        cost!("{6}", TapSelf),
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You),
        }],
        target = Some(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::You))
    ),
];

card!(
    index = index::CONQUEROR_S_GALLEON,
    oracle_id = "88b18901-50cd-461c-b1bc-be900210be8e",
    scryfall_id = "02bb4b2a-3c9f-48ab-b2a5-ae31f06b82d9",
    faces = &[
        face!(
            name = "Conqueror's Galleon",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT,
            subtypes = &[subtypes::artifact::VEHICLE],
            power = Some(2),
            toughness = Some(10),
        ),
        face!(
            name = "Conqueror's Foothold",
            types = TypeSet::LAND,
            castable_from_hand = false,
            abilities = BACK_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "Crew 4 has no cost vocabulary and the transform-on-attack trigger \
         needs an end-of-combat delayed transform; the Foothold's four \
         activated abilities are implemented",
    ),
);
