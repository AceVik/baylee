//! Tidal Control — {1}{U}{U} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Pay 2 life or {2}: Counter target red or green spell. Any player may activate this ability.
//! Set: ALL #40 — Alliances | Scryfall ID: cb9a7b7d-3d37-4bb6-ab48-1fec2bfb4fdc | Oracle ID: 855e200e-5375-4aac-b1f7-5113162e7e14

use baylee_cards_dsl::prelude::*;

static RED_OR_GREEN_SPELL: Filter =
    Filter::HasColor(ColorSet::from_slice(&[Color::Red, Color::Green]));

card!(
    index = index::TIDAL_CONTROL,
    oracle_id = "855e200e-5375-4aac-b1f7-5113162e7e14",
    scryfall_id = "cb9a7b7d-3d37-4bb6-ab48-1fec2bfb4fdc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Tidal Control",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Partial(
        "Cumulative upkeep {2} and countering target red or green spell by paying either 2 life or {2} are implemented; only Tidal Control's controller may activate the ability rather than any player."
    ),
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounter {
                    kind: counters::AGE,
                    amount: Amount::Fixed(1),
                },
                Effect::PlayerMayPayManaEachThenElse {
                    player: PlayerRel::You,
                    each: mana!("{2}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        activated!(
            cost!(PayLife(2)),
            &[Effect::CounterTargetSpell],
            target = Some(TargetSpec::Spell(&RED_OR_GREEN_SPELL))
        ),
        activated!(
            cost!("{2}"),
            &[Effect::CounterTargetSpell],
            target = Some(TargetSpec::Spell(&RED_OR_GREEN_SPELL))
        ),
    ],
);
