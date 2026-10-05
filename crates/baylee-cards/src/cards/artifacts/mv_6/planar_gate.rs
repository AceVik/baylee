//! Planar Gate — {6} — Artifact
//! Oracle: Creature spells you cast cost {2} less to cast.
//! Set: ME4 #221 — Masters Edition IV | Scryfall ID: c880c236-a9d9-4e0d-bc4b-8910c75639cd | Oracle ID: 632f1034-d2ad-432a-a79a-4bd1307be24a
// PARTIAL — the static cost reducer is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_GATE,
    oracle_id = "632f1034-d2ad-432a-a79a-4bd1307be24a",
    scryfall_id = "c880c236-a9d9-4e0d-bc4b-8910c75639cd",
    faces = &[face!(
        name = "Planar Gate",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "creature spells you cast cost {2} less to cast has no Modifier that \
         reduces a cost"
    ),
    // NOT SUPPORTED: "Creature spells you cast cost {2} less to cast." — no
    // `Modifier` reduces a spell's cost (`Modifier::SpellsCostMore` is the
    // only cost modifier and it raises one), and `CostReduction` is read off
    // the spell being cast itself, never off another permanent's static
    // ability.
);
