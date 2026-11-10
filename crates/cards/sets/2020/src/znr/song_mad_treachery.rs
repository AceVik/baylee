//! Song-Mad Treachery // Song-Mad Ruins — {3}{R}{R} — Sorcery // Land
//! Oracle: Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {R}.
//! Set: ZNR #165 — Zendikar Rising | Scryfall ID: 782ca27f-9f18-476c-b582-89c06fb2e322 | Oracle ID: 81b61770-2ed5-4a50-84d0-97790002fc5a
//! Face: Song-Mad Treachery — {3}{R}{R} — Sorcery
//! Face: Song-Mad Ruins —  — Land
// IMPLEMENTED — Act of Treason's three clauses as one spell: a layer-2
// `GainControl` on the target until end of turn, an untap of the same
// creature, and haste from the pump that follows; the back face is a
// tapland (`EnterModifier::Tapped`) with its own `{T}: Add {R}`.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SONG_MAD_TREACHERY,
    oracle_id = "81b61770-2ed5-4a50-84d0-97790002fc5a",
    scryfall_id = "782ca27f-9f18-476c-b582-89c06fb2e322",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[
        face!(
            name = "Song-Mad Treachery",
            mana_cost = mana!("{3}{R}{R}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Song-Mad Ruins",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
        ),
    ],
    abilities = &[spell!(
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::GainControl,
                Duration::UntilEndOfTurn
            ),
            Effect::UntapTarget,
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
