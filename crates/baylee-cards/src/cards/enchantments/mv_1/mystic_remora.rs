//! Mystic Remora — {U} — Enchantment
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Whenever an opponent casts a noncreature spell, you may draw a card unless that player pays {4}.
//! Set: DMR #59 — Dominaria Remastered | Scryfall ID: 40140991-cffa-4b52-9a25-37e9a8aa9ddd | Oracle ID: 8a52f3c0-2552-4425-b2e3-5496eb2232a7
// IMPLEMENTED — cumulative upkeep {1} as the triggered ability CR 702.24a
// says it means, and the opponent-cast tax asked of the player who cast.

use baylee_cards_dsl::prelude::*;

// "an opponent casts a noncreature spell" — the spell, whose controller is
// an opponent of this enchantment's controller.
static OPPONENT_NONCREATURE_SPELL: Filter = f!(opponents NONCREATURE);

card!(
    index = index::MYSTIC_REMORA,
    oracle_id = "8a52f3c0-2552-4425-b2e3-5496eb2232a7",
    scryfall_id = "40140991-cffa-4b52-9a25-37e9a8aa9ddd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Mystic Remora",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        // Cumulative upkeep {1} (CR 702.24a): "At the beginning of your
        // upkeep, if this permanent is on the battlefield, put an age
        // counter on this permanent. Then you may pay [cost] for each age
        // counter on it. If you don't, sacrifice it."
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
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf,
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield
            ))),
        ),
        // "…unless that player pays {4}": the player who cast the spell.
        triggered!(
            Trigger::SpellCast(&OPPONENT_NONCREATURE_SPELL),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfEventObject,
                mana: Amount::Fixed(4),
                effect: &Effect::MayDo {
                    effects: &[Effect::draw(1)]
                },
            }]
        ),
    ],
);
