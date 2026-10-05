//! Merchant Ship — {U} — Creature — Human
//! Oracle: This creature can't attack unless defending player controls an Island.
//! Oracle: Whenever this creature attacks and isn't blocked, you gain 2 life.
//! Oracle: When you control no Islands, sacrifice this creature.
//! Set: ARN #17 — Arabian Nights | Scryfall ID: 2b827094-fb2c-46db-b898-02e0c308601f | Oracle ID: 69556f6c-c05b-4902-bac7-012f0ed81b75
// PARTIAL — the attack restriction and the no-Islands sacrifice are built;
// the attack-and-isn't-blocked trigger is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND: Filter = Filter::HasSubtype(subtypes::land::ISLAND);

static YOUR_ISLANDS: Filter = Filter::And(&[ISLAND, Filter::ControlledByYou]);

static NO_ISLANDS: Condition = Condition::ControlCountAtMost(&YOUR_ISLANDS, 0);

card!(
    index = index::MERCHANT_SHIP,
    oracle_id = "69556f6c-c05b-4902-bac7-012f0ed81b75",
    scryfall_id = "2b827094-fb2c-46db-b898-02e0c308601f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Merchant Ship",
        mana_cost = mana!("{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(0),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial(
        "no trigger hears \"attacks and isn't blocked\": Trigger::Attacks fires \
         at declare attackers, before blockers are declared, so Filter::Unblocked \
         is false there and the closest trigger is a different event"
    ),
    // NOT SUPPORTED: "Whenever this creature attacks and isn't blocked, you
    // gain 2 life." — the gain is Effect::gain_life(2), but no Trigger names
    // the unblocked attacker: Trigger::Attacks fires at declare attackers
    // (CR 508.1), where Filter::Unblocked is not yet true (it needs blockers
    // declared, CR 509.1h), and no delayed trigger hears "becomes unblocked".
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::CantAttackUnlessDefenderControls(&ISLAND)
        ),
        triggered!(Trigger::State(&NO_ISLANDS), &[Effect::SacrificeSelf]),
    ],
);
