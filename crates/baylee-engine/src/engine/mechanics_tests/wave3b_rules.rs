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
