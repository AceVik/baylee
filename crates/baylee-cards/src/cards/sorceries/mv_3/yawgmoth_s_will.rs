//! Yawgmoth's Will — {2}{B} — Sorcery
//! Oracle: Until end of turn, you may play lands and cast spells from your graveyard.
//! Oracle: If a card would be put into your graveyard from anywhere this turn, exile that card instead.
//! Set: VMA #148 — Vintage Masters | Scryfall ID: 337239c7-73c4-4e2d-9160-ed26927dea1d | Oracle ID: 322f0459-f394-44f0-977b-55fd0cbe0712
// PARTIAL — the land half of the first sentence, created as a continuous
// PlayLandsFromGraveyard effect until end of turn. The spell half of that
// sentence and the whole second sentence have no DSL variant; both are
// marked NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAWGMOTH_S_WILL,
    oracle_id = "322f0459-f394-44f0-977b-55fd0cbe0712",
    scryfall_id = "337239c7-73c4-4e2d-9160-ed26927dea1d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Yawgmoth's Will",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "no Modifier grants casting spells from a graveyard, and ReplacementRule has no graveyard-to-exile variant"
    ),
    abilities = &[spell!(&[
        // "Until end of turn, you may play lands … from your graveyard."
        Effect::continuous(
            &Filter::Any,
            Modifier::PlayLandsFromGraveyard,
            Duration::UntilEndOfTurn,
        ),
        // NOT SUPPORTED: "… and cast spells from your graveyard" — no Modifier
        // grants a casting permission over a zone. Modifier::GrantsFlashback is
        // a grant to one named card in a graveyard, not a permission to cast the
        // cards that are sitting there.
        // NOT SUPPORTED: "If a card would be put into your graveyard from anywhere
        // this turn, exile that card instead." — ReplacementRule carries only
        // token/counter doubling and trigger multipliers/suppressors.
    ])],
);
