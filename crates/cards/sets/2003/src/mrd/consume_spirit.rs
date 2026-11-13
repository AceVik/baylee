//! Consume Spirit — {X}{1}{B} — Sorcery
//! Oracle: Spend only black mana on X.
//! Oracle: Consume Spirit deals X damage to any target and you gain X life.
//! Set: DVD #56 — Duel Decks Anthology: Divine vs. Demonic | Scryfall ID: ca8d42f5-99a6-4fc1-a539-831cbfd9763e | Oracle ID: 861fa80d-99e0-4332-a2b8-5aa959fd41a4
use baylee_cards_dsl::prelude::*;

card!(
    index = index::CONSUME_SPIRIT,
    oracle_id = "861fa80d-99e0-4332-a2b8-5aa959fd41a4",
    scryfall_id = "ca8d42f5-99a6-4fc1-a539-831cbfd9763e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Consume Spirit",
        mana_cost = mana!("{X}{1}{B}"),
        types = TypeSet::SORCERY,
        x_mana_color = Some(Color::Black),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::DealDamage {
                amount: Amount::X,
                target: TargetSpec::AnyTarget,
            },
            Effect::GainLife { amount: Amount::X },
        ],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
