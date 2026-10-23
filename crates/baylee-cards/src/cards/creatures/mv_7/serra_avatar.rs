//! Serra Avatar — {4}{W}{W}{W} — Creature — Avatar
//! Oracle: Serra Avatar's power and toughness are each equal to your life total.
//! Oracle: When Serra Avatar is put into a graveyard from anywhere, shuffle it into its owner's library.
//! Set: USG #45 — Urza's Saga | Scryfall ID: 288b0976-78e8-4fbe-8607-2e55d8761d3e | Oracle ID: c363725f-15cf-4955-aaf1-536b1f8daaed
// PARTIAL: characteristic-defining power and toughness equal to your life total is not in PtCount and triggering from any zone on entering a graveyard is not in Trigger; plays as a 0/0 creature that shuffles itself into its owner's library when it dies.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERRA_AVATAR,
    oracle_id = "c363725f-15cf-4955-aaf1-536b1f8daaed",
    scryfall_id = "288b0976-78e8-4fbe-8607-2e55d8761d3e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "characteristic-defining power and toughness equal to your life total is not in PtCount and triggering from any zone on entering a graveyard is not in Trigger; plays as a 0/0 creature that shuffles itself into its owner's library when it dies",
    ),
    faces = &[face!(
        name = "Serra Avatar",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(0),
        toughness = Some(0),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::GraveyardToTop {
                target: TargetSpec::EventObject,
            },
            Effect::ShuffleLibrary {
                who: PlayerRel::OwnerOfSource,
            },
        ],
    ),],
);
