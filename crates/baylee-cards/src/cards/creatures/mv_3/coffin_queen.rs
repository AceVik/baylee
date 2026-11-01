//! Coffin Queen — {2}{B} — Creature — Zombie Wizard
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {2}{B}, {T}: Put target creature card from a graveyard onto the battlefield under your control. When this creature becomes untapped or you lose control of this creature, exile that creature.
//! Set: TPR #87 — Tempest Remastered | Scryfall ID: 632b83e3-f0c5-4135-96db-0bf33e6067aa | Oracle ID: c19ec6aa-aadd-4767-b695-384fd07b4ded
// PARTIAL — Only "You may choose not to untap this creature during your untap step" is implemented. NOT SUPPORTED: "{2}{B}, {T}: Put target creature card from a graveyard onto the battlefield under your control. When this creature becomes untapped or you lose control of this creature, exile that creature": no trigger says "becomes untapped" or "you lose control of this creature" for the exile, and the reanimation without it would keep the creature for good.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::COFFIN_QUEEN,
    oracle_id = "c19ec6aa-aadd-4767-b695-384fd07b4ded",
    scryfall_id = "632b83e3-f0c5-4135-96db-0bf33e6067aa",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Only \"You may choose not to untap this creature during your untap step\" is implemented. NOT SUPPORTED: \"{2}{B}, {T}: Put target creature card from a graveyard onto the battlefield under your control. When this creature becomes untapped or you lose control of this creature, exile that creature\": no trigger says \"becomes untapped\" or \"you lose control of this creature\" for the exile, and the reanimation without it would keep the creature for good."
    ),
    faces = &[face!(
        name = "Coffin Queen",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        // NOT SUPPORTED: "{2}{B}, {T}: Put target creature card from a graveyard
        // onto the battlefield under your control. When this creature becomes
        // untapped or you lose control of this creature, exile that creature."
        // The exile has no trigger ("becomes untapped", "you lose control of
        // this creature"); the reanimation alone would keep the creature for
        // good, which is more than printed.
    ],
);
