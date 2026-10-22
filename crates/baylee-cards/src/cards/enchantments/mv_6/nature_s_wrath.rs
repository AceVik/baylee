//! Nature's Wrath — {4}{G}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you pay {G}.
//! Oracle: Whenever a player puts an Island or blue permanent onto the battlefield, that player sacrifices an Island or blue permanent of their choice.
//! Oracle: Whenever a player puts a Swamp or black permanent onto the battlefield, that player sacrifices a Swamp or black permanent of their choice.
//! Set: ME2 #172 — Masters Edition II | Scryfall ID: 91a05306-f3f7-436d-8d8c-58217f04610b | Oracle ID: 5a647f28-6e26-4262-8593-d582e0a691f7
// IMPLEMENTED — upkeep {G} payment or sacrifice, plus ETB triggers for Islands/blue permanents and Swamps/black permanents.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ISLAND_OR_BLUE: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::ISLAND),
    Filter::HasColor(ColorSet::from_slice(&[Color::Blue])),
]);
static SWAMP_OR_BLACK: Filter = Filter::Or(&[
    Filter::HasSubtype(subtypes::land::SWAMP),
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);

card!(
    index = index::NATURE_S_WRATH,
    oracle_id = "5a647f28-6e26-4262-8593-d582e0a691f7",
    scryfall_id = "91a05306-f3f7-436d-8d8c-58217f04610b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nature's Wrath",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaOr {
                player: PlayerRel::You,
                cost: mana!("{G}"),
                effect: &Effect::SacrificeSelf,
            }],
        ),
        triggered!(
            Trigger::EntersBattlefield(&ISLAND_OR_BLUE),
            &[Effect::SacrificeFilter {
                who: PlayerRel::ControllerOfEvent,
                filter: &ISLAND_OR_BLUE,
            }],
        ),
        triggered!(
            Trigger::EntersBattlefield(&SWAMP_OR_BLACK),
            &[Effect::SacrificeFilter {
                who: PlayerRel::ControllerOfEvent,
                filter: &SWAMP_OR_BLACK,
            }],
        ),
    ],
);
