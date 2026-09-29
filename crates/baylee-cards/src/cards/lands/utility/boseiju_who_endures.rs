//! Boseiju, Who Endures — (no cost) — Legendary Land
//! Oracle: {T}: Add {G}.
//! Oracle: Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.
//! Set: NEO #266 — Kamigawa: Neon Dynasty | Scryfall ID: 2135ac5a-187b-4dc9-8f82-34e8d1603416 | Oracle ID: bf1341dd-41a3-49f6-87ec-63170dde4324
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "target artifact, enchantment, or nonbasic land an opponent controls" —
/// the controller clause qualifies all three nouns.
static DESTROY_TARGETS: Filter = Filter::And(&[
    Filter::Or(&[Filter::ARTIFACT, Filter::ENCHANTMENT, Filter::NONBASIC_LAND]),
    Filter::ControlledByOpponent,
]);

/// "each legendary creature you control", counted for the channel's price.
static LEGENDARY_CREATURE_YOU_CONTROL: Filter =
    Filter::And(&[Filter::LEGENDARY_CREATURE, Filter::ControlledByYou]);

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
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Boseiju, Who Endures",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
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
            cost_reduction = Some(CostReduction::PerCount {
                amount: Amount::CountOf {
                    filter: &LEGENDARY_CREATURE_YOU_CONTROL,
                    zone: ZoneSel::Battlefield,
                },
                each: 1,
            }),
        ),
    ],
);
