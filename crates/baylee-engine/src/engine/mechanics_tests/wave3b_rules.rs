//! Wave 3 push B of the partials lane (K16, pay per unit and variable
//! costs), one rule per test, played by cards nobody printed:
//!
//! - `Effect::PlayerMayPayManaEachThenElse` (Cyclone): a coloured price
//!   repeated N times, one payment, two branches.
//! - `Effect::UntapChosenPayingEach` (Magnetic Mountain): choose any number of
//!   tapped permanents, pay per permanent, untap them.
//! - `CostPart::DiscardLastDrawnThisTurn` (Jandor's Ring).
//! - `Amount::SacrificedTotalPower` and `Effect::ExileSourceAndSacrificed`
//!   (Sword of the Ages), with `CostPart::SacrificeX`.

use super::*;
use baylee_cards_dsl::{Amount, Cost, CostPart, Filter, PlayerRel, TargetSpec};

const GALE: u32 = 7600;
const LODESTONE: u32 = 7601;
const SPRITE: u32 = 7602;
const RING: u32 = 7603;
const BLADE: u32 = 7604;
const BRUTE: u32 = 7605;

static GALE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::PlayerMayPayManaEachThenElse {
        player: PlayerRel::You,
        each: baylee_core::mana::ManaCost::parse("{G}"),
        times: Amount::Fixed(2),
        paid: &[Effect::GainLife {
            amount: Amount::Fixed(5),
        }],
        unpaid: &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
    }],
    None,
)];

static LODESTONE_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::UntapChosenPayingEach {
        who: PlayerRel::You,
        filter: &Filter::CREATURE,
        each: baylee_core::mana::ManaCost::parse("{G}"),
    }],
    None,
)];

static RING_COST: &[CostPart] = &[CostPart::DiscardLastDrawnThisTurn];
static DRAW_TWO: &[Effect] = &[Effect::DrawCards {
    amount: Amount::Fixed(2),
}];
/// 0: "Discard the last card you drew this turn: gain 1 life"; 1: "{0}:
/// draw two cards".
static RING_ABILITIES: &[AbilityDef] = &[
    paid(
        Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: RING_COST,
        },
        &[Effect::GainLife {
            amount: Amount::Fixed(1),
        }],
        None,
    ),
    free(DRAW_TWO, None),
];

static YOUR_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
static BLADE_COST: &[CostPart] = &[
    CostPart::SacrificeSelf,
    CostPart::SacrificeX(&YOUR_CREATURE),
];
static BLADE_ABILITIES: &[AbilityDef] = &[paid(
    Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: BLADE_COST,
    },
    &[
        Effect::DealDamage {
            amount: Amount::SacrificedTotalPower,
            target: TargetSpec::Player(PlayerRel::Opponent),
        },
        Effect::ExileSourceAndSacrificed,
    ],
    None,
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            GALE,
            face("Gale", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            GALE_ABILITIES,
        ),
        card(
            LODESTONE,
            face("Lodestone", "{0}", TypeSet::ENCHANTMENT),
            KeywordSet::EMPTY,
            LODESTONE_ABILITIES,
        ),
        card(
            SPRITE,
            creature_face("Sprite", "{1}", 1, 1),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            RING,
            face("Ring", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            RING_ABILITIES,
        ),
        card(
            BLADE,
            face("Blade", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            BLADE_ABILITIES,
        ),
        card(
            BRUTE,
            creature_face("Brute", "{1}", 3, 3),
            KeywordSet::EMPTY,
            &[],
        ),
    ]
}

fn zone_of(engine: &Bench, id: ObjectId) -> Option<crate::zone::Zone> {
    engine.state().object(id).map(|o| o.zone)
}

fn tap(engine: &mut Bench, id: ObjectId) {
    engine
        .dev_state_mut(me())
        .expect("the bench may set boards up")
        .set_tapped(id, true);
    engine.refresh_offer();
}

/// Cyclone's price, {G} twice: with GG floating, paying runs the paid branch
/// (5 life) and empties the pool; declining runs the other (1 life).
#[test]
fn a_repeated_coloured_price_is_paid_at_once_or_declined() {
    for (pay, gained) in [(true, 5), (false, 1)] {
        let mut engine = bench(
            7600,
            cards(),
            [Seat::with(&[GALE, forest(), forest()]), Seat::with(&[])],
        );
        let gale = the(&engine, ZoneLocation::Battlefield, GALE);
        to_main(&mut engine, me());
        float_all(&mut engine, me());
        let before = life(&engine, me());
        activate(&mut engine, me(), gale, 0);
        walk_until(&mut engine, |e| {
            matches!(e.pending(), Pending::YesNo { .. })
        });
        let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
            unreachable!()
        };
        assert_eq!(
            prompt,
            crate::choice::YesNoPrompt::PayMana {
                cost: baylee_core::mana::ManaCost::parse("{G}{G}")
            },
            "the whole price, once"
        );
        engine
            .apply(me(), PlayerAction::YesNo(pay))
            .expect("an answer");
        settle(&mut engine, me());
        assert_eq!(life(&engine, me()), before + gained, "paid: {pay}");
        assert_eq!(
            engine.state().players[0].mana_pool.total(),
            if pay { 0 } else { 2 },
            "paid: {pay}"
        );
    }
}

/// Magnetic Mountain's choice: two tapped Sprites, one chosen, {G} paid for
/// it: that one untaps, the other stays tapped.
#[test]
fn chosen_tapped_permanents_untap_once_their_price_is_paid() {
    let mut engine = bench(
        7601,
        cards(),
        [
            Seat::with(&[LODESTONE, SPRITE, SPRITE, forest()]),
            Seat::with(&[]),
        ],
    );
    let lodestone = the(&engine, ZoneLocation::Battlefield, LODESTONE);
    let sprites = objects(&engine, ZoneLocation::Battlefield, SPRITE);
    to_main(&mut engine, me());
    for s in &sprites {
        tap(&mut engine, *s);
    }
    float_all(&mut engine, me());
    activate(&mut engine, me(), lodestone, 0);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options.len(), 2, "both tapped Sprites");
    assert_eq!(max, 2, "any number of them");
    engine
        .apply(
            me(),
            PlayerAction::ChooseObjects {
                objects: vec![sprites[0]],
            },
        )
        .expect("one Sprite");
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!("the price is asked: {:?}", engine.pending());
    };
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::PayMana {
            cost: baylee_core::mana::ManaCost::parse("{G}")
        }
    );
    engine.apply(me(), PlayerAction::YesNo(true)).expect("paid");
    settle(&mut engine, me());
    let tapped = |e: &Bench, id: ObjectId| {
        e.state()
            .object(id)
            .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
    };
    assert!(!tapped(&engine, sprites[0]), "the chosen Sprite untapped");
    assert!(tapped(&engine, sprites[1]), "the other did not");
}

/// Jandor's Ring: before anything is drawn this turn the ability is not
/// offered; after "draw two", it discards the second card drawn, the last.
#[test]
fn the_last_card_drawn_this_turn_is_the_one_discarded() {
    let mut engine = bench(7602, cards(), [Seat::with(&[RING]), Seat::with(&[])]);
    let ring = the(&engine, ZoneLocation::Battlefield, RING);
    to_main(&mut engine, me());
    let (_, legal) = offer(&engine);
    assert!(
        !legal.abilities.contains(&(ring, 0)),
        "nothing drawn this turn: the cost can't be paid"
    );
    activate(&mut engine, me(), ring, 1);
    settle(&mut engine, me());
    let drawn: Vec<ObjectId> = engine
        .state()
        .per_turn
        .drawn
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let last = *drawn.last().expect("two cards drawn");
    activate(&mut engine, me(), ring, 0);
    settle(&mut engine, me());
    assert_eq!(
        zone_of(&engine, last),
        Some(crate::zone::Zone::Graveyard),
        "the last card drawn was discarded"
    );
    assert_eq!(
        zone_of(&engine, drawn[drawn.len() - 2]),
        Some(crate::zone::Zone::Hand),
        "the one before stays"
    );
}

/// Sword of the Ages: sacrificing itself and two creatures (3 and 1 power)
/// deals 4 to the opponent, then the Blade and both creature cards are
/// exiled.
#[test]
fn the_sacrificed_creatures_total_power_is_dealt_then_they_are_exiled() {
    let mut engine = bench(
        7603,
        cards(),
        [Seat::with(&[BLADE, BRUTE, SPRITE]), Seat::with(&[])],
    );
    let blade = the(&engine, ZoneLocation::Battlefield, BLADE);
    let brute = the(&engine, ZoneLocation::Battlefield, BRUTE);
    let sprite = the(&engine, ZoneLocation::Battlefield, SPRITE);
    let before = life(&engine, them());
    to_main(&mut engine, me());
    activate(&mut engine, me(), blade, 0);
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseNumber { max, .. } => {
                assert_eq!(max, 2, "two creatures could be sacrificed");
                engine
                    .apply(me(), PlayerAction::ChooseNumber(2))
                    .expect("X = 2");
            }
            Pending::ChooseCards { options, min, .. } => {
                let picks: Vec<ObjectId> = options
                    .iter()
                    .copied()
                    .filter(|o| [brute, sprite].contains(o))
                    .take(usize::from(min))
                    .collect();
                engine
                    .apply(me(), PlayerAction::ChooseObjects { objects: picks })
                    .expect("the creatures to sacrifice");
            }
            Pending::ChooseTargets { options, .. } => {
                let mut picks = options.clone();
                picks.retain(|o| [brute, sprite].contains(o));
                engine
                    .apply(
                        me(),
                        PlayerAction::ChooseTargets {
                            objects: picks,
                            players: vec![],
                        },
                    )
                    .expect("the creatures to sacrifice");
            }
            _ => break,
        }
    }
    settle(&mut engine, me());
    assert_eq!(life(&engine, them()), before - 4, "3 + 1");
    for id in [blade, brute, sprite] {
        assert_eq!(
            zone_of(&engine, id),
            Some(crate::zone::Zone::Exile),
            "{id:?} is exiled"
        );
    }
}

const SLOWTRIP_ORB: u32 = 7606;
const CU_SHRINE: u32 = 7607;
const CU_FOUNT: u32 = 7608;
const CU_SHAMAN: u32 = 7609;

static SLOWTRIP_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::AtNextTurnsUpkeep {
        effects: &[Effect::GainLife {
            amount: Amount::Fixed(3),
        }],
    }],
    None,
)];

static CU_SHRINE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    baylee_cards_dsl::Trigger::StepBegin {
        step: baylee_cards_dsl::StepKind::Upkeep,
        whose: PlayerRel::You,
    },
    &[
        Effect::AddCounter {
            kind: baylee_cards_dsl::counters::AGE,
            amount: Amount::Fixed(1),
        },
        Effect::PlayerMayPayOr {
            player: PlayerRel::You,
            mana: Amount::CountersOnSource(baylee_cards_dsl::counters::AGE),
            effect: &Effect::SacrificeSelf,
        },
    ],
    condition = Some(baylee_cards_dsl::Condition::SourceMatches(&Filter::InZone(
        baylee_cards_dsl::ZoneRef::Battlefield
    ))),
)];

static CU_FOUNT_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::mana_ability!(
    baylee_cards_dsl::cost!(TapSelf),
    &[
        Effect::mana(baylee_core::mana::ManaColor::Blue, 1).restricted_for(
            &Filter::Any,
            baylee_cards_dsl::SpendRider::None,
            baylee_cards_dsl::ManaUse::CumulativeUpkeep,
        )
    ],
)];

static NON_CU_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::ControlledByYou,
    Filter::Not(&Filter::HasCumulativeUpkeep),
]);

static CU_SHAMAN_ABILITIES: &[AbilityDef] = &[paid(
    Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[],
    },
    &[Effect::destroy(TargetSpec::Object(&NON_CU_ENCHANTMENT))],
    Some(baylee_cards_dsl::TargetReq::one(TargetSpec::Object(
        &NON_CU_ENCHANTMENT,
    ))),
)];

#[test]
fn at_next_turns_upkeep_fires_on_opponents_upkeep_next_turn() {
    let mut defs = cards();
    defs.push(card(
        SLOWTRIP_ORB,
        face("Slowtrip Orb", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        SLOWTRIP_ABILITIES,
    ));
    let mut engine = bench(7606, defs, [Seat::with(&[SLOWTRIP_ORB]), Seat::default()]);
    to_main(&mut engine, me());
    let orb = the(&engine, ZoneLocation::Battlefield, SLOWTRIP_ORB);
    let before = life(&engine, me());
    activate(&mut engine, me(), orb, 0);
    settle(&mut engine, me());
    assert_eq!(
        life(&engine, me()),
        before,
        "delayed trigger does not fire during the turn it was created"
    );

    to_main(&mut engine, them());
    assert_eq!(
        life(&engine, me()),
        before + 3,
        "delayed trigger fired at the beginning of the opponent's upkeep (the next turn's upkeep)"
    );
}

#[test]
fn cumulative_upkeep_mana_pays_cumulative_upkeep_and_refuses_ordinary_costs() {
    let mut defs = cards();
    defs.push(card(
        CU_SHRINE,
        face("CU Shrine", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CU_SHRINE_ABILITIES,
    ));
    defs.push(card(
        CU_FOUNT,
        face("CU Fount", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CU_FOUNT_ABILITIES,
    ));
    let mut engine = bench(
        7607,
        defs,
        [Seat::with(&[CU_SHRINE, CU_FOUNT]), Seat::default()],
    );
    let shrine = the(&engine, ZoneLocation::Battlefield, CU_SHRINE);
    let fount = the(&engine, ZoneLocation::Battlefield, CU_FOUNT);
    let one = baylee_core::mana::ManaCost::parse("{1}");

    walk_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: crate::choice::YesNoPrompt::PayTax { mana: 1 },
                ..
            }
        )
    });
    engine
        .apply(me(), PlayerAction::YesNo(true))
        .expect("accept paying cumulative upkeep to open CR 605.3a mana window");
    activate(&mut engine, me(), fount, 0);
    assert!(
        !engine.can_pay_mana(me(), crate::casting::SpendFor::Other, &one),
        "cumulative-upkeep-only mana cannot pay ordinary nonspell taxes"
    );
    assert!(
        !engine.can_pay_mana(me(), crate::casting::SpendFor::Spell(shrine), &one),
        "cumulative-upkeep-only mana cannot pay spells"
    );
    assert!(
        engine.can_pay_mana(me(), crate::casting::SpendFor::CumulativeUpkeep(None), &one),
        "cumulative-upkeep-only mana is admitted for cumulative upkeep payments"
    );
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("pay cumulative upkeep with CU Fount mana");
    settle(&mut engine, me());
    assert_eq!(
        zone_of(&engine, shrine),
        Some(crate::zone::Zone::Battlefield),
        "CU Shrine stays on the battlefield after paying its cumulative upkeep"
    );
}

#[test]
fn has_cumulative_upkeep_filter_distinguishes_permanent_with_and_without_cumulative_upkeep() {
    let mut defs = cards();
    defs.push(card(
        CU_SHRINE,
        face("CU Shrine", "{0}", TypeSet::ENCHANTMENT),
        KeywordSet::EMPTY,
        CU_SHRINE_ABILITIES,
    ));
    defs.push(card(
        CU_SHAMAN,
        face("CU Shaman", "{0}", TypeSet::ARTIFACT),
        KeywordSet::EMPTY,
        CU_SHAMAN_ABILITIES,
    ));
    let mut engine = bench(
        7608,
        defs,
        [Seat::with(&[CU_SHRINE, GALE, CU_SHAMAN]), Seat::default()],
    );
    let shrine = the(&engine, ZoneLocation::Battlefield, CU_SHRINE);
    let gale = the(&engine, ZoneLocation::Battlefield, GALE);
    let shaman = the(&engine, ZoneLocation::Battlefield, CU_SHAMAN);
    activate(&mut engine, me(), shaman, 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&gale),
        "enchantment without cumulative upkeep is a legal target"
    );
    assert!(
        !options.contains(&shrine),
        "enchantment with cumulative upkeep is excluded by Filter::Not(&Filter::HasCumulativeUpkeep)"
    );
}
