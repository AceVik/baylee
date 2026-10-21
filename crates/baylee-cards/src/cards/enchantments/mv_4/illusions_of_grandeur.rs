//! Illusions of Grandeur — {3}{U} — Enchantment
//! Oracle: Cumulative upkeep {2} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: When this enchantment enters, you gain 20 life.
//! Oracle: When this enchantment leaves the battlefield, you lose 20 life.
//! Set: ME1 #40 — Masters Edition | Scryfall ID: 44c20ed5-7064-436d-ad76-85a1d6bf0103 | Oracle ID: d7c81b3e-8082-4aaf-8979-2717d118234c

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUSIONS_OF_GRANDEUR,
    oracle_id = "d7c81b3e-8082-4aaf-8979-2717d118234c",
    scryfall_id = "44c20ed5-7064-436d-ad76-85a1d6bf0103",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Illusions of Grandeur",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
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
        triggered!(Trigger::ETB, &[Effect::gain_life(20)]),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::This),
            &[Effect::LoseLife {
                amount: Amount::Fixed(20),
                target: PlayerRel::You,
            }],
        ),
    ],
);
