//! Remove Enchantments — {W} — Instant
//! Oracle: Return to your hand all enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control. Then destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.
//! Set: LEG #33 — Legends | Scryfall ID: bf2e3a8a-b386-474d-b8e9-4c2d56a2b742 | Oracle ID: fa879d99-3d2b-4a9a-a17f-d5ac109f8f44
// PARTIAL — the non-Aura halves of both sentences are built: your
// enchantments are returned, then the ones you control but do not own are
// destroyed. The Aura clauses need an attachment relation no Filter carries.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REMOVE_ENCHANTMENTS,
    oracle_id = "fa879d99-3d2b-4a9a-a17f-d5ac109f8f44",
    scryfall_id = "bf2e3a8a-b386-474d-b8e9-4c2d56a2b742",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the Aura clauses of both sentences need an \"attached to a permanent \
         you control / attacking creature your opponents control\" filter, \
         and only Filter::AttachedToBySource (the effect's own host) and \
         Filter::IsAttached (attached to anything) exist; the non-Aura \
         enchantment clauses are built"
    ),
    faces = &[face!(
        name = "Remove Enchantments",
        mana_cost = mana!("{W}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "all Auras you own attached to permanents you control,
    // and all Auras you own attached to attacking creatures your opponents
    // control" — no `Filter` can say what a permanent is attached to:
    // `Filter::AttachedToBySource` matches only the source's own host and
    // `Filter::IsAttached` only that it is attached to something, so the
    // attachment half of the return clause cannot be named.
    // NOT SUPPORTED: "all other Auras attached to permanents you control, and
    // all other Auras attached to attacking creatures your opponents
    // control" — the same missing attachment filter, so the destroy clause
    // stops at the enchantments it can name.
    abilities = &[spell!(&[
        Effect::ReturnAllToHand {
            filter: &Filter::And(&[
                Filter::ENCHANTMENT,
                Filter::OwnedByYou,
                Filter::ControlledByYou,
            ]),
            opponents_only: false,
        },
        Effect::destroy_all(&Filter::And(&[
            Filter::ENCHANTMENT,
            Filter::ControlledByYou,
            Filter::Not(&Filter::OwnedByYou),
        ])),
    ])],
);
