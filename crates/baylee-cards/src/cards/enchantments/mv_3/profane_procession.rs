//! Profane Procession // Tomb of the Dusk Rose — {1}{W}{B} — Legendary Enchantment // Legendary Land
//! Oracle: {3}{W}{B}: Exile target creature. Then if there are three or more cards exiled with Profane Procession, transform it.
//! Oracle: (Transforms from Profane Procession.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {2}{W}{B}, {T}: Put a creature card exiled with this permanent onto the battlefield under your control.
//! Set: RIX #166 — Rivals of Ixalan | Scryfall ID: 1d94ff37-f04e-48ee-8253-d62ab07f0632 | Oracle ID: a656ad7f-133f-4d93-919a-43bcf1f815f3
//! Face: Profane Procession — {1}{W}{B} — Legendary Enchantment
//! Face: Tomb of the Dusk Rose —  — Legendary Land
// PARTIAL — the front face's {3}{W}{B}: exile target creature, and the back
// face's {T}: Add one mana of any color. The transform branch and the
// return-from-exile ability are NOT SUPPORTED, each named where it belongs.

use baylee_cards_dsl::prelude::*;

// The back face's only ability. Its face is reached by transforming and is
// never cast, which is why it says so below.
// NOT SUPPORTED: "{2}{W}{B}, {T}: Put a creature card exiled with this
// permanent onto the battlefield under your control." — the nearest variant
// is `Effect::ReturnLinkedToBattlefield`, which returns *every* card exiled
// with a link to the source, under its owner's control. Naming one creature
// card, and naming *your* control, are both beyond it.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_of_any_color()])];

card!(
    index = index::PROFANE_PROCESSION,
    oracle_id = "a656ad7f-133f-4d93-919a-43bcf1f815f3",
    scryfall_id = "1d94ff37-f04e-48ee-8253-d62ab07f0632",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[
        face!(
            name = "Profane Procession",
            mana_cost = mana!("{1}{W}{B}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Tomb of the Dusk Rose",
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            // Only ever reached by turning the card over (CR 712.2), never
            // cast: the oracle text transforms, which an MDFC's back does not.
            castable_from_hand = false,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "nothing counts the cards exiled with the source, so the transform \
         branch cannot be asked; and the linked-exile return puts back every \
         card under its owner's control rather than a chosen creature card \
         under yours"
    ),
    // NOT SUPPORTED: "Then if there are three or more cards exiled with
    // Profane Procession, transform it." — the nearest variant is
    // `Effect::ExileSelfReturnAsFace { face: 1 }`, and what is missing is the
    // branch guarding it: no `Condition`, no `Amount` and no `ZoneSel` reads
    // the cards exiled with the source, so running it unguarded would turn the
    // permanent over with nought cards under it.
    abilities = &[activated!(
        cost!("{3}{W}{B}"),
        &[Effect::exile(TargetSpec::Object(&Filter::CREATURE))],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
