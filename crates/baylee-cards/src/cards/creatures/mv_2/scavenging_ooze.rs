//! Scavenging Ooze — {1}{G} — Creature — Ooze
//! Oracle: {G}: Exile target card from a graveyard. If it was a creature card, put a +1/+1 counter on this creature and you gain 1 life.
//! Set: FDN #232 — Foundations | Scryfall ID: 8c504c23-1e9a-411b-9cfe-4180d0c744f6 | Oracle ID: 1ff25f67-36a7-4cfa-a2b1-2135b5b6fb67
// PARTIAL — {G}: exile target card from a graveyard; the "if it was a
// creature card" rider is dropped, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "target card from a graveyard" — any player's graveyard, which is the one
/// target both the ability and its effect name.
static GRAVEYARD_CARD: TargetSpec =
    TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer);

card!(
    index = index::SCAVENGING_OOZE,
    oracle_id = "1ff25f67-36a7-4cfa-a2b1-2135b5b6fb67",
    scryfall_id = "8c504c23-1e9a-411b-9cfe-4180d0c744f6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "{G}: exile target card from a graveyard works; nothing reads back \
         what was exiled, so the +1/+1 counter and 1 life are missing"
    ),
    faces = &[face!(
        name = "Scavenging Ooze",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::OOZE],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{G}"),
        &[Effect::exile(GRAVEYARD_CARD)],
        target = Some(GRAVEYARD_CARD),
    ),],
);

// NOT SUPPORTED: "If it was a creature card, put a +1/+1 counter on this
// creature and you gain 1 life." — no Effect branches on the exiled card's
// type, so the rider is dropped rather than made unconditional.
