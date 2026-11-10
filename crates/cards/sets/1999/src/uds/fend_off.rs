//! Fend Off — {1}{W} — Instant
//! Oracle: Prevent all combat damage that would be dealt by target creature this turn.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: UDS #7 — Urza's Destiny | Scryfall ID: a64d7a33-986d-45ad-8662-7bca80d3628d | Oracle ID: fa8f3827-8cd9-4896-ab0c-26fecacceb40
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FEND_OFF,
    oracle_id = "fa8f3827-8cd9-4896-ab0c-26fecacceb40",
    scryfall_id = "a64d7a33-986d-45ad-8662-7bca80d3628d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fend Off",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn,
            )],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand
        ),
    ],
);
