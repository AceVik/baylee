//! Powerleech — {G}{G} — Enchantment
//! Oracle: Whenever an artifact an opponent controls becomes tapped or an opponent activates an artifact's ability without {T} in its activation cost, you gain 1 life.
//! Set: ATQ #34 — Antiquities | Scryfall ID: ae1d7b09-3a1f-410f-b330-04ae768b0455 | Oracle ID: 877d42be-f74d-4521-87b7-bb59824d0acc
// PARTIAL — the "becomes tapped" half is written; the "opponent activates an
// ability without {T}" half is off the card.

use baylee_cards_dsl::prelude::*;

/// "An artifact an opponent controls."
static OPPONENT_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByOpponent]);

card!(
    index = index::POWERLEECH,
    oracle_id = "877d42be-f74d-4521-87b7-bb59824d0acc",
    scryfall_id = "ae1d7b09-3a1f-410f-b330-04ae768b0455",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no trigger hears an activated ability being activated: only \
         `Trigger::TappedForMana` observes an activation, and only of a mana \
         ability that tapped its source, so the {T}-less-activation half is off"
    ),
    faces = &[face!(
        name = "Powerleech",
        mana_cost = mana!("{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "or an opponent activates an artifact's ability without
    // {T} in its activation cost" — no `Trigger` variant hears an activated
    // ability; `Trigger::TappedForMana` is mana abilities only.
    abilities = &[triggered!(
        Trigger::BecomesTapped(&OPPONENT_ARTIFACT),
        &[Effect::gain_life(1)]
    )],
);
