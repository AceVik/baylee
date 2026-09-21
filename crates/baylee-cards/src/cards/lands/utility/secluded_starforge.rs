//! Secluded Starforge — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Tap X untapped artifacts you control: Target creature gets +X/+0 until end of turn. Activate only as a sorcery.
//! Oracle: {5}, {T}: Create a 2/2 colorless Robot artifact creature token.
//! Set: EOE #257 — Edge of Eternities | Scryfall ID: a997ff9f-045a-44a2-983d-f36414cef1ab | Oracle ID: 69f55a7c-6ddf-412e-b63b-b395731a1ff2
// PARTIAL — {T}: Add {C} is built; the other two lines come off the card,
// each with a `// NOT SUPPORTED:` saying what the vocabulary cannot say.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SECLUDED_STARFORGE,
    oracle_id = "69f55a7c-6ddf-412e-b63b-b395731a1ff2",
    scryfall_id = "a997ff9f-045a-44a2-983d-f36414cef1ab",
    faces = &[face!(name = "Secluded Starforge", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the \"Tap X untapped artifacts you control\" cost has no spelling \
         (CostPart::TapOther names one permanent by a filter and carries no \
         count), and the pool's token registry has no 2/2 colorless Robot \
         artifact creature token for the last ability to create",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}, {T}, Tap X untapped artifacts you control:
        // Target creature gets +X/+0 until end of turn. Activate only as a
        // sorcery." — the cost cannot be said. CostPart::TapOther takes a
        // filter and names exactly one permanent; no cost part carries a
        // count, so writing it would ship "tap one artifact" beside a pump
        // reading an X that no activation ever announced.
        //
        // NOT SUPPORTED: "{5}, {T}: Create a 2/2 colorless Robot artifact
        // creature token." — a token is made from a TokenDef in the pool's
        // registry, and a card file may not define one of its own: a literal
        // written here has no id, so the token would reach the table with no
        // identity for a client to draw. No registry constant exists for it
        // yet.
    ],
);
