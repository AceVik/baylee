//! Svogthos, the Restless Tomb — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}{B}{G}: Until end of turn, this land becomes a black and green Plant Zombie creature with "This creature's power and toughness are each equal to the number of creature cards in your graveyard." It's still a land.
//! Set: CM2 #270 — Commander Anthology Volume II | Scryfall ID: ffe08b0f-bb53-40b9-8057-10b60070459b | Oracle ID: a34a70b8-02e5-4e8c-a9e7-b21c5a11dddf
// IMPLEMENTED — {T}: Add {C}; {3}{B}{G} animates this land into a black and
// green Plant Zombie creature whose power and toughness are set from the
// creature cards in your graveyard, and it is still a land.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "…are each equal to the number of creature cards in your graveyard."
///
/// A count over a zone rather than over the battlefield, which is why it is
/// an `Amount` on `SetPTFilter` and not a `Modifier`: no modifier carries an
/// amount.
const GRAVEYARD_CREATURES: Amount = Amount::CountOf {
    filter: &Filter::CREATURE,
    zone: ZoneSel::GraveyardYou,
};

/// The whole animation — one printed sentence, five characteristics, one
/// duration, every effect bound to the source (`Filter::This`, and the
/// ability targets nothing).
///
/// "It's still a land" is what is *not* written: nothing here removes a type,
/// so the land type survives the sentence by construction.
static ANIMATE: &[Effect] = &[
    Effect::continuous(
        &Filter::This,
        Modifier::AddType(TypeSet::CREATURE),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddColor(ColorSet::from_slice(&[Color::Black, Color::Green])),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddSubtype(subtypes::creature::PLANT),
        Duration::UntilEndOfTurn,
    ),
    Effect::continuous(
        &Filter::This,
        Modifier::AddSubtype(subtypes::creature::ZOMBIE),
        Duration::UntilEndOfTurn,
    ),
    Effect::SetPTFilter {
        filter: &Filter::This,
        power: GRAVEYARD_CREATURES,
        toughness: GRAVEYARD_CREATURES,
        duration: Duration::UntilEndOfTurn,
    },
];

card!(
    index = index::SVOGTHOS_THE_RESTLESS_TOMB,
    oracle_id = "a34a70b8-02e5-4e8c-a9e7-b21c5a11dddf",
    scryfall_id = "ffe08b0f-bb53-40b9-8057-10b60070459b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Svogthos, the Restless Tomb",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(cost!("{3}{B}{G}"), ANIMATE),
    ],
);
