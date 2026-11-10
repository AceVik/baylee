//! Mystic Might — {U} — Enchantment — Aura
//! Oracle: Enchant land you control
//! Oracle: Cumulative upkeep {1}{U} (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)
//! Oracle: Enchanted land has "{T}: Target creature gets +2/+2 until end of turn."
//! Set: ICE #86 — Ice Age | Scryfall ID: e35d7f08-0687-41bd-8c53-31a49adabb11 | Oracle ID: 967eca77-d273-4fc6-97d1-df7d8cc6ff59
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static GRANTED: AbilityDef = activated!(
    cost!(TapSelf),
    &[Effect::PumpTarget {
        power: Amount::Fixed(2),
        toughness: Amount::Fixed(2),
        keywords: KeywordSet::EMPTY,
        duration: Duration::UntilEndOfTurn,
    }],
    target = Some(TargetSpec::Object(&Filter::CREATURE))
);

card!(
    index = index::MYSTIC_MIGHT,
    oracle_id = "967eca77-d273-4fc6-97d1-df7d8cc6ff59",
    scryfall_id = "e35d7f08-0687-41bd-8c53-31a49adabb11",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mystic Might",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND)))
        ),
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
                    each: mana!("{1}{U}"),
                    times: Amount::CountersOnSource(counters::AGE),
                    paid: &[],
                    unpaid: &[Effect::SacrificeSelf],
                },
            ],
            condition = Some(Condition::SourceMatches(&Filter::InZone(
                ZoneRef::Battlefield,
            ))),
        ),
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&GRANTED)
        ),
    ],
);
