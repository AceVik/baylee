//! Sundering Eruption // Volcanic Fissure — {2}{R} — Sorcery // Land
//! Oracle: Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle. Creatures without flying can't block this turn.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: MH3 #248 — Modern Horizons 3 | Scryfall ID: 50686ac7-346c-43d1-bdaa-28d46a12ad93 | Oracle ID: c95309e9-5c2f-4518-b2fd-825d3d0a4ae0
//! Face: Sundering Eruption — {2}{R} — Sorcery
//! Face: Volcanic Fissure —  — Land
// IMPLEMENTED — land destruction plus the destroyed land's controller's
// basic-land search, the board-wide "can't block" rider, and the back
// face's pay-3-life entry and {R} mana ability.

use baylee_cards_dsl::prelude::*;

/// "Creatures without flying" — every creature on the table and not only
/// yours, which is what the printed sentence says and what makes this a
/// `PumpFilter` with no `controlled_by` rather than a targeted grant.
///
/// The set is fixed as the spell resolves (CR 611.2c), so a creature that
/// arrives afterwards may block and one that loses flying in response may
/// not.
static GROUNDED: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),
]);

card!(
    index = index::SUNDERING_ERUPTION,
    oracle_id = "c95309e9-5c2f-4518-b2fd-825d3d0a4ae0",
    scryfall_id = "50686ac7-346c-43d1-bdaa-28d46a12ad93",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Sundering Eruption",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Volcanic Fissure",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
        ),
    ],
    abilities = &[spell!(
        &[
            Effect::destroy(TargetSpec::Object(&Filter::LAND)),
            Effect::OptionalBasicLandSearchFor {
                player: PlayerRel::ControllerOfTarget,
            },
            Effect::PumpFilter {
                filter: &GROUNDED,
                controlled_by: None,
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::CANT_BLOCK,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
    )],
);
