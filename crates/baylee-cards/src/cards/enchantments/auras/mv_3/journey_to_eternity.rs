//! Journey to Eternity // Atzal, Cave of Eternity — {1}{B}{G} — Legendary Enchantment — Aura // Legendary Land
//! Oracle: Enchant creature you control
//! Oracle: When enchanted creature dies, return it to the battlefield under your control, then return this card to the battlefield transformed under your control.
//! Oracle: (Transforms from Journey to Eternity.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {3}{B}{G}, {T}: Return target creature card from your graveyard to the battlefield.
//! Set: RIX #160 — Rivals of Ixalan | Scryfall ID: d81c4b3f-81c2-403b-8a5d-c9415f73a1f9 | Oracle ID: 7d6ccd0b-df16-40b2-930b-bcde0b6ef73f
//! Face: Journey to Eternity — {1}{B}{G} — Legendary Enchantment — Aura
//! Face: Atzal, Cave of Eternity —  — Legendary Land
// IMPLEMENTED — the Aura's enchanting clause as one `spell!` (AttachSelf plus
// the "creature you control" requirement the engine reads its legality from),
// its dies trigger returning the event object from the graveyard and then
// returning this card as its back face, and Atzal's two activated abilities:
// any-colour mana and the graveyard reanimation.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Atzal's side, reached only by transforming the card.
static ATZAL_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana_of_any_color()]),
    activated!(
        cost!("{3}{B}{G}", TapSelf),
        &[Effect::GraveyardToBattlefield {
            target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
        }],
        target = Some(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You
        )),
    ),
];

card!(
    index = index::JOURNEY_TO_ETERNITY,
    oracle_id = "7d6ccd0b-df16-40b2-930b-bcde0b6ef73f",
    scryfall_id = "d81c4b3f-81c2-403b-8a5d-c9415f73a1f9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[
        face!(
            name = "Journey to Eternity",
            mana_cost = mana!("{1}{B}{G}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::enchantment::AURA],
        ),
        face!(
            name = "Atzal, Cave of Eternity",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            castable_from_hand = false,
            abilities = ATZAL_ABILITIES,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_CREATURE)))
        ),
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[
                Effect::GraveyardToBattlefield {
                    target: TargetSpec::EventObject,
                },
                Effect::ExileSelfReturnAsFace { face: 1 },
            ]
        ),
    ],
);
