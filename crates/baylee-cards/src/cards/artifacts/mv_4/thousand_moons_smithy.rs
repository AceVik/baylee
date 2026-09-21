//! Thousand Moons Smithy // Barracks of the Thousand — {2}{W}{W} — Legendary Artifact // Legendary Artifact Land
//! Oracle: When Thousand Moons Smithy enters, create a white Gnome Soldier artifact creature token with "This token's power and toughness are each equal to the number of artifacts and/or creatures you control."
//! Oracle: At the beginning of your first main phase, you may tap five untapped artifacts and/or creatures you control. If you do, transform Thousand Moons Smithy.
//! Oracle: (Transforms from Thousand Moons Smithy.)
//! Oracle: {T}: Add {W}.
//! Oracle: Whenever you cast an artifact or creature spell using mana produced by Barracks of the Thousand, create a white Gnome Soldier artifact creature token with "This token's power and toughness are each equal to the number of artifacts and/or creatures you control."
//! Set: LCI #39 — The Lost Caverns of Ixalan | Scryfall ID: 4a6bec46-1acd-4726-b8d9-3045ac6a2ea2 | Oracle ID: 32af5f7b-a970-484a-9aff-226749551d32
//! Face: Thousand Moons Smithy — {2}{W}{W} — Legendary Artifact
//! Face: Barracks of the Thousand —  — Legendary Artifact Land
// PARTIAL — only the back face's `{T}: Add {W}` is built. The three clauses
// that make the card are below, each with the vocabulary it would need.
//
// NOT SUPPORTED: "When Thousand Moons Smithy enters, create a white Gnome
// Soldier artifact creature token with '…'." The *effect* is expressible
// (`Effect::CreateTokenPtPerCount` with a filter of artifacts and/or
// creatures you control), but no Gnome Soldier token exists in
// `crate::tokens`, and a card file may not define one — the token's id is its
// index in the ledger, so a `TokenDef` literal here has no id at all.
//
// NOT SUPPORTED: "At the beginning of your first main phase, you may tap five
// untapped artifacts and/or creatures you control. If you do, transform
// Thousand Moons Smithy." `Trigger::StepBegin`/`StepKind` has no main phase
// (Upkeep, Draw, CombatBegin, End), and no cost taps five permanents —
// `CostPart::TapOther(filter)` names one, and "if you do" is outside the
// vocabulary too.
//
// NOT SUPPORTED: "Whenever you cast an artifact or creature spell using mana
// produced by Barracks of the Thousand, create a white Gnome Soldier …". Mana
// carries no provenance: `SpendRider` is None/Uncounterable/Scry(n), and
// nothing ties a cast trigger back to the permanent that produced the mana.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {W}.` — the whole of the back face the DSL can say.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::THOUSAND_MOONS_SMITHY,
    oracle_id = "32af5f7b-a970-484a-9aff-226749551d32",
    scryfall_id = "4a6bec46-1acd-4726-b8d9-3045ac6a2ea2",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Thousand Moons Smithy",
            mana_cost = mana!("{2}{W}{W}"),
            types = TypeSet::ARTIFACT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Barracks of the Thousand",
            types = TypeSet::ARTIFACT.union(TypeSet::LAND),
            supertypes = SupertypeSet::LEGENDARY,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "the enter-trigger Gnome Soldier token, the first-main-phase transform and the mana-produced-by cast trigger are not expressible; only the back face's {T}: Add {W} is built"
    ),
);
