//! Ojer Taq, Deepest Foundation // Temple of Civilization — {4}{W}{W} — Legendary Creature — God // Land
//! Oracle: Vigilance
//! Oracle: If one or more creature tokens would be created under your control, three times that many of those tokens are created instead.
//! Oracle: When Ojer Taq dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Ojer Taq, Deepest Foundation.)
//! Oracle: {T}: Add {W}.
//! Oracle: {2}{W}, {T}: Transform this land. Activate only if you attacked with three or more creatures this turn and only as a sorcery.
//! Set: LCI #26 — The Lost Caverns of Ixalan | Scryfall ID: 1ca79dd4-67fc-496c-96fc-489b039c4932 | Oracle ID: 486bb9a5-73f1-4cec-b097-fb07ac80b72e
//! Face: Ojer Taq, Deepest Foundation — {4}{W}{W} — Legendary Creature — God
//! Face: Temple of Civilization —  — Land
// PARTIAL — front-face vigilance and Temple of Civilization's {T}: Add {W}
// are built; the token tripler, the dies trigger and the transform activation
// have no DSL shape.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// `{T}: Add {W}.` — Temple of Civilization prints no basic land type, so
/// CR 305.6 grants it no intrinsic mana ability and the card states its own.
static TEMPLE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::OJER_TAQ_DEEPEST_FOUNDATION,
    oracle_id = "486bb9a5-73f1-4cec-b097-fb07ac80b72e",
    scryfall_id = "1ca79dd4-67fc-496c-96fc-489b039c4932",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Ojer Taq, Deepest Foundation",
            mana_cost = mana!("{4}{W}{W}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(6),
            toughness = Some(6),
            keywords = KeywordSet::VIGILANCE,
        ),
        face!(
            name = "Temple of Civilization",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = TEMPLE_MANA,
        ),
    ],
    // NOT SUPPORTED: "If one or more creature tokens would be created under
    // your control, three times that many of those tokens are created
    // instead." — ReplacementRule carries DoubleTokenCreation and nothing
    // that counts higher than twice.
    // NOT SUPPORTED: "When Ojer Taq dies, return it to the battlefield tapped
    // and transformed under its owner's control." — no effect returns the
    // source out of a graveyard, let alone tapped and on its back face.
    // NOT SUPPORTED: "{2}{W}, {T}: Transform this land. Activate only if you
    // attacked with three or more creatures this turn and only as a
    // sorcery." — there is no transform effect, and no Condition for "you
    // attacked with N creatures this turn".
    coverage = Coverage::Partial(
        "ReplacementRule cannot triple tokens, no effect returns the source \
         from a graveyard transformed, and no transform effect exists",
    ),
);
