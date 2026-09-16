//! Past in Flames — {3}{R} — Sorcery
//! Oracle: Each instant and sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.
//! Oracle: Flashback {4}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MM3 #105 — Modern Masters 2017 | Scryfall ID: 2b7472f4-37b0-439f-b4ac-80706d40d191 | Oracle ID: 37a18736-5fe2-4897-809b-013497bdd890
// PARTIAL — the graveyard-wide flashback grant is built as a continuous
// effect carrying `Modifier::GrantsFlashback`; this card's own flashback
// {4}{R} has no spelling in the DSL and is noted below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PAST_IN_FLAMES,
    oracle_id = "37a18736-5fe2-4897-809b-013497bdd890",
    scryfall_id = "2b7472f4-37b0-439f-b4ac-80706d40d191",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Past in Flames",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
    coverage = Coverage::Partial(
        "this card's own flashback {4}{R} — no variant gives a face a second \
         cost paid from a graveyard",
    ),
    // NOT SUPPORTED: Flashback {4}{R} — nothing in the DSL casts a card from a graveyard: there is no ability kind for it, an `AlternativeCost` is paid from hand (its conditions are `Always`, `NotYourTurn`, `CommanderControlled`), and no keyword bit reads flashback.
    abilities = &[spell!(&[Effect::continuous(
        &Filter::And(&[
            Filter::INSTANT_OR_SORCERY,
            Filter::InZone(ZoneRef::Graveyard),
            Filter::OwnedByYou,
        ]),
        Modifier::GrantsFlashback,
        Duration::UntilEndOfTurn,
    )])],
);
