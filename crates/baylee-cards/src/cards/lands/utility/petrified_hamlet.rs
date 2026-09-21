//! Petrified Hamlet — (no cost) — Land
//! Oracle: When this land enters, choose a land card name.
//! Oracle: Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.
//! Oracle: Lands with the chosen name have "{T}: Add {C}."
//! Oracle: {T}: Add {C}.
//! Set: SOS #259 — Secrets of Strixhaven | Scryfall ID: 355dd460-b0e9-41f2-a058-b7f7e39ac387 | Oracle ID: 78a2972c-14f4-41f3-99f9-167948bdd73a
// PARTIAL — {T}: Add {C} is built; the printed name-choosing package is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PETRIFIED_HAMLET,
    oracle_id = "78a2972c-14f4-41f3-99f9-167948bdd73a",
    scryfall_id = "355dd460-b0e9-41f2-a058-b7f7e39ac387",
    faces = &[face!(name = "Petrified Hamlet", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "choose a land card name is not expressible: no EnterModifier asks for \
         a name, no Filter matches one, and no Modifier locks activation by one",
    ),
    // NOT SUPPORTED: When this land enters, choose a land card name. — the
    // entry modifiers carry a question about a *subtype* (`ChooseSubtype`) or
    // a colour (`ChooseColor`/`ChooseColorExcept`), and nothing stores a card
    // name; there is no `Filter` for one either, since
    // `MatchesChosenTypeOfSource` reads the chosen subtype.
    // NOT SUPPORTED: Activated abilities of sources with the chosen name
    // can't be activated unless they're mana abilities. — no `Modifier` locks
    // an activation by name; `Modifier::CantActivateArtifacts` is the only
    // such lock in the vocabulary and it names a card type, not a choice.
    // NOT SUPPORTED: Lands with the chosen name have "{T}: Add {C}." —
    // `Modifier::GrantActivated` could carry the granted ability, but nothing
    // can name the lands it applies to without a filter over the chosen name,
    // and "creatures also gain the subtype in their base" has no counterpart
    // for lands and a name.
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
