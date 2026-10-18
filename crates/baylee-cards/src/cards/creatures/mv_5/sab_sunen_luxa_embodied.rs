//! Sab-Sunen, Luxa Embodied — {3}{G}{U} — Legendary Creature — God
//! Oracle: Reach, trample, indestructible
//! Oracle: Sab-Sunen can't attack or block unless it has an even number of counters on it. (Zero is even.)
//! Oracle: At the beginning of your first main phase, put a +1/+1 counter on Sab-Sunen. Then if it has an odd number of counters on it, draw two cards.
//! Set: DFT #221 — Aetherdrift | Scryfall ID: 2ef555b1-666d-4386-8983-0e88f9b6cdec | Oracle ID: 07e15387-f346-4fa0-b556-d1c52d4acc07
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ODD_COUNTERS_ON_SELF: Condition = Condition::Not(&Condition::EvenCountersOnSelf);

card!(
    index = index::SAB_SUNEN_LUXA_EMBODIED,
    oracle_id = "07e15387-f346-4fa0-b556-d1c52d4acc07",
    scryfall_id = "2ef555b1-666d-4386-8983-0e88f9b6cdec",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::REACH
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::INDESTRUCTIBLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sab-Sunen, Luxa Embodied",
        mana_cost = mana!("{3}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GOD],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)),
            condition = Some(ODD_COUNTERS_ON_SELF),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::FirstMain,
                whose: PlayerRel::You,
            },
            &[
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::IfCondition {
                    condition: ODD_COUNTERS_ON_SELF,
                    then: &[Effect::draw(2)],
                    otherwise: &[],
                },
            ],
        ),
    ],
);
