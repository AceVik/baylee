//! Petrified Hamlet — (no cost) — Land
//! Oracle: When this land enters, choose a land card name.
//! Oracle: Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.
//! Oracle: Lands with the chosen name have "{T}: Add {C}."
//! Oracle: {T}: Add {C}.
//! Set: SOS #259 — Secrets of Strixhaven | Scryfall ID: 355dd460-b0e9-41f2-a058-b7f7e39ac387 | Oracle ID: 78a2972c-14f4-41f3-99f9-167948bdd73a
// PARTIAL — {T}: Add {C} is built; the lock by name exists in the engine, but
// the trigger that names a land card and the grant to lands with the name do
// not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PETRIFIED_HAMLET,
    oracle_id = "78a2972c-14f4-41f3-99f9-167948bdd73a",
    scryfall_id = "355dd460-b0e9-41f2-a058-b7f7e39ac387",
    faces = &[face!(name = "Petrified Hamlet", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the name is chosen by a trigger and must be a land card's, which \
         EnterModifier::ChooseCardName (as it enters, any name) does not say, \
         and no Filter matches the lands with the chosen name",
    ),
    // NOT SUPPORTED: When this land enters, choose a land card name. — the
    // one card-name question is `EnterModifier::ChooseCardName`, which is
    // asked *as* a permanent enters (a replacement, Pithing Needle) and takes
    // any card's name; this one is a triggered ability, answered on the
    // stack, and only a land card's name may be chosen (CR 201.4a).
    // NOT SUPPORTED: Activated abilities of sources with the chosen name
    // can't be activated unless they're mana abilities. — this sentence is
    // `Modifier::ChosenNameCantActivate`, which reads the name the permanent
    // keeps (`chosen_name`); it waits on the question above to write one.
    // NOT SUPPORTED: Lands with the chosen name have "{T}: Add {C}." —
    // `Modifier::GrantActivated` could carry the granted ability, but no
    // `Filter` matches an object by the chosen name.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
