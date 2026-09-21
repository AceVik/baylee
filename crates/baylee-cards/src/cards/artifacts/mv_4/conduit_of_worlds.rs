//! Conduit of Worlds — {2}{G}{G} — Artifact
//! Oracle: You may play lands from your graveyard.
//! Oracle: {T}: Choose target nonland permanent card in your graveyard. If you haven't cast a spell this turn, you may cast that card. If you do, you can't cast additional spells this turn. Activate only as a sorcery.
//! Set: MSC #171 — Marvel Super Heroes Commander | Scryfall ID: 8380eb8d-d1c3-4f96-b3b9-54845188c1d1 | Oracle ID: ed14be15-8f8d-4fe3-a147-f5da8ed873bf
// PARTIAL — the land permission is built; the {T} ability is not expressible.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "{T}: Choose target nonland permanent card in your
// graveyard. If you haven't cast a spell this turn, you may cast that card.
// If you do, you can't cast additional spells this turn. Activate only as a
// sorcery." — `GraveyardToHand`/`GraveyardToBattlefield` move a card but
// nothing casts one from a graveyard, no `Condition` asks how many spells
// have been cast this turn, and no `Modifier` forbids casting further spells.

card!(
    index = index::CONDUIT_OF_WORLDS,
    oracle_id = "ed14be15-8f8d-4fe3-a147-f5da8ed873bf",
    scryfall_id = "8380eb8d-d1c3-4f96-b3b9-54845188c1d1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "the {T} ability casts a nonland permanent card from the graveyard \
         behind a spells-cast-this-turn condition with a can't-cast-more \
         rider; no effect casts from a graveyard, no condition reads spells \
         cast this turn, and no modifier restricts further casts"
    ),
    faces = &[face!(
        name = "Conduit of Worlds",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[static_ability!(
        Filter::Any,
        Modifier::PlayLandsFromGraveyard
    )],
);
