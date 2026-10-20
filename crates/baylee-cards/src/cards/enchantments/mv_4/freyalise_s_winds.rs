//! Freyalise's Winds — {2}{G}{G} — Enchantment
//! Oracle: Whenever a permanent becomes tapped, put a wind counter on it.
//! Oracle: If a permanent with a wind counter on it would untap during its controller's untap step, remove all wind counters from it instead.
//! Set: ME3 #119 — Masters Edition III | Scryfall ID: a4a9d2fb-fdd1-4287-b48c-21ded059fd23 | Oracle ID: ffeb792a-6df0-46c1-badc-4deb857f7d48
use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: If a permanent with a wind counter on it would untap during its controller's untap step, remove all wind counters from it instead.
card!(
    index = index::FREYALISE_S_WINDS,
    oracle_id = "ffeb792a-6df0-46c1-badc-4deb857f7d48",
    scryfall_id = "a4a9d2fb-fdd1-4287-b48c-21ded059fd23",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "untap-step replacement removing wind counters instead of untapping not supported",
    ),
    faces = &[face!(
        name = "Freyalise's Winds",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTapped(&Filter::Any),
        &[Effect::AddCounter {
            kind: counters::WIND,
            amount: Amount::Fixed(1),
        }],
        targets = Some(TargetReq::one(TargetSpec::EventObject)),
    )],
);
