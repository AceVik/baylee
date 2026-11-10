//! Aether Storm — {3}{U} — Enchantment
//! Oracle: Creature spells can't be cast.
//! Oracle: Pay 4 life: Destroy this enchantment. It can't be regenerated. Any player may activate this ability.
//! Set: ME2 #39 — Masters Edition II | Scryfall ID: 1881a6b8-f73f-4be2-b64f-751d23ead511 | Oracle ID: ff4297d3-3d96-4bd6-a606-1bdc20a6df2b
// IMPLEMENTED — nonland creature spells can't be cast; any player may pay 4 life to destroy it (it can't be regenerated).

use baylee_cards_dsl::prelude::*;

static NONLAND_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::NONLAND]);

card!(
    index = index::AETHER_STORM,
    oracle_id = "ff4297d3-3d96-4bd6-a606-1bdc20a6df2b",
    scryfall_id = "1881a6b8-f73f-4be2-b64f-751d23ead511",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Aether Storm",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        // "Creature spells can't be cast": a land creature (Dryad Arbor) is
        // played, not cast, so lands stay out of the prohibition.
        static_ability!(Filter::This, Modifier::PlayersCantPlay(&NONLAND_CREATURE)),
        activated!(
            cost!(PayLife(4)),
            &[Effect::destroy_no_regen(TargetSpec::ThisObject)],
            zone = ActivationZone::BattlefieldAnyPlayer
        ),
    ],
);
