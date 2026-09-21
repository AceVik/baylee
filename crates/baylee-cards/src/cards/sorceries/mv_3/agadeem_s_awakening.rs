//! Agadeem's Awakening // Agadeem, the Undercrypt — {X}{B}{B}{B} — Sorcery // Land
//! Oracle: Return from your graveyard to the battlefield any number of target creature cards that each have a different mana value X or less.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #90 — Zendikar Rising | Scryfall ID: 67f4c93b-080c-4196-b095-6a120a221988 | Oracle ID: 562d71b9-1646-474e-9293-55da6947a758
//! Face: Agadeem's Awakening — {X}{B}{B}{B} — Sorcery
//! Face: Agadeem, the Undercrypt —  — Land
// PARTIAL — built the land face: pay 3 life as it enters or it enters tapped,
// and {T}: Add {B}. The sorcery face's clause is not expressible; see the
// NOT SUPPORTED note below.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Return from your graveyard to the battlefield any number of
// target creature cards that each have a different mana value X or less." —
// the movement is Effect::GraveyardToBattlefield, but the spell's target
// requirement has no spelling: the cap is X, and Filter::CmcAtMost takes a
// constant, while "each have a different mana value" constrains the chosen set
// as a whole, which no TargetReq carries. Written without the cap the spell
// would reanimate the whole graveyard, so the ability stays off the card.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::AGADEEM_S_AWAKENING,
    oracle_id = "562d71b9-1646-474e-9293-55da6947a758",
    scryfall_id = "67f4c93b-080c-4196-b095-6a120a221988",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the sorcery face: 'any number of target creature cards that each have a different mana value X or less' has no spelling — Filter::CmcAtMost takes a constant and no TargetReq restricts the chosen set"
    ),
    faces = &[
        face!(
            name = "Agadeem's Awakening",
            mana_cost = mana!("{X}{B}{B}{B}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Agadeem, the Undercrypt",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = BACK_MANA,
        ),
    ],
);
