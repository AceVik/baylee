//! Venarian Gold — {X}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, tap enchanted creature and put X sleep counters on it.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step if it has a sleep counter on it.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, remove a sleep counter from that creature.
//! Set: LEG #83 — Legends | Scryfall ID: 11fb92c0-bb1e-463a-a6b6-887a5d0cb873 | Oracle ID: 5d7acb89-1778-409a-9e91-aaeadd13ca27
// PARTIAL — enchant creature is written; the three sleep-counter sentences are
// off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VENARIAN_GOLD,
    oracle_id = "5d7acb89-1778-409a-9e91-aaeadd13ca27",
    scryfall_id = "11fb92c0-bb1e-463a-a6b6-887a5d0cb873",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "sleep counters have no id in `baylee_cards_dsl::counters`, and a card file \
         may not spell a raw custom-counter id (a bare number collides with the \
         next card's and `no_card_file_spells_a_counter_id_as_a_number` refuses \
         it), so all three sentences that name one are off the card"
    ),
    faces = &[face!(
        name = "Venarian Gold",
        mana_cost = mana!("{X}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "When this Aura enters, tap enchanted creature and put X
    // sleep counters on it." / "Enchanted creature doesn't untap during its
    // controller's untap step if it has a sleep counter on it." / "At the
    // beginning of the upkeep of enchanted creature's controller, remove a
    // sleep counter from that creature." — all three turn on a sleep counter,
    // and there is no `counters::SLEEP`: adding one is a DSL change, and a
    // card file writing `CounterKind::Custom(n)` is refused by the pool lints.
    abilities = &[spell!(
        &[Effect::AttachSelf {
            target: TargetSpec::Object(&Filter::CREATURE)
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
