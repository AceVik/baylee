//! Willow Satyr — {2}{G}{G} — Creature — Satyr
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Gain control of target legendary creature for as long as you control this creature and this creature remains tapped.
//! Set: ME3 #139 — Masters Edition III | Scryfall ID: 2afe33f6-d93b-4ca5-bf21-396efbb9b94f | Oracle ID: c7c660bd-5f58-464c-bc75-dd244b7ca535
// PARTIAL — the untap-step choice is built; the control-change ability is
// off the card, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WILLOW_SATYR,
    oracle_id = "c7c660bd-5f58-464c-bc75-dd244b7ca535",
    scryfall_id = "2afe33f6-d93b-4ca5-bf21-396efbb9b94f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Duration lasts while the source remains tapped, so the control \
         change cannot be written"
    ),
    faces = &[face!(
        name = "Willow Satyr",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SATYR],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);

// NOT SUPPORTED: "{T}: Gain control of target legendary creature for as long
// as you control this creature and this creature remains tapped." — the
// change itself is `Effect::continuous(&Filter::This, Modifier::GainControl,
// …)` with `TargetSpec::Object(&Filter::LEGENDARY_CREATURE)`, but no
// `Duration` watches the source's tapped state: `Duration::WhileYouControlSource`
// covers only the printed "for as long as you control this creature" and
// would keep control after Willow Satyr untaps, so the ability comes off the
// card.
