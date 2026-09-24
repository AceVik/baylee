//! Wishclaw Talisman — {1}{B} — Artifact
//! Oracle: This artifact enters with three wish counters on it.
//! Oracle: {1}, {T}, Remove a wish counter from this artifact: Search your library for a card, put it into your hand, then shuffle. An opponent gains control of this artifact. Activate only during your turn.
//! Set: FDN #617 — Foundations | Scryfall ID: 69d0f5bd-ccea-49b2-bd79-ad5e4d850cf5 | Oracle ID: 81c70ae7-3c18-4c9b-8505-e4db9e0e6518
// PARTIAL — the three wish counters on entry, the tutor and the hand-over
// are built; the printed "Activate only during your turn" is not (see the
// NOT SUPPORTED line above the ability).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WISHCLAW_TALISMAN,
    oracle_id = "81c70ae7-3c18-4c9b-8505-e4db9e0e6518",
    scryfall_id = "69d0f5bd-ccea-49b2-bd79-ad5e4d850cf5",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Wishclaw Talisman",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ARTIFACT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::WISH,
            amount: Amount::Fixed(3),
        }],
    ),],
    coverage = Coverage::Partial(
        "\"Activate only during your turn\" has no spelling — no ActivationTiming and no \
         Condition reads whose turn it is; the opponent that gains control is auto-chosen \
         heads-up (player choice is M3)",
    ),
    // NOT SUPPORTED: "Activate only during your turn." — `activated!` offers
    // InstantSpeed and SorcerySpeed and nothing in between, and no
    // `Condition` asks whose turn it is, so the ability is offered on every
    // turn. SorcerySpeed is a stricter, different card and is not the
    // printed sentence.
    abilities = &[activated!(
        cost!(
            "{1}",
            TapSelf,
            RemoveCounterSelf {
                kind: counters::WISH,
                n: 1
            }
        ),
        &[
            Effect::SearchLibrary {
                filter: &Filter::Any,
                finds: &[Find::HAND],
                optional: false,
            },
            Effect::ChangeController {
                new_controller: PlayerRel::Opponent,
            },
        ]
    )],
);
