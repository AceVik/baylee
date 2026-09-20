//! Great Hall of the Biblioplex — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {T}, Pay 1 life: Add one mana of any color. Spend this mana only to cast an instant or sorcery spell.
//! Oracle: {5}: If this land isn't a creature, it becomes a 2/4 Wizard creature with "Whenever you cast an instant or sorcery spell, this creature gets +1/+0 until end of turn." It's still a land.
//! Set: SOS #257 — Secrets of Strixhaven | Scryfall ID: 42d92674-2664-411c-b9c5-b04da7c845f4 | Oracle ID: a8c70dab-1e27-4a9c-bd2d-910d5720d02d
// PARTIAL — both mana abilities ({C}, and one mana of any color for {T} plus 1
// life, spendable only on an instant or sorcery spell); the {5} animation is
// not expressible, see the NOT SUPPORTED line at the foot of the file.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GREAT_HALL_OF_THE_BIBLIOPLEX,
    oracle_id = "a8c70dab-1e27-4a9c-bd2d-910d5720d02d",
    scryfall_id = "42d92674-2664-411c-b9c5-b04da7c845f4",
    faces = &[face!(
        name = "Great Hall of the Biblioplex",
        types = TypeSet::LAND,
    ),],
    coverage = Coverage::Partial(
        "the {5} ability: no DSL effect animates the source itself (a created \
         continuous effect binds Filter::This to its first target, and this \
         ability targets nothing), no branch asks whether the source is a \
         creature, and nothing animates a permanent while leaving it a land"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(
            cost!(TapSelf, PayLife(1)),
            &[Effect::mana_of_any_color()
                .restricted(&Filter::INSTANT_OR_SORCERY, SpendRider::None)]
        ),
    ],
);

// NOT SUPPORTED: "{5}: If this land isn't a creature, it becomes a 2/4 Wizard
// creature with 'Whenever you cast an instant or sorcery spell, this creature
// gets +1/+0 until end of turn.' It's still a land." — turning the *source*
// into a creature is three characteristics at once (type, P/T, a granted
// trigger) and the DSL has no effect that acts on the source without being
// given a target it does not print; nor is there a conditional that reads the
// source's own types.
