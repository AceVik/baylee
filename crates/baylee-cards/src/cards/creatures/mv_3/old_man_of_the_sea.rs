//! Old Man of the Sea — {1}{U}{U} — Creature — Djinn
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Gain control of target creature with power less than or equal to this creature's power for as long as this creature remains tapped and that creature's power remains less than or equal to this creature's power.
//! Set: ME3 #45 — Masters Edition III | Scryfall ID: 9029ff19-753d-4031-a192-d36b165ebfe5 | Oracle ID: cce84cf1-5574-43b0-9d75-72e6451403a7
// PARTIAL — the untap-step choice is built; the control-change ability is
// off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::OLD_MAN_OF_THE_SEA,
    oracle_id = "cce84cf1-5574-43b0-9d75-72e6451403a7",
    scryfall_id = "9029ff19-753d-4031-a192-d36b165ebfe5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Old Man of the Sea",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(2),
        toughness = Some(3),
    ),],
    coverage = Coverage::Partial(
        "no Duration lasts while the source remains tapped or ends when the \
         target's power passes the source's power, so the control change \
         cannot be written"
    ),
    // NOT SUPPORTED: "{T}: Gain control of target creature with power less
    // than or equal to this creature's power for as long as this creature
    // remains tapped and that creature's power remains less than or equal
    // to this creature's power." — the change itself is Aladdin's shape,
    // Effect::continuous(&Filter::This, Modifier::GainControl, …), but its
    // Duration::WhileYouControlSource ends at the wrong event; no Duration
    // watches the source's tapped state, none ends when the target's power
    // exceeds the source's, and the target filter's "or equal" has no
    // sibling of Filter::PowerLessThanSourcePower.
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);
