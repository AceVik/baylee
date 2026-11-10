//! Rapid Decay — {1}{B} — Instant
//! Oracle: Exile up to three target cards from a single graveyard.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: UDS #67 — Urza's Destiny | Scryfall ID: 1678d911-1456-4631-a2f4-d7de4906644b | Oracle ID: 2c769dfc-79be-4fcb-840c-b72cafdd0a23
// NOT SUPPORTED: exiles up to one target card from a graveyard instead of up to three from a single graveyard.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RAPID_DECAY,
    oracle_id = "2c769dfc-79be-4fcb-840c-b72cafdd0a23",
    scryfall_id = "1678d911-1456-4631-a2f4-d7de4906644b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "exiles up to one target card from a graveyard instead of up to three from a single graveyard ('from a single graveyard' is narrowed for an activated ability only, never for a spell, so up to three targets could come from two graveyards); Cycling {2} is supported"
    ),
    faces = &[face!(
        name = "Rapid Decay",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
        spell!(
            &[Effect::exile(TargetSpec::CardInGraveyard(
                &Filter::Any,
                PlayerRel::EachPlayer,
            ))],
            targets = Some(TargetReq::up_to(
                TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
                1,
            )),
        ),
    ],
);
