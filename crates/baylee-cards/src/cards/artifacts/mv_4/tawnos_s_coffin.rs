//! Tawnos's Coffin — {4} — Artifact
//! Oracle: You may choose not to untap this artifact during your untap step.
//! Oracle: {3}, {T}: Exile target creature and all Auras attached to it. Note the number and kind of counters that were on that creature. When this artifact leaves the battlefield or becomes untapped, return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that permanent.
//! Set: ME1 #169 — Masters Edition | Scryfall ID: 286fcfbe-296d-4b24-92d5-a06b3d0437d5 | Oracle ID: 05f20087-416f-4928-a0ab-6d0d2ca2ad05
// PARTIAL — the untap-step choice is built; the exile-and-return ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TAWNOS_S_COFFIN,
    oracle_id = "05f20087-416f-4928-a0ab-6d0d2ca2ad05",
    scryfall_id = "286fcfbe-296d-4b24-92d5-a06b3d0437d5",
    faces = &[face!(
        name = "Tawnos's Coffin",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "the exile sentence needs three things the DSL does not have: an \
         ExileUntil that ends when the source becomes untapped, a way to note \
         the counters that were on the exiled creature for its return, and a \
         way to return the exiled Auras attached"
    ),
    // NOT SUPPORTED: "{3}, {T}: Exile target creature and all Auras attached
    // to it. Note the number and kind of counters that were on that creature.
    // When this artifact leaves the battlefield or becomes untapped, return
    // that exiled card to the battlefield under its owner's control tapped
    // with the noted number and kind of counters on it. If you do, return the
    // other exiled cards to the battlefield under their owner's control
    // attached to that permanent." — `Effect::ExileLinked` and
    // `ReturnLinkedToBattlefield` are the nearest vocabulary, but `ExileUntil`
    // has no "or becomes untapped", the return is untapped with nothing on the
    // card, nothing notes counters for it, and nothing re-attaches the exiled
    // Auras. Exiling now and returning untapped later is a different card, so
    // the ability comes off whole.
    abilities = &[static_ability!(Filter::This, Modifier::MayChooseNotToUntap)],
);
