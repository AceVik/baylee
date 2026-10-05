//! Lesser Werewolf — {3}{B} — Creature — Werewolf
//! Oracle: {B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1 counter on target creature blocking or blocked by this creature. Activate only during the declare blockers step.
//! Set: ME3 #71 — Masters Edition III | Scryfall ID: dcbda60c-5a53-4232-b89c-267f7b11c192 | Oracle ID: 5542afeb-285f-4889-9d09-46ce20d0d189
// PARTIAL — the whole ability is off the card, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LESSER_WEREWOLF,
    oracle_id = "5542afeb-285f-4889-9d09-46ce20d0d189",
    scryfall_id = "dcbda60c-5a53-4232-b89c-267f7b11c192",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "no Filter names the creatures blocking or blocked by this creature, \
         so the ability has no target it can name"
    ),
    faces = &[face!(
        name = "Lesser Werewolf",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WEREWOLF],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// NOT SUPPORTED: "{B}: If this creature's power is 1 or more, it gets -1/-0
// until end of turn and put a -0/-1 counter on target creature blocking or
// blocked by this creature. Activate only during the declare blockers
// step." — the power gate (`Condition::SourceMatches(&Filter::PowerAtLeast(1))`),
// the self pump (`Effect::continuous` with `Modifier::ModifyPT(-1, 0)`), the
// counter (`Effect::AddCounter` with `CounterKind::Minus { power: 0,
// toughness: 1 }`), and the window (`Condition::DuringStep(DeclareBlockers)`)
// are each sayable, but no `Filter` names the combat partners of the source:
// `Filter::Blocking` tests a creature's own combat state, not a relationship
// to another object. The ability comes off the card rather than shipping
// with an untargeted approximation.
