//! Repopulate — {1}{G} — Instant
//! Oracle: Shuffle all creature cards from target player's graveyard into that player's library.
//! Oracle: Cycling {2} ({2}, Discard this card: Draw a card.)
//! Set: ULG #111 — Urza's Legacy | Scryfall ID: aac77869-97bc-4976-9ee0-3d60e162b78a | Oracle ID: 3a225d39-c1f8-4ab0-98ec-dad63bf268db
// NOT SUPPORTED: filtering the shuffled graveyard cards to only creature cards requires a filtered graveyard-to-library shuffle variant.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REPOPULATE,
    oracle_id = "3a225d39-c1f8-4ab0-98ec-dad63bf268db",
    scryfall_id = "aac77869-97bc-4976-9ee0-3d60e162b78a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "shuffles target player's graveyard into their library and has cycling {2}; filtering the shuffled graveyard cards to only creature cards requires a filtered graveyard-to-library shuffle variant",
    ),
    faces = &[face!(
        name = "Repopulate",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::ShuffleIntoLibrary {
                who: PlayerRel::Chosen,
                hand: false,
                graveyard: true,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
        ),
        activated!(
            cost!("{2}", DiscardSelf),
            &[Effect::draw(1)],
            zone = ActivationZone::Hand,
        ),
    ],
);
