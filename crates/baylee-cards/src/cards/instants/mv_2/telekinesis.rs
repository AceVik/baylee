//! Telekinesis — {U}{U} — Instant
//! Oracle: Tap target creature. Prevent all combat damage that would be dealt by that creature this turn. It doesn't untap during its controller's next two untap steps.
//! Set: ME1 #52 — Masters Edition | Scryfall ID: d63f7944-5738-4a13-8f84-6813c188ac3a | Oracle ID: 1b9dd2b6-d14d-4c1e-9885-00ab2c0bf8da
// PARTIAL — the tap and the combat-damage prevention are built; the
// two-untap-step clause has no duration and is left off.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEKINESIS,
    oracle_id = "1b9dd2b6-d14d-4c1e-9885-00ab2c0bf8da",
    scryfall_id = "d63f7944-5738-4a13-8f84-6813c188ac3a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Duration::UntilYourNextUntapStep is the only untap-step duration: it \
         suppresses one untap step of the effect's controller, where the card \
         says the creature's controller's next two untap steps, so the third \
         sentence is not written"
    ),
    faces = &[face!(
        name = "Telekinesis",
        mana_cost = mana!("{U}{U}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "It doesn't untap during its controller's next two
    // untap steps." — `Duration::UntilYourNextUntapStep` is the only
    // untap-step duration in the DSL: it suppresses one step, of the
    // effect's controller rather than the affected creature's controller,
    // and the card asks for two of its controller's steps.
    abilities = &[spell!(
        &[
            Effect::TapTarget,
            Effect::continuous(
                &Filter::This,
                Modifier::PreventDamageFromIt,
                Duration::UntilEndOfTurn
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
