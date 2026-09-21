//! Silundi Vision // Silundi Isle — {2}{U} — Instant // Land
//! Oracle: Look at the top six cards of your library. You may reveal an instant or sorcery card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: ZNR #80 — Zendikar Rising | Scryfall ID: 11568cdf-6148-494c-8b98-f5ca5797d775 | Oracle ID: b0182ca0-f353-4012-9121-6f4ac9f7a046
//! Face: Silundi Vision — {2}{U} — Instant
//! Face: Silundi Isle —  — Land
// PARTIAL — Silundi Isle is complete: it enters tapped and taps for {U}.
// Silundi Vision's look is Effect::LookAtTopPick, which cannot carry the
// printed restriction on what may be revealed.

use baylee_cards_dsl::prelude::*;

static SILUNDI_ISLE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

// NOT SUPPORTED: "You may reveal an instant or sorcery card from among them
// and put it into your hand. Put the rest on the bottom of your library in a
// random order." Effect::LookAtTopPick { count, pick } has no filter — it
// hands over any `pick` of the cards looked at — and it bottoms the rest in
// the order the player chose rather than at random.

card!(
    index = index::SILUNDI_VISION,
    oracle_id = "b0182ca0-f353-4012-9121-6f4ac9f7a046",
    scryfall_id = "11568cdf-6148-494c-8b98-f5ca5797d775",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Silundi Vision",
            mana_cost = mana!("{2}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Silundi Isle",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = SILUNDI_ISLE_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "Silundi Vision: only an instant or sorcery card may be revealed, and \
         the rest go to the bottom in a random order; Effect::LookAtTopPick \
         has no filter and bottoms the rest by choice"
    ),
    abilities = &[spell!(&[Effect::LookAtTopPick { count: 6, pick: 1 }])],
);
