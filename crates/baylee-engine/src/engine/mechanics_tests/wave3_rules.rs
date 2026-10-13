//! Wave 3 of the partials lane, one rule per test, played by cards nobody
//! printed:
//!
//! - `Modifier::CantAttackPlayerIdleLastTurn` (Arboria): a player who
//!   neither cast a spell nor put a nontoken permanent onto the battlefield
//!   during their last turn can't be attacked.
//! - `Filter::OfSourceChosenColor` and `Condition::ChosenOpponentControls`
//!   (Jihad): an anthem and a state trigger read back the colour and the
//!   opponent chosen as the source entered.
//! - `Filter::SharesAColorWith` (Invoke Prejudice): a creature spell that
//!   shares no colour with a creature you control.
//! - `Condition::TargetSpellWouldDestroy` (Equinox): a spell that would
//!   destroy a land you control, directly, if it resolved now.
//! - Improvise (CR 702.126a), printed and granted to the next spell
//!   (Archway of Innovation).

use super::*;
use baylee_cards_dsl::{Amount, Condition, EnterModifier, Filter, Modifier, TargetSpec, Trigger};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::Defender;

const WATCHTOWER: u32 = 7500;
const RAIDER: u32 = 7501;
const TRINKET: u32 = 7502;
const ZEALOT_BANNER: u32 = 7503;
const PALADIN: u32 = 7504;
const FIREBRAND: u32 = 7505;
const BIGOT: u32 = 7506;
const ELF: u32 = 7507;
const GOBLIN: u32 = 7508;
const GOLEM: u32 = 7509;
const WARDEN: u32 = 7510;
const LAND_RAZE: u32 = 7511;
const SPARK: u32 = 7512;
const LAND_WIPE: u32 = 7513;
const TINKERING: u32 = 7514;
const COG: u32 = 7515;
const WORKSHOP: u32 = 7516;
const WIDGET: u32 = 7517;

// ------------------------------------------------------------- Arboria

static WATCHTOWER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::static_ability!(
    Filter::CREATURE,
    Modifier::CantAttackPlayerIdleLastTurn
)];

// -------------------------------------------------------------- Jihad

static NONTOKEN_OF_CHOSEN_COLOR: Filter =
    Filter::And(&[Filter::Not(&Filter::IsToken), Filter::OfSourceChosenColor]);
const WHITE_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasColor(ColorSet::from_slice(&[Color::White])),
]);
static BANNER_ABILITIES: &[AbilityDef] = &[
    baylee_cards_dsl::static_ability!(
        WHITE_CREATURE,
        Modifier::ModifyPT(2, 1),
        condition = Some(Condition::ChosenOpponentControls(&NONTOKEN_OF_CHOSEN_COLOR))
    ),
    baylee_cards_dsl::triggered!(
        Trigger::State(&Condition::Not(&Condition::ChosenOpponentControls(
            &NONTOKEN_OF_CHOSEN_COLOR
        ))),
        &[Effect::SacrificeSelf]
    ),
];

// --------------------------------------------------- Invoke Prejudice

static YOUR_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);
static UNLIKE_CREATURE_SPELL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::ControlledByOpponent,
    Filter::Not(&Filter::SharesAColorWith(&YOUR_CREATURE)),
]);
static BIGOT_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::SpellCast(&UNLIKE_CREATURE_SPELL),
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }]
)];

// ------------------------------------------------------------ Equinox

static YOUR_LAND: Filter = Filter::And(&[Filter::LAND, Filter::ControlledByYou]);
static ANY_SPELL: TargetSpec = TargetSpec::Spell(&Filter::Any);
static WARDEN_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::IfCondition {
        condition: Condition::TargetSpellWouldDestroy(&YOUR_LAND),
        then: &[Effect::CounterTargetSpell],
        otherwise: &[],
    }],
    Some(TargetReq::one(ANY_SPELL)),
)];
static LAND_RAZE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::destroy(TargetSpec::Object(&Filter::LAND))],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
)];
static SPARK_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::GainLife {
        amount: Amount::Fixed(1)
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
)];
static LAND_WIPE_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::destroy_all(
    &Filter::LAND
)])];

// ---------------------------------------------------------- improvise

static TINKERING_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(&[Effect::GainLife {
    amount: Amount::Fixed(5)
}])];
static WORKSHOP_ABILITIES: &[AbilityDef] = &[free(&[Effect::NextSpellThisTurnHasImprovise], None)];

#[allow(clippy::too_many_lines)] // One entry per card the five rules need.
fn cards() -> Vec<&'static CardDef> {
    use baylee_core::types::TypeSet as T;
    vec![
        card(
            WATCHTOWER,
            face("Watchtower", "{0}", T::ENCHANTMENT),
            KeywordSet::EMPTY,
            WATCHTOWER_ABILITIES,
        ),
        card(
            RAIDER,
            creature_face("Raider", "{1}", 2, 2),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            TRINKET,
            face("Trinket", "{0}", T::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            ZEALOT_BANNER,
            FaceDef {
                enter_modifiers: &[EnterModifier::ChooseColor, EnterModifier::ChooseOpponent],
                ..face("Zealot Banner", "{0}", T::ENCHANTMENT)
            },
            KeywordSet::EMPTY,
            BANNER_ABILITIES,
        ),
        card(
            PALADIN,
            coloured("Paladin", Color::White),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            FIREBRAND,
            coloured("Firebrand", Color::Red),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            BIGOT,
            face("Bigot", "{0}", T::ENCHANTMENT),
            KeywordSet::EMPTY,
            BIGOT_ABILITIES,
        ),
        card(ELF, coloured("Elf", Color::Green), KeywordSet::EMPTY, &[]),
        card(
            GOBLIN,
            coloured("Goblin", Color::Red),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            GOLEM,
            FaceDef {
                types: T::ARTIFACT.union(T::CREATURE),
                ..creature_face("Golem", "{0}", 1, 1)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            WARDEN,
            face("Warden", "{0}", T::ARTIFACT),
            KeywordSet::EMPTY,
            WARDEN_ABILITIES,
        ),
        card(
            LAND_RAZE,
            face("Land Raze", "{0}", T::INSTANT),
            KeywordSet::EMPTY,
            LAND_RAZE_ABILITIES,
        ),
        card(
            SPARK,
            face("Spark", "{0}", T::INSTANT),
            KeywordSet::EMPTY,
            SPARK_ABILITIES,
        ),
        card(
            LAND_WIPE,
            face("Land Wipe", "{0}", T::INSTANT),
            KeywordSet::EMPTY,
            LAND_WIPE_ABILITIES,
        ),
        card(
            TINKERING,
            FaceDef {
                improvise: true,
                ..face("Tinkering", "{3}", T::SORCERY)
            },
            KeywordSet::EMPTY,
            TINKERING_ABILITIES,
        ),
        card(COG, face("Cog", "{0}", T::ARTIFACT), KeywordSet::EMPTY, &[]),
        card(
            WORKSHOP,
            face("Workshop", "{0}", T::ARTIFACT),
            KeywordSet::EMPTY,
            WORKSHOP_ABILITIES,
        ),
        card(
            WIDGET,
            face("Widget", "{2}", T::SORCERY),
            KeywordSet::EMPTY,
            TINKERING_ABILITIES,
        ),
    ]
}

/// A 1/1 creature of one colour that costs nothing: the colour comes from
/// its indicator, so no mana is needed to cast it.
fn coloured(name: &'static str, colour: Color) -> FaceDef {
    FaceDef {
        color_indicator: ColorSet::from_slice(&[colour]),
        ..creature_face(name, "{0}", 1, 1)
    }
}

/// Casts the `index` card in `seat`'s hand, answering nothing else.
#[track_caller]
fn cast(engine: &mut Bench, seat: PlayerId, index: u32) -> ObjectId {
    let card = the(engine, ZoneLocation::Hand(seat), index);
    engine
        .apply(seat, PlayerAction::CastSpell { card })
        .expect("the card is cast");
    card
}

fn zone_of(engine: &Bench, id: ObjectId) -> Option<crate::zone::Zone> {
    engine.state().object(id).map(|o| o.zone)
}

/// Whether `seat` may declare `creature` attacking the other seat now.
#[track_caller]
fn may_attack(engine: &mut Bench, seat: PlayerId, creature: ObjectId) -> bool {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    let defender = if seat == me() { them() } else { me() };
    // A refused declaration changes nothing (the walk then declares none);
    // an accepted one is the last thing the caller asks.
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(creature, Defender::Player(defender))],
            },
        )
        .is_ok()
}

/// Arboria: on the first turn the other seat has had no turn, so it can't be
/// attacked; after a turn of doing nothing it still can't; after a turn in
/// which it cast a Trinket (a nontoken permanent onto the battlefield, by a
/// spell) it can.
#[test]
fn a_player_idle_during_their_last_turn_cannot_be_attacked() {
    let mut engine = bench(
        7500,
        cards(),
        [
            Seat::with(&[WATCHTOWER, RAIDER]),
            Seat::with(&[]).holding(&[TRINKET]),
        ],
    );
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);

    assert!(!may_attack(&mut engine, me(), raider), "no last turn yet");
    to_main(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        !may_attack(&mut engine, me(), raider),
        "a turn of doing nothing"
    );

    to_main(&mut engine, them());
    cast(&mut engine, them(), TRINKET);
    settle(&mut engine, them());
    to_main(&mut engine, me());
    assert!(
        may_attack(&mut engine, me(), raider),
        "they cast a spell and put a permanent onto the battlefield"
    );
}

/// Jihad: the Banner, cast, asks for a colour (red) and an opponent. While
/// that opponent controls a nontoken red permanent, white creatures get
/// +2/+1; once it controls none, the Banner is sacrificed.
#[test]
fn a_chosen_colour_and_opponent_are_read_back_by_an_anthem_and_a_state_trigger() {
    let mut engine = bench(
        7501,
        cards(),
        [
            Seat::with(&[PALADIN]).holding(&[ZEALOT_BANNER]),
            Seat::with(&[FIREBRAND]),
        ],
    );
    let paladin = the(&engine, ZoneLocation::Battlefield, PALADIN);
    let firebrand = the(&engine, ZoneLocation::Battlefield, FIREBRAND);
    to_main(&mut engine, me());
    let banner = cast(&mut engine, me(), ZEALOT_BANNER);
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    engine
        .apply(
            me(),
            PlayerAction::ChooseColor(baylee_core::mana::ManaColor::Red),
        )
        .expect("red is a colour");
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    engine
        .apply(me(), PlayerAction::ChoosePlayer(them()))
        .expect("the opponent is offered");
    settle(&mut engine, me());
    let power = |e: &Bench| {
        e.state()
            .object(paladin)
            .and_then(|o| o.characteristics().power)
    };
    assert_eq!(power(&engine), Some(3), "the Paladin gets +2/+1");
    assert_eq!(
        zone_of(&engine, banner),
        Some(crate::zone::Zone::Battlefield)
    );

    crate::sba::destroy(
        engine
            .dev_state_mut(me())
            .expect("the bench may set boards up"),
        firebrand,
    );
    // The board was rewritten behind the engine: let it take a step, which
    // checks the state trigger before anyone gets priority (CR 603.8).
    engine
        .apply(me(), PlayerAction::PassPriority)
        .expect("a pass");
    settle(&mut engine, me());
    assert_eq!(
        zone_of(&engine, banner),
        Some(crate::zone::Zone::Graveyard),
        "no red nontoken permanent left: the Banner is sacrificed"
    );
    assert_eq!(power(&engine), Some(1));
}

/// Invoke Prejudice: with a green Elf on my side, the other seat casting a
/// green Elf shares a colour (no trigger), a red Goblin does not (trigger),
/// and a colourless Golem shares a colour with nothing (trigger).
#[test]
fn a_spell_that_shares_no_colour_with_your_creatures_is_seen() {
    let mut engine = bench(
        7502,
        cards(),
        [
            Seat::with(&[BIGOT, ELF]),
            Seat::with(&[]).holding(&[ELF, GOBLIN, GOLEM]),
        ],
    );
    let life_before = life(&engine, me());
    to_main(&mut engine, them());
    for (index, gained) in [(ELF, 0), (GOBLIN, 1), (GOLEM, 1)] {
        let before = life(&engine, me());
        let card = the(&engine, ZoneLocation::Hand(them()), index);
        engine
            .apply(them(), PlayerAction::CastSpell { card })
            .expect("the spell is cast");
        settle(&mut engine, them());
        assert_eq!(life(&engine, me()), before + gained, "{index}");
    }
    assert_eq!(life(&engine, me()), life_before + 2);
}

/// Equinox: the Warden's ability counters a spell that would destroy a land
/// its controller controls (Land Raze aimed at it, Land Wipe), and lets
/// through one that would not (Land Raze aimed at the caster's own land,
/// Spark aimed at mine).
#[test]
fn a_spell_that_would_destroy_your_land_is_countered_and_others_are_not() {
    for (spell, aim_at_mine, countered) in [
        (LAND_RAZE, true, true),
        (LAND_RAZE, false, false),
        (SPARK, true, false),
        (LAND_WIPE, true, true),
    ] {
        let mut engine = bench(
            7503,
            cards(),
            [
                Seat::with(&[WARDEN, forest()]),
                Seat::with(&[forest()]).holding(&[spell]),
            ],
        );
        let warden = the(&engine, ZoneLocation::Battlefield, WARDEN);
        let forests = objects(&engine, ZoneLocation::Battlefield, forest());
        let mine = *forests
            .iter()
            .find(|f| {
                engine
                    .state()
                    .object(**f)
                    .is_some_and(|o| o.controller == me())
            })
            .expect("my Forest");
        let theirs = *forests
            .iter()
            .find(|f| {
                engine
                    .state()
                    .object(**f)
                    .is_some_and(|o| o.controller == them())
            })
            .expect("their Forest");
        to_main(&mut engine, them());
        let card = cast(&mut engine, them(), spell);
        if let Pending::ChooseTargets { .. } = engine.pending() {
            let aim = if aim_at_mine { mine } else { theirs };
            engine
                .apply(
                    them(),
                    PlayerAction::ChooseTargets {
                        objects: vec![aim],
                        players: vec![],
                    },
                )
                .expect("a land is a legal target");
        }
        walk_until(
            &mut engine,
            |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == me()),
        );
        activate(&mut engine, me(), warden, 0);
        let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
            panic!("the Warden asks for a spell: {:?}", engine.pending());
        };
        assert!(options.contains(&card));
        engine
            .apply(
                me(),
                PlayerAction::ChooseTargets {
                    objects: vec![card],
                    players: vec![],
                },
            )
            .expect("the spell is a legal target");
        walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
        assert_eq!(
            zone_of(&engine, mine) == Some(crate::zone::Zone::Battlefield),
            countered || !aim_at_mine || spell == SPARK,
            "{spell} at mine {aim_at_mine}: my Forest"
        );
        let countered_now = engine.journal().entries().iter().any(|e| {
            matches!(e.event, crate::event::GameEvent::SpellCountered { object } if object == card)
        });
        assert_eq!(countered_now, countered, "{spell} at mine {aim_at_mine}");
    }
}

/// Improvise: Tinkering ({3}) is cast with three untapped artifacts tapped
/// and no mana; the tap question offers the artifacts and not the creature.
/// Granted: after the Workshop's ability, Widget ({2}, no improvise of its
/// own) is cast the same way, and the grant is spent by that cast.
#[test]
fn improvise_taps_artifacts_to_pay_generic_mana_printed_or_granted() {
    let mut engine = bench(
        7504,
        cards(),
        [
            Seat::with(&[COG, COG, COG, ELF]).holding(&[TINKERING]),
            Seat::with(&[]),
        ],
    );
    let cogs = objects(&engine, ZoneLocation::Battlefield, COG);
    let elf = the(&engine, ZoneLocation::Battlefield, ELF);
    let before = life(&engine, me());
    to_main(&mut engine, me());
    cast(&mut engine, me(), TINKERING);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the tap question: {:?}", engine.pending());
    };
    assert!(cogs.iter().all(|c| options.contains(c)), "{options:?}");
    assert!(!options.contains(&elf), "a creature is not an artifact");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: cogs.clone(),
                players: vec![],
            },
        )
        .expect("three artifacts pay {3}");
    settle(&mut engine, me());
    assert_eq!(life(&engine, me()), before + 5, "Tinkering resolved");

    let mut engine = bench(
        7505,
        cards(),
        [
            Seat::with(&[WORKSHOP, COG, COG]).holding(&[WIDGET, WIDGET]),
            Seat::with(&[]),
        ],
    );
    let workshop = the(&engine, ZoneLocation::Battlefield, WORKSHOP);
    let cogs = objects(&engine, ZoneLocation::Battlefield, COG);
    to_main(&mut engine, me());
    let widget = objects(&engine, ZoneLocation::Hand(me()), WIDGET)[0];
    assert!(
        engine
            .apply(me(), PlayerAction::CastSpell { card: widget })
            .is_err()
            || !matches!(engine.pending(), Pending::ChooseTargets { .. }),
        "without the grant nothing pays for Widget"
    );
    let mut engine = bench(
        7505,
        cards(),
        [
            Seat::with(&[WORKSHOP, COG, COG]).holding(&[WIDGET, WIDGET]),
            Seat::with(&[]),
        ],
    );
    to_main(&mut engine, me());
    activate(&mut engine, me(), workshop, 0);
    settle(&mut engine, me());
    let widget = objects(&engine, ZoneLocation::Hand(me()), WIDGET)[0];
    engine
        .apply(me(), PlayerAction::CastSpell { card: widget })
        .expect("Widget is castable with the grant");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: cogs,
                players: vec![],
            },
        )
        .expect("two artifacts pay {2}");
    settle(&mut engine, me());
    assert!(
        !engine.state().per_turn.improvise_next_spell[0],
        "the grant is spent by that cast"
    );
}
