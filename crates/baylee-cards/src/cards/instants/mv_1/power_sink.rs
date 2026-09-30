//! Power Sink — {X}{U} — Instant
//! Oracle: Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.
//! Set: VMA #88 — Vintage Masters | Scryfall ID: 9a6f3ce5-d4a5-4d7b-a7f9-b249c1d88e8f | Oracle ID: 39412e6d-2837-4729-abf9-e64a5ba87e40
// PARTIAL — tapping the lands and emptying the mana pool of a player who doesn't
// pay are not in the engine; it counters unless {X} is paid.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::POWER_SINK,
    oracle_id = "39412e6d-2837-4729-abf9-e64a5ba87e40",
    scryfall_id = "9a6f3ce5-d4a5-4d7b-a7f9-b249c1d88e8f",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "tapping the lands and emptying the mana pool of a player who doesn't pay are not in the engine; it counters unless {X} is paid"
    ),
    faces = &[face!(
        name = "Power Sink",
        mana_cost = mana!("{X}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfTarget,
                mana: Amount::X,
                effect: &Effect::CounterTargetSpell,
            }],
            targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
        ),
        // NOT SUPPORTED: If that player doesn't, they tap all lands with mana abilities
        // they control and lose all unspent mana.
    ],
);
