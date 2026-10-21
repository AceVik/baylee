//! Musician — {2}{U} — Creature — Human Wizard
//! Oracle: Cumulative upkeep {1} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: {T}: Put a music counter on target creature. If it doesn't have "At the beginning of your upkeep, destroy this creature unless you pay {1} for each music counter on it," it gains that ability.
//! Set: ME2 #56 — Masters Edition II | Scryfall ID: 66d265d0-9237-4774-bfbf-d8771a583398 | Oracle ID: 0d6e9468-adb2-4feb-87cf-9113db8717b1
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: No Filter tests whether a creature already has the granted upkeep ability, so Musician checks whether the target lacks a music counter before putting one on it; if all music counters are later removed from that creature and Musician targets it again, it gains a duplicate upkeep trigger.
card!(
    index = index::MUSICIAN,
    oracle_id = "0d6e9468-adb2-4feb-87cf-9113db8717b1",
    scryfall_id = "66d265d0-9237-4774-bfbf-d8771a583398",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "checks whether the target lacks a music counter rather than whether it already has the upkeep ability, so removing all music counters and targeting it again grants a second upkeep trigger",
    ),
    faces = &[face!(
        name = "Musician",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(3),
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
                Effect::PlayerMayPayOr {
                    player: PlayerRel::You,
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: &Effect::SacrificeSelf,
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        activated!(
            cost!(TapSelf),
            &[
                Effect::IfTargetMatches {
                    filter: &Filter::Not(&Filter::HasCounter(counters::MUSIC)),
                    then: &[Effect::continuous(
                        &Filter::Reference,
                        Modifier::GrantTriggered {
                            trigger: Trigger::StepBegin {
                                step: StepKind::Upkeep,
                                whose: PlayerRel::You,
                            },
                            effects: &[Effect::PlayerMayPayOr {
                                player: PlayerRel::You,
                                mana: Amount::CountersOnSource(counters::MUSIC),
                                effect: &Effect::destroy(TargetSpec::ThisObject),
                            }],
                            target: None,
                        },
                        Duration::Indefinitely,
                    )],
                },
                Effect::AddCounter {
                    kind: counters::MUSIC,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
