//! Rule tests for source-color, chosen-source, matching-source, and per-source
//! damage prevention shields (CR 615.1, CR 615.7, CR 609.7a–b, CR 400.7).
#![allow(clippy::too_many_lines)]

use super::*;
use baylee_cards_dsl::{
    Amount, Duration, EnterModifier, Filter, Layer, Modifier, TargetSpec, counters::AGE,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::creature as creature_type;
use baylee_core::ids::Defender;
use baylee_core::mana::ManaColor;

const STORY_SHIELD: u32 = 9500;
const RED_PINGER: u32 = 9501;
const BLUE_PINGER: u32 = 9502;
const DESTROY_ENCH: u32 = 9503;
const BLINK_PERM: u32 = 9504;
const PAINT_BLUE: u32 = 9505;
const PILGRIM_SHIELD: u32 = 9506;
const ADVOCATE_SHIELD: u32 = 9507;
const RED_SWEEPER: u32 = 9508;
const SOLACE_SHIELD: u32 = 9509;
const SMALL_GOBLIN: u32 = 9510;
const BIG_GOBLIN: u32 = 9511;
const ELF_ATTACKER: u32 = 9512;
const STRANDS_SPELL: u32 = 9513;
const COVER_WINTER: u32 = 9514;
const CLERIC_GUARD: u32 = 9515;
const CLERIC_ALLY: u32 = 9516;
const TRAMPLER: u32 = 9517;

static RED_SOURCE: Filter = Filter::HasColor(ColorSet::of(Color::Red));
static CHOSEN_TYPE_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::MatchesChosenTypeOfSource]);
static YOUR_CLERIC: Filter = Filter::And(&[
    Filter::YOUR_CREATURE,
    Filter::HasSubtype(creature_type::CLERIC),
]);

static STORY_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PreventNextFromChosenSource {
        sources: &Filter::OfSourceChosenColor,
        combat_only: false,
        all_but: 0,
        gain_life: false,
    }],
    None,
)];

static PING_PLAYER: &[AbilityDef] = &[free(
    &[Effect::DealDamage {
        amount: Amount::Fixed(3),
        target: TargetSpec::AnyPlayer,
    }],
    Some(TargetReq::one(TargetSpec::AnyPlayer)),
)];

static PING_ANY: &[AbilityDef] = &[free(
    &[Effect::DealDamage {
        amount: Amount::Fixed(2),
        target: TargetSpec::PlayerOrObject(&Filter::CREATURE),
    }],
    Some(TargetReq::one(TargetSpec::PlayerOrObject(
        &Filter::CREATURE,
    ))),
)];

static SWEEP_TWO: &[AbilityDef] = &[free(
    &[
        Effect::DealDamageEach {
            amount: Amount::Fixed(2),
            filter: &Filter::CREATURE,
        },
        Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(baylee_cards_dsl::PlayerRel::EachPlayer),
        },
    ],
    None,
)];

static DESTROY_ENCHANTMENT_SPELL: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::destroy(TargetSpec::Object(&Filter::ENCHANTMENT))],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT))),
)];

static BLINK_ANY_SPELL: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::Blink {
        target: TargetSpec::Object(&Filter::Any),
        owner_control: false,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
)];

static PAINT_BLUE_SPELL: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::CreateContinuousEffect {
        layer: Layer::Color,
        filter: &Filter::This,
        modifier: Modifier::SetColor(ColorSet::of(Color::Blue)),
        duration: Duration::UntilEndOfTurn,
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::Any))),
)];

static PILGRIM_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PreventFromChosenSource {
        target: None,
        sources: &RED_SOURCE,
        next_only: true,
        combat_only: false,
        all_but: 0,
        gain_life: false,
    }],
    None,
)];

static ADVOCATE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PreventFromChosenSource {
        target: Some(TargetSpec::PlayerOrObject(&Filter::CREATURE)),
        sources: &Filter::Any,
        next_only: false,
        combat_only: false,
        all_but: 0,
        gain_life: false,
    }],
    Some(TargetReq::one(TargetSpec::PlayerOrObject(
        &Filter::CREATURE,
    ))),
)];

static SOLACE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PreventNextFromMatchingSources {
        target: Some(TargetSpec::Player(baylee_cards_dsl::PlayerRel::You)),
        sources: &CHOSEN_TYPE_CREATURE,
        combat_only: false,
    }],
    None,
)];

static STRANDS_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[
    Effect::ChooseColor,
    Effect::PreventAllDamageFromMatchingThisTurn {
        target: None,
        sources: &Filter::OfSourceChosenColor,
        combat_only: false,
    },
])];

static COVER_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::AddCounterSource {
            kind: AGE,
            amount: Amount::Fixed(2),
        }],
        None,
    ),
    baylee_cards_dsl::static_ability!(
        Filter::YOUR_CREATURE,
        Modifier::PreventDamagePerSource {
            sources: &Filter::CREATURE,
            include_you: true,
            combat_only: true,
            amount: Amount::CountersOnSource(AGE),
        }
    ),
];

static CLERIC_GUARD_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    YOUR_CLERIC,
    Modifier::PreventDamageToAffectedPerSource {
        sources: &Filter::Any,
        combat_only: false,
        amount: Amount::Fixed(1),
    }
)];

fn shield_cards() -> Vec<&'static CardDef> {
    vec![
        card(
            STORY_SHIELD,
            FaceDef {
                enter_modifiers: &[EnterModifier::ChooseColor],
                ..face("Story Shield", "{0}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            STORY_ABILITIES,
        ),
        card(
            RED_PINGER,
            FaceDef {
                color_indicator: ColorSet::of(Color::Red),
                ..creature_face("Red Pinger", "{0}", 2, 2)
            },
            KeywordSet::HASTE,
            PING_PLAYER,
        ),
        card(
            BLUE_PINGER,
            FaceDef {
                color_indicator: ColorSet::of(Color::Blue),
                ..creature_face("Blue Pinger", "{0}", 2, 2)
            },
            KeywordSet::HASTE,
            PING_ANY,
        ),
        card(
            DESTROY_ENCH,
            face("Destroy Ench", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            DESTROY_ENCHANTMENT_SPELL,
        ),
        card(
            BLINK_PERM,
            face("Blink Perm", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            BLINK_ANY_SPELL,
        ),
        card(
            PAINT_BLUE,
            face("Paint Blue", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            PAINT_BLUE_SPELL,
        ),
        card(
            PILGRIM_SHIELD,
            creature_face("Pilgrim Shield", "{0}", 1, 2),
            KeywordSet::EMPTY,
            PILGRIM_ABILITIES,
        ),
        card(
            ADVOCATE_SHIELD,
            creature_face("Advocate Shield", "{0}", 1, 3),
            KeywordSet::EMPTY,
            ADVOCATE_ABILITIES,
        ),
        card(
            RED_SWEEPER,
            FaceDef {
                color_indicator: ColorSet::of(Color::Red),
                ..creature_face("Red Sweeper", "{0}", 2, 3)
            },
            KeywordSet::EMPTY,
            SWEEP_TWO,
        ),
        card(
            SOLACE_SHIELD,
            FaceDef {
                enter_modifiers: &[EnterModifier::ChooseSubtype],
                ..face("Solace Shield", "{0}", TypeSet::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            SOLACE_ABILITIES,
        ),
        card(
            SMALL_GOBLIN,
            FaceDef {
                subtypes: &[creature_type::GOBLIN],
                ..creature_face("Small Goblin", "{0}", 1, 1)
            },
            KeywordSet::HASTE,
            &[],
        ),
        card(
            BIG_GOBLIN,
            FaceDef {
                subtypes: &[creature_type::GOBLIN],
                ..creature_face("Big Goblin", "{0}", 3, 3)
            },
            KeywordSet::HASTE,
            &[],
        ),
        card(
            ELF_ATTACKER,
            FaceDef {
                subtypes: &[creature_type::ELF],
                ..creature_face("Elf Attacker", "{0}", 2, 2)
            },
            KeywordSet::HASTE,
            &[],
        ),
        card(
            STRANDS_SPELL,
            face("Strands Spell", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            STRANDS_ABILITIES,
        ),
        card(
            COVER_WINTER,
            face("Cover Winter", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            COVER_ABILITIES,
        ),
        card(
            CLERIC_GUARD,
            FaceDef {
                subtypes: &[creature_type::CLERIC],
                ..creature_face("Cleric Guard", "{0}", 2, 4)
            },
            KeywordSet::EMPTY,
            CLERIC_GUARD_ABILITIES,
        ),
        card(
            CLERIC_ALLY,
            FaceDef {
                subtypes: &[creature_type::CLERIC],
                ..creature_face("Cleric Ally", "{0}", 2, 3)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            TRAMPLER,
            creature_face("Trampler", "{0}", 5, 5),
            KeywordSet::HASTE.union(KeywordSet::TRAMPLE),
            &[],
        ),
    ]
}

#[test]
fn story_circle_chosen_color_survives_destruction_in_response_blink_and_checks_damage_time_color() {
    let mut engine = bench(
        9501,
        shield_cards(),
        [
            Seat::with(&[])
                .holding(&[STORY_SHIELD, STORY_SHIELD, BLINK_PERM])
                .at(20),
            Seat::with(&[RED_PINGER, BLUE_PINGER])
                .holding(&[DESTROY_ENCH, PAINT_BLUE])
                .at(20),
        ],
    );
    to_main(&mut engine, me());
    let red_pinger = the(&engine, ZoneLocation::Battlefield, RED_PINGER);

    // Cast first Story Shield, choosing Red as it enters.
    let story_in_hand = objects(&engine, ZoneLocation::Hand(me()), STORY_SHIELD)[0];
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: story_in_hand,
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    settle(&mut engine, me());

    let story = the(&engine, ZoneLocation::Battlefield, STORY_SHIELD);
    // Activate Story Shield, and destroy it in response BEFORE it resolves!
    activate(&mut engine, me(), story, 0);
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let destroy = the(&engine, ZoneLocation::Hand(them()), DESTROY_ENCH);
    engine
        .apply(them(), PlayerAction::CastSpell { card: destroy })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![story],
                players: vec![],
            },
        )
        .unwrap();

    // Despite Story Shield being in the graveyard, its activated ability still
    // remembers `chosen_color == Some(Red)` from its stack source incarnation!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].object, red_pinger);
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: options[0],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // Red Pinger deals 3 to me -> prevented! Second 3 -> NOT prevented (17 life).
    for _ in 0..2 {
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
        activate(&mut engine, them(), red_pinger, 0);
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![me()],
                },
            )
            .unwrap();
        settle(&mut engine, me());
    }
    assert_eq!(life(&engine, me()), 17);

    // Cast second Story Shield choosing Red, activate it on Red Pinger, then
    // blink Story Shield and choose Blue on the new incarnation!
    let second_story_hand = the(&engine, ZoneLocation::Hand(me()), STORY_SHIELD);
    engine
        .apply(
            me(),
            PlayerAction::CastSpell {
                card: second_story_hand,
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    settle(&mut engine, me());

    let story2 = the(&engine, ZoneLocation::Battlefield, STORY_SHIELD);
    activate(&mut engine, me(), story2, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: options[0],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    let blink = the(&engine, ZoneLocation::Hand(me()), BLINK_PERM);
    engine
        .apply(me(), PlayerAction::CastSpell { card: blink })
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![story2],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    settle(&mut engine, me());

    // Red Pinger deals 3 -> STILL prevented because the shield froze Red!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), red_pinger, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 17);

    // Activate Blue-blinked Story2 on Blue Pinger; wait, test CR 609.7b:
    // if Red Pinger had a Red shield and gets painted Blue, shield won't prevent!
    // Paint Blue Pinger? Better: activate Blue-blinked Story2 on Blue Pinger,
    // THEN paint Blue Pinger Red? Wait: `PAINT_BLUE` paints `red_pinger` Blue!
    // Let's test Blue-blinked Story2 choosing Blue Pinger (prevented -> 17),
    // AND test `PAINT_BLUE` turning Blue Pinger? No: paint `red_pinger` Blue
    // AFTER creating a Red shield? `story2` chose Blue now, so `options` has
    // ONLY `blue_pinger`, proving `story2`'s new incarnation reads Blue while
    // its earlier shield still read Red!
    let blue_pinger = the(&engine, ZoneLocation::Battlefield, BLUE_PINGER);
    activate(&mut engine, me(), story2, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].object, blue_pinger);
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: options[0],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), blue_pinger, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 17);

    // Also test CR 609.7b: if `red_pinger` is painted Blue (`PAINT_BLUE`), now
    // `story2` (whose chosen color is Blue) CAN choose `red_pinger`, and if
    // `story2` creates a shield on `red_pinger` while `red_pinger` is Blue, it prevents!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let paint = the(&engine, ZoneLocation::Hand(them()), PAINT_BLUE);
    engine
        .apply(them(), PlayerAction::CastSpell { card: paint })
        .unwrap();
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![red_pinger],
                players: vec![],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    activate(&mut engine, me(), story2, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.iter().any(|r| r.object == red_pinger));
    let painted_ref = options
        .into_iter()
        .find(|r| r.object == red_pinger)
        .unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: painted_ref,
            },
        )
        .unwrap();
    settle(&mut engine, me());
}

#[test]
fn general_chosen_source_shield_protects_everything_or_specific_target() {
    let mut engine = bench(
        9502,
        shield_cards(),
        [
            Seat::with(&[PILGRIM_SHIELD, ADVOCATE_SHIELD]).at(20),
            Seat::with(&[RED_SWEEPER, BLUE_PINGER]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let pilgrim = the(&engine, ZoneLocation::Battlefield, PILGRIM_SHIELD);
    let advocate = the(&engine, ZoneLocation::Battlefield, ADVOCATE_SHIELD);
    let sweeper = the(&engine, ZoneLocation::Battlefield, RED_SWEEPER);
    let blue = the(&engine, ZoneLocation::Battlefield, BLUE_PINGER);

    // 1) Pilgrim (`target: None`, `next_only: true`, `&RED_SOURCE`): choose `sweeper`.
    activate(&mut engine, me(), pilgrim, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].object, sweeper);
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: options[0],
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // `sweeper` deals 2 to ALL creatures and ALL players simultaneously -> ALL prevented!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), sweeper, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 20);
    assert_eq!(life(&engine, them()), 20);
    assert_eq!(engine.state().object(pilgrim).unwrap().damage, 0);

    // 2) Advocate (`target: Some(...)`, `next_only: false`): shield `pilgrim` from `blue` all turn.
    activate(&mut engine, me(), advocate, 0);
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![pilgrim],
                players: vec![],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    let blue_ref = options.into_iter().find(|r| r.object == blue).unwrap();
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageSource {
                choice,
                source: blue_ref,
            },
        )
        .unwrap();
    settle(&mut engine, me());

    // `blue` pings `pilgrim` (1/2) TWICE for 2 each -> both prevented (`next_only: false`)!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    for _ in 0..2 {
        activate(&mut engine, them(), blue, 0);
        engine
            .apply(
                them(),
                PlayerAction::ChooseTargets {
                    objects: vec![pilgrim],
                    players: vec![],
                },
            )
            .unwrap();
        settle(&mut engine, me());
        engine.apply(me(), PlayerAction::PassPriority).unwrap();
    }
    assert_eq!(engine.state().object(pilgrim).unwrap().damage, 0);

    // `blue` pings `me()` -> NOT prevented (shield only covered `pilgrim`)!
    activate(&mut engine, them(), blue, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 18);
}

#[test]
fn circle_of_solace_next_matching_prevents_one_chosen_goblin_in_simultaneous_combat() {
    let mut engine = bench(
        9503,
        shield_cards(),
        [
            Seat::with(&[]).holding(&[SOLACE_SHIELD]).at(20),
            Seat::with(&[SMALL_GOBLIN, BIG_GOBLIN, ELF_ATTACKER]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let solace_hand = the(&engine, ZoneLocation::Hand(me()), SOLACE_SHIELD);
    engine
        .apply(me(), PlayerAction::CastSpell { card: solace_hand })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(options.contains(&creature_type::GOBLIN));
    engine
        .apply(me(), PlayerAction::ChooseSubtype(creature_type::GOBLIN))
        .unwrap();
    settle(&mut engine, me());

    // Advance to opponent's main phase and activate Solace Shield.
    to_main(&mut engine, them());
    let solace = the(&engine, ZoneLocation::Battlefield, SOLACE_SHIELD);
    let small_goblin = the(&engine, ZoneLocation::Battlefield, SMALL_GOBLIN);
    let big_goblin = the(&engine, ZoneLocation::Battlefield, BIG_GOBLIN);
    let elf = the(&engine, ZoneLocation::Battlefield, ELF_ATTACKER);

    engine.apply(them(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, me(), solace, 0);
    settle(&mut engine, them());

    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (small_goblin, Defender::Player(me())),
                    (big_goblin, Defender::Player(me())),
                    (elf, Defender::Player(me())),
                ],
            },
        )
        .unwrap();

    // Unblocked simultaneous combat damage: Solace offers 2 options (1/1 Goblin
    // vs 3/3 Goblin), never the Elf!
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageEffect { .. })
    });
    let Pending::ChooseDamageEffect {
        choice, options, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options.len(), 2);
    let big_opt = options
        .iter()
        .find(|o| {
            matches!(
                o.kind,
                crate::choice::DamageEffectKind::PreventFromSource { source, .. }
                    if source == big_goblin
            )
        })
        .expect("offered Big Goblin");
    engine
        .apply(
            me(),
            PlayerAction::ChooseDamageEffect {
                choice,
                effect: big_opt.id,
            },
        )
        .unwrap();

    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
    // 3 from Big Goblin prevented; 1 (Small Goblin) + 2 (Elf) = 3 dealt -> 17 life, shield spent!
    assert_eq!(life(&engine, me()), 17);
    assert!(engine.state().shields.is_empty());
}

#[test]
fn prismatic_strands_choose_color_prevents_all_matching_including_later_arrivals() {
    let mut engine = bench(
        9504,
        shield_cards(),
        [
            Seat::with(&[]).holding(&[STRANDS_SPELL]).at(20),
            Seat::with(&[BLUE_PINGER]).holding(&[RED_PINGER]).at(20),
        ],
    );
    to_main(&mut engine, me());
    let strands = the(&engine, ZoneLocation::Hand(me()), STRANDS_SPELL);
    engine
        .apply(me(), PlayerAction::CastSpell { card: strands })
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(me(), PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    settle(&mut engine, me());

    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    let blue = the(&engine, ZoneLocation::Battlefield, BLUE_PINGER);
    activate(&mut engine, them(), blue, 0);
    engine
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 18, "Blue source is not prevented");

    // Now on opponent's main phase in a fresh game: cast Strands BEFORE opponent casts Red Pinger!
    let mut engine2 = bench(
        9505,
        shield_cards(),
        [
            Seat::with(&[]).holding(&[STRANDS_SPELL]).at(20),
            Seat::with(&[]).holding(&[RED_PINGER]).at(20),
        ],
    );
    to_main(&mut engine2, them());
    engine2.apply(them(), PlayerAction::PassPriority).unwrap();
    let strands2 = the(&engine2, ZoneLocation::Hand(me()), STRANDS_SPELL);
    engine2
        .apply(me(), PlayerAction::CastSpell { card: strands2 })
        .unwrap();
    walk_until(&mut engine2, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine2
        .apply(me(), PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    settle(&mut engine2, them());

    // Opponent casts Red Pinger AFTER Strands resolved (CR 611.2c)!
    let red_hand = the(&engine2, ZoneLocation::Hand(them()), RED_PINGER);
    engine2
        .apply(them(), PlayerAction::CastSpell { card: red_hand })
        .unwrap();
    settle(&mut engine2, them());
    let red_entered = the(&engine2, ZoneLocation::Battlefield, RED_PINGER);
    activate(&mut engine2, them(), red_entered, 0);
    engine2
        .apply(
            them(),
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![me()],
            },
        )
        .unwrap();
    settle(&mut engine2, them());
    assert_eq!(
        life(&engine2, me()),
        20,
        "Red creature entering after Prismatic Strands resolved still has all its damage prevented"
    );
}

#[test]
fn cover_of_winter_and_daunting_defender_prevent_per_source_and_allocate_trample() {
    let mut engine = bench(
        9506,
        shield_cards(),
        [
            Seat::with(&[COVER_WINTER, CLERIC_GUARD, CLERIC_ALLY, SMALL_GOBLIN]).at(20),
            Seat::with(&[TRAMPLER, RED_SWEEPER])
                .holding(&[ELF_ATTACKER, SMALL_GOBLIN])
                .at(20),
        ],
    );
    to_main(&mut engine, me());
    let cover = the(&engine, ZoneLocation::Battlefield, COVER_WINTER);
    let guard = the(&engine, ZoneLocation::Battlefield, CLERIC_GUARD);
    let ally = the(&engine, ZoneLocation::Battlefield, CLERIC_ALLY);
    let my_goblin = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == me() && o.card.is_some_and(|c| c.index.get() == SMALL_GOBLIN)
            })
        })
        .unwrap();
    let sweeper = the(&engine, ZoneLocation::Battlefield, RED_SWEEPER);

    // Put 2 age counters on Cover of Winter.
    activate(&mut engine, me(), cover, 0);
    settle(&mut engine, me());

    // 1) Test Daunting Defender (`CLERIC_GUARD`): `sweeper` deals 2 noncombat
    // damage to each creature and player. Cover of Winter is `combat_only: true`
    // (does NOT prevent noncombat!), while `CLERIC_GUARD` prevents 1 on EACH
    // Cleric (`guard` and `ally` take 1 each) and 0 on `my_goblin` (dies to 2)!
    engine.apply(me(), PlayerAction::PassPriority).unwrap();
    activate(&mut engine, them(), sweeper, 0);
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), 18);
    assert_eq!(engine.state().object(guard).unwrap().damage, 1);
    assert_eq!(engine.state().object(ally).unwrap().damage, 1);
    assert!(
        objects(&engine, ZoneLocation::Graveyard(me()), SMALL_GOBLIN).contains(&my_goblin),
        "non-Cleric creature received no Cleric prevention and no noncombat Cover of Winter prevention"
    );

    // 2) Advance to opponent's turn, cast Elf (2/2 Haste) and Small Goblin
    // (1/1 Haste), and attack alongside Trampler (5/5 Trample) into `me()`.
    to_main(&mut engine, them());
    let elf_hand = the(&engine, ZoneLocation::Hand(them()), ELF_ATTACKER);
    engine
        .apply(them(), PlayerAction::CastSpell { card: elf_hand })
        .unwrap();
    settle(&mut engine, them());
    let goblin_hand = the(&engine, ZoneLocation::Hand(them()), SMALL_GOBLIN);
    engine
        .apply(them(), PlayerAction::CastSpell { card: goblin_hand })
        .unwrap();
    settle(&mut engine, them());

    let elf = the(&engine, ZoneLocation::Battlefield, ELF_ATTACKER);
    let their_goblin = the(&engine, ZoneLocation::Battlefield, SMALL_GOBLIN);
    let trampler = the(&engine, ZoneLocation::Battlefield, TRAMPLER);

    // Remove Cleric Guard's static from this combat test? Wait: `ally` is a Cleric (2/3),
    // so if `ally` blocks `trampler` (5/5), `ally` has BOTH Cover of Winter (2 shared with `me()`)
    // AND Cleric Guard (1 on `ally`)! Let's block `trampler` with `ally` and watch BOTH work,
    // OR let's test Cover of Winter alone on a non-Cleric 2/3 in a clean 2nd step:
    // Actually look: if `Elf` (2/2) and `their_goblin` (1/1) go unblocked at `me()`,
    // Cover of Winter (2 age counters) prevents ALL 2 from Elf AND ALL 1 from `their_goblin`
    // (proving 2 per source, 3 prevented total across the two unblocked creatures!).
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            them(),
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (elf, Defender::Player(me())),
                    (their_goblin, Defender::Player(me())),
                    (trampler, Defender::Player(me())),
                ],
            },
        )
        .unwrap();
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::DeclareBlockers {
                blockers: vec![(ally, trampler)],
            },
        )
        .unwrap();

    // Opponent assigns Trampler's 5 damage: 3 lethal to 2/3 `ally` and 2 trample to `me()`.
    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseDamageEffect { .. } | Pending::AllocatePrevention { .. }
        )
    });
    // `ally` has Cleric Guard (1) AND Cover of Winter (2 shared across `ally` + `me()`):
    // if `ChooseDamageEffect` is asked first, apply Cover of Winter first so we also test `AllocatePrevention`!
    if let Pending::ChooseDamageEffect {
        choice, options, ..
    } = engine.pending().clone()
    {
        let cover_opt = options
            .iter()
            .find(|o| o.source == Some(cover) && o.parts.len() == 2)
            .expect("Cover of Winter shared candidate across blocker and player");
        engine
            .apply(
                me(),
                PlayerAction::ChooseDamageEffect {
                    choice,
                    effect: cover_opt.id,
                },
            )
            .unwrap();
    }
    let Pending::AllocatePrevention {
        choice,
        damage,
        total,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected AllocatePrevention for Cover of Winter across blocker and player, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(total, 2);
    assert_eq!(damage.len(), 2);
    let ally_part = damage
        .iter()
        .find(|p| p.recipient == crate::event::DamageTarget::Object(ally))
        .unwrap()
        .id;
    let me_part = damage
        .iter()
        .find(|p| p.recipient == crate::event::DamageTarget::Player(me()))
        .unwrap()
        .id;
    // Allocate 2 to `ally` (reducing 3 -> 1, and Cleric Guard prevents the remaining 1!)
    // and 0 to `me()` (so `me()` takes 2 trample from Trampler, 0 from Elf, 0 from Goblin -> 16 life!).
    engine
        .apply(
            me(),
            PlayerAction::AllocatePrevention {
                choice,
                allocation: vec![(ally_part, 2), (me_part, 0)],
            },
        )
        .unwrap();

    walk_until(&mut engine, |e| {
        holds_priority_in(e, them(), crate::turn::Step::Main)
            && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(
        life(&engine, me()),
        16,
        "Elf (2) and Goblin (1) were each prevented per-source by Cover of Winter; only Trampler's 2 unallocated trample got through"
    );
    assert_eq!(
        engine.state().object(ally).unwrap().damage,
        0,
        "2 prevented by Cover of Winter allocation + 1 prevented by Cleric Guard saved the 2/3 Cleric with 0 damage"
    );
}
