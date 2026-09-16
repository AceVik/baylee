//! Pinnacle Monk // Mystic Peak — {3}{R}{R} — Creature — Djinn Monk // Land
//! Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
//! Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: MH3 #246 — Modern Horizons 3 | Scryfall ID: 24d4f26e-7f96-4b38-867e-4fac819b2679 | Oracle ID: f3d48efa-910a-4872-a5b1-a353c5dbce99
//! Face: Pinnacle Monk — {3}{R}{R} — Creature — Djinn Monk
//! Face: Mystic Peak —  — Land
// IMPLEMENTED — a prowess Djinn Monk whose own ETB buys one instant or
// sorcery back out of your graveyard; the modal back is reached by the face
// choice on a land play (CR 712.4a), asks for 3 life as it enters or comes
// down tapped, and taps for {R}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PEAK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])];

card!(
    index = index::PINNACLE_MONK,
    oracle_id = "f3d48efa-910a-4872-a5b1-a353c5dbce99",
    scryfall_id = "24d4f26e-7f96-4b38-867e-4fac819b2679",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Pinnacle Monk",
            mana_cost = mana!("{3}{R}{R}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::DJINN, subtypes::creature::MONK],
            power = Some(2),
            toughness = Some(2),
        ),
        face!(
            name = "Mystic Peak",
            types = TypeSet::LAND,
            abilities = PEAK_MANA,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    keywords = KeywordSet::PROWESS,
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::GraveyardToHand {
            target: TargetSpec::CardInGraveyard(&Filter::INSTANT_OR_SORCERY, PlayerRel::You),
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::INSTANT_OR_SORCERY,
            PlayerRel::You,
        )))
    )],
);

// Engine-level test belongs in baylee-engine (mdfc_tests): the creature front
// is a 2/2 that grows to 3/3 for the turn on a noncreature spell and pulls one
// instant or sorcery out of its controller's graveyard as it enters, while the
// back face is offered as a second `CastModeKind::PlayLandFace` on a land play,
// puts up the pay-3-life-or-enter-tapped question, and taps for {R}.
