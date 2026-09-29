//! Boseiju, Who Endures — (no cost) — Legendary Land
//! Oracle: {T}: Add {G}.
//! Oracle: Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #266 — Kamigawa: Neon Dynasty | Scryfall ID: 2135ac5a-187b-4dc9-8f82-34e8d1603416 | Oracle ID: bf1341dd-41a3-49f6-87ec-63170dde4324
// PARTIAL — {T}: Add {G}, plus the channel ability as an activation from hand
// that discards this card, destroys an artifact, enchantment or nonbasic land
// an opponent controls, and lets that player search their library for a land
// card with a basic land type, onto the battlefield untapped. The channel's
// cost reduction is not expressible.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "target artifact, enchantment, or nonbasic land an opponent controls" —
/// the controller clause qualifies all three nouns.
static DESTROY_TARGETS: Filter = Filter::And(&[
    Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT, Filter::NONBASIC_LAND]),
    Filter::ControlledByOpponent,
]);

/// "a land card with a basic land type" (CR 205.3i names the five).
static BASIC_TYPED_LAND: Filter = Filter::And(&[
    Filter::LAND,
    Filter::Or(&[
        Filter::HasSubtype(subtypes::land::PLAINS),
        Filter::HasSubtype(subtypes::land::ISLAND),
        Filter::HasSubtype(subtypes::land::SWAMP),
        Filter::HasSubtype(subtypes::land::MOUNTAIN),
        Filter::HasSubtype(subtypes::land::FOREST),
    ]),
]);

card!(
    index = index::BOSEIJU_WHO_ENDURES,
    oracle_id = "bf1341dd-41a3-49f6-87ec-63170dde4324",
    scryfall_id = "2135ac5a-187b-4dc9-8f82-34e8d1603416",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "\"This ability costs {1} less to activate for each legendary creature \
         you control\" — nothing reduces an activation cost"
    ),
    faces = &[face!(
        name = "Boseiju, Who Endures",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        // NOT SUPPORTED: "This ability costs {1} less to activate for each
        // legendary creature you control." — `CostReduction` is a spell's
        // (`FaceDef::cost_reduction`) and carries only `NotStartingPlayer(n)`;
        // nothing reduces an activation cost, so this activation always costs
        // its printed {1}{G}.
        activated!(
            cost!("{1}{G}", DiscardSelf),
            &[
                Effect::destroy(TargetSpec::Object(&DESTROY_TARGETS)),
                Effect::SearchLibraryOf {
                    library: PlayerRel::ControllerOfTarget,
                    owner_searches: true,
                    filter: &BASIC_TYPED_LAND,
                    mana_value: None,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                },
            ],
            zone = ActivationZone::Hand,
            target = Some(TargetSpec::Object(&DESTROY_TARGETS)),
        ),
    ],
);
