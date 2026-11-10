//! Early Harvest — {1}{G}{G} — Instant
//! Oracle: Target player untaps all basic lands they control.
//! Set: 9ED #235 — Ninth Edition | Scryfall ID: 22113d08-ecc9-4efd-90f1-00d699582ec3 | Oracle ID: 32001382-3079-4bbe-8d86-05da654c0d1a
// PARTIAL — only its caster can be targeted; untapping another player's basic lands is unsupported.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARLY_HARVEST,
    oracle_id = "32001382-3079-4bbe-8d86-05da654c0d1a",
    scryfall_id = "22113d08-ecc9-4efd-90f1-00d699582ec3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "Target yourself to untap all basic lands you control; filtering basic lands controlled by another targeted player is unsupported."
    ),
    faces = &[face!(
        name = "Early Harvest",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: Basic lands controlled by another targeted player cannot be selected by a filter.
    abilities = &[spell!(
        &[Effect::UntapAll {
            filter: &Filter::YOUR_BASIC_LAND
        }],
        targets = Some(TargetReq::one(TargetSpec::Player(PlayerRel::You))),
    )],
);
