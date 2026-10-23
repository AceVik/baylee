//! Volrath's Dungeon — {2}{B}{B} — Enchantment
//! Oracle: Pay 5 life: Destroy this enchantment. Any player may activate this ability but only during their turn.
//! Oracle: Discard a card: Target player puts a card from their hand on top of their library. Activate only as a sorcery.
//! Set: EXO #78 — Exodus | Scryfall ID: a4ab28e1-74e1-4c4e-920f-a658c6a44d75 | Oracle ID: a8b968ea-76f3-4ee7-9d53-d251d8a9faf6
// PARTIAL — The discard-to-put-hand-card-on-top ability is unsupported and omitted; any player may pay 5 life to destroy this enchantment only during their own turn.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: The discard-to-put-hand-card-on-top ability is unsupported and omitted; any player may pay 5 life to destroy this enchantment only during their own turn.
card!(
    index = index::VOLRATH_S_DUNGEON,
    oracle_id = "a8b968ea-76f3-4ee7-9d53-d251d8a9faf6",
    scryfall_id = "a4ab28e1-74e1-4c4e-920f-a658c6a44d75",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "The discard-to-put-hand-card-on-top ability is unsupported and omitted; any player may pay 5 life to destroy this enchantment only during their own turn."
    ),
    faces = &[face!(
        name = "Volrath's Dungeon",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[activated!(
        cost!(PayLife(5)),
        &[Effect::Destroy {
            target: TargetSpec::ThisObject,
            no_regen: false,
        }],
        zone = ActivationZone::BattlefieldAnyPlayer,
        condition = Some(Condition::YourTurn),
    )],
);
