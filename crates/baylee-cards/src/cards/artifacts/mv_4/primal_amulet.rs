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
// NotStartingPlayer and is read by nothing.
// NOT SUPPORTED: "Then if there are four or more charge counters on it, you
// may remove those counters and transform it." — no effect branches on a
// counter threshold (`Effect::IfNoCountersOnSelf` is the zero comparison
// only), nothing removes counters from the source as an effect, and the one
// transform shape, `Effect::ExileSelfReturnAsFace`, is out of reach behind
// both.
// NOT SUPPORTED: "When that mana is spent to cast an instant or sorcery
// spell, copy that spell…" — `SpendRider` has no copy arm, and pool mana
// carries no provenance.

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
        "instant/sorcery cost reduction, the four-counter transform clause and the back face's mana copy rider are not expressible"
    ),
    abilities = &[triggered!(
        Trigger::SpellCast(&f!(your INSTANT_OR_SORCERY)),
        &[Effect::AddCounter {
            kind: CounterKind::Charge,
            amount: Amount::Fixed(1),
        }]
    )],
);
