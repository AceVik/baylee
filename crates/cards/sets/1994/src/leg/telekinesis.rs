//! Telekinesis — {U}{U} — Instant
//! Oracle: Tap target creature. Prevent all combat damage that would be dealt by that creature this turn. It doesn't untap during its controller's next two untap steps.
//! Set: ME1 #52 — Masters Edition | Scryfall ID: d63f7944-5738-4a13-8f84-6813c188ac3a | Oracle ID: 1b9dd2b6-d14d-4c1e-9885-00ab2c0bf8da
use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEKINESIS,
    oracle_id = "1b9dd2b6-d14d-4c1e-9885-00ab2c0bf8da",
    scryfall_id = "d63f7944-5738-4a13-8f84-6813c188ac3a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Telekinesis",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            ),
            Effect::SuppressUntap {
                target: TargetSpec::Object(&Filter::CREATURE),
                steps: 2
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    ),],
);
