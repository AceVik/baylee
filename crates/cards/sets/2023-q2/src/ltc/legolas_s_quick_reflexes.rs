//! Legolas's Quick Reflexes — {G} — Instant
//! Oracle: Split second (As long as this spell is on the stack, players can't cast spells or activate abilities that aren't mana abilities.)
//! Oracle: Untap target creature. Until end of turn, it gains reach, hexproof, and "Whenever this creature becomes tapped, it deals damage equal to its power to up to one target creature."
//! Set: LTC #493 — Tales of Middle-earth Commander | Scryfall ID: 851c0167-04ba-4d15-b0fa-c211bd8826f1 | Oracle ID: 59154c04-6bc5-4b95-af1f-6de5fed05649
use baylee_cards_dsl::prelude::*;

card!(
    index = index::LEGOLAS_S_QUICK_REFLEXES,
    oracle_id = "59154c04-6bc5-4b95-af1f-6de5fed05649",
    scryfall_id = "851c0167-04ba-4d15-b0fa-c211bd8826f1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::SPLIT_SECOND,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Legolas's Quick Reflexes",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::ModifyTargets {
                modifier: Modifier::AddKeyword(KeywordSet::REACH.union(KeywordSet::HEXPROOF)),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::ModifyTargets {
                modifier: Modifier::GrantTriggeredUpToOne {
                    trigger: Trigger::BecomesTapped(&Filter::This),
                    effects: &[Effect::DealDamage {
                        amount: Amount::SourcePower,
                        target: TargetSpec::Object(&Filter::CREATURE),
                    }],
                    target: TargetSpec::Object(&Filter::CREATURE),
                },
                duration: Duration::UntilEndOfTurn,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
