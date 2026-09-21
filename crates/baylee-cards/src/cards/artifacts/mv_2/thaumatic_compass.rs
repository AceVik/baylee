//! Thaumatic Compass // Spires of Orazca — {2} — Artifact // Land
//! Oracle: {3}, {T}: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.
//! Oracle: At the beginning of your end step, if you control seven or more lands, transform this artifact.
//! Oracle: (Transforms from Thaumatic Compass.)
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Untap target attacking creature an opponent controls and remove it from combat.
//! Set: XLN #249 — Ixalan | Scryfall ID: 392af78e-34d5-4b1b-8b29-0e702271e4d7 | Oracle ID: f9085e55-2833-41b7-9100-a35dc04dee93
//! Face: Thaumatic Compass — {2} — Artifact
//! Face: Spires of Orazca —  — Land
// PARTIAL — the front face's land search and its seven-or-more-lands end-step
// flip (Effect::ExileSelfReturnAsFace) are built, and so is Spires of
// Orazca's {T}: Add {C}. The back face's second ability is not; see the
// NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::THAUMATIC_COMPASS,
    oracle_id = "f9085e55-2833-41b7-9100-a35dc04dee93",
    scryfall_id = "392af78e-34d5-4b1b-8b29-0e702271e4d7",
    faces = &[
        face!(
            name = "Thaumatic Compass",
            mana_cost = mana!("{2}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Spires of Orazca",
            types = TypeSet::LAND,
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
        ),
    ],
    coverage = Coverage::Partial(
        "Spires of Orazca's \"{T}: Untap target attacking creature an opponent \
         controls and remove it from combat\": no effect takes a creature out of \
         combat, so the ability is off the card rather than shipped as an untap \
         that leaves the attacker attacking"
    ),
    abilities = &[
        activated!(
            cost!("{3}", TapSelf),
            &[Effect::SearchLibrary {
                filter: &Filter::BASIC_LAND,
                finds: &[Find::HAND],
                optional: false,
            }]
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::End,
                whose: PlayerRel::You,
            },
            &[Effect::ExileSelfReturnAsFace { face: 1 }],
            condition = Some(Condition::ControlCount(&Filter::LAND, 7)),
        ),
    ],
);

// NOT SUPPORTED: Spires of Orazca — "{T}: Untap target attacking creature an
// opponent controls and remove it from combat". Effect::UntapTarget says the
// first half and nothing says the second — PhaseOut and ExileAndReturnAtEndStep
// are different sentences — and half of that ability (the untap, with the
// creature still in combat) is a card that looks like it saves you and does
// not, so the ability comes off whole.
