//! Primal Amulet // Primal Wellspring — {4} — Artifact // Land
//! Oracle: Instant and sorcery spells you cast cost {1} less to cast.
//! Oracle: Whenever you cast an instant or sorcery spell, put a charge counter on this artifact. Then if there are four or more charge counters on it, you may remove those counters and transform it.
//! Oracle: (Transforms from Primal Amulet.)
//! Oracle: {T}: Add one mana of any color. When that mana is spent to cast an instant or sorcery spell, copy that spell and you may choose new targets for the copy.
//! Set: XLN #243 — Ixalan | Scryfall ID: d4d379b5-7f56-4a7d-a4ac-131fc3d579c6 | Oracle ID: 8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49
//! Face: Primal Amulet — {4} — Artifact
//! Face: Primal Wellspring —  — Land
// PARTIAL — the front face's charge-counter trigger and the back face's
// mana ability are built; the cost reduction, the counter-threshold
// transform clause and the mana's copy rider have no variant.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "Instant and sorcery spells you cast cost {1} less to cast."
// — no `Modifier` reduces a cost; `CostReduction` carries only
// NotStartingPlayer, which reduces the card's own printed cost.
// NOT SUPPORTED: "Then if there are four or more charge counters on it, you
// may remove those counters and transform it." — the check is sayable
// (`Effect::IfCondition` over `Condition::CountersOnSelf`), but nothing
// removes counters from the source as an effect, and nothing transforms a
// permanent in place (#206).
// NOT SUPPORTED: "When that mana is spent to cast an instant or sorcery
// spell, copy that spell…" — `SpendRider` has no copy arm.

static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_of_any_color()])];

card!(
    index = index::PRIMAL_AMULET,
    oracle_id = "8e4d0da0-c7d8-4a20-9bfd-02c1331a7a49",
    scryfall_id = "d4d379b5-7f56-4a7d-a4ac-131fc3d579c6",
    faces = &[
        face!(
            name = "Primal Amulet",
            mana_cost = mana!("{4}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Primal Wellspring",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "instant and sorcery cost reduction has no Modifier, the four-counter transform needs a transform in place (#206) and an effect that removes counters, and Primal Wellspring's mana has no spend rider that copies the spell"
    ),
    abilities = &[triggered!(
        Trigger::SpellCast(&f!(your INSTANT_OR_SORCERY)),
        &[Effect::AddCounter {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }]
    )],
);
