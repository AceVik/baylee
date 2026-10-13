//! What combat leaves behind for a rule to read, played by creatures nobody
//! printed.
//!
//! Three readings, each with the board that tells it from its wrong twin:
//!
//! - **A dead source's combat partners.** "When this creature dies, destroy
//!   all creatures blocking or blocked by it" (Abu Ja'far) is read as the
//!   creature last existed on the battlefield (CR 608.2h): it died in combat
//!   damage and is out of combat by the time its trigger resolves, so the
//!   live combat no longer pairs it with anything. A creature that was never
//!   in combat with it is not destroyed.
//! - **An attacking source's defending player.** "Target artifact defending
//!   player controls" on an ability of an attacking creature means the player
//!   that creature attacks (CR 508.5); at a table of three the other
//!   opponent's artifact is not a legal target.
//! - **A defending player that outlives the attack.** Once an attacking
//!   creature is removed from combat (here: destroyed in response to its
//!   attack trigger), "defending player" is still the player it was
//!   attacking before (CR 508.5), at a table of three too.
//! - **This turn's end of combat.** "At this turn's next end of combat"
//!   (Glyph of Doom) lapses with the turn (CR 603.7b), where a plain "at end
//!   of combat" waits for the next one.
//! - **Assigning no combat damage.** A creature that "assigns no combat
//!   damage this turn" is left out of the step altogether (it overrides
//!   CR 510.1a): blocked by two creatures, nobody is asked to divide its
//!   damage (CR 510.1c), and neither blocker is dealt any.

use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier, TargetSpec, Trigger};
use baylee_core::ids::Defender;

const MARTYR: u32 = 7300;
const OGRE: u32 = 7301;
const BYSTANDER: u32 = 7302;
const RAIDER: u32 = 7303;
const RELIC: u32 = 7304;
const PACIFIST: u32 = 7305;
const DRAINER: u32 = 7306;
const EXECUTIONER: u32 = 7307;
const DOOM_ENGINE: u32 = 7308;
const FEINT: u32 = 7309;
const SWAP: u32 = 7310;
const TRINKET: u32 = 7311;
const CALTROPS: u32 = 7312;
const WALL: u32 = 7313;

static PARTNERS: Filter = Filter::Or(&[Filter::BlockingSource, Filter::BlockedBySource]);

static MARTYR_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Dies(&Filter::This),
    &[Effect::destroy_all_no_regen(&PARTNERS)]
)];

static DEFENDERS_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::ControlledByDefendingPlayer]);

static RAIDER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::destroy(TargetSpec::Object(&DEFENDERS_ARTIFACT))],
    targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDERS_ARTIFACT)))
)];

static PACIFIST_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::continuous(
        &Filter::This,
        Modifier::AssignsNoCombatDamage,
        Duration::UntilEndOfTurn,
    )]
)];

static DRAINER_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::Attacks(&Filter::This),
    &[Effect::LoseLife {
        amount: baylee_cards_dsl::Amount::Fixed(2),
        target: baylee_cards_dsl::PlayerRel::DefendingPlayer,
    }]
)];

static EXECUTIONER_ABILITIES: &[AbilityDef] = &[free(
    &[Effect::destroy(TargetSpec::Object(&Filter::CREATURE))],
    Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
)];

/// Ability 0: "at this turn's next end of combat, destroy target creature";
/// ability 1: the same "at end of combat", with no stated turn.
static DOOM_ENGINE_ABILITIES: &[AbilityDef] = &[
    free(
        &[Effect::AtEndOfCombatThisTurn {
            about: TargetSpec::Object(&Filter::CREATURE),
            effects: &[Effect::destroy(TargetSpec::EventObject)],
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
    free(
        &[Effect::AtEndOfCombat {
            about: TargetSpec::Object(&Filter::CREATURE),
            effects: &[Effect::destroy(TargetSpec::EventObject)],
        }],
        Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    ),
];

static FEINTED: Filter = Filter::Or(&[Filter::Reference, Filter::BlockingReference]);
/// "Prevent all combat damage that would be dealt this turn by target
/// attacking creature and each creature blocking it."
static FEINT_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::continuous(
        &FEINTED,
        Modifier::PreventDamageFromIt,
        Duration::UntilEndOfTurn,
    )],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &Filter::ATTACKING_CREATURE
    )))
)];

static YOUR_ARTIFACT: Filter = Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou]);
static THEIR_LIKE: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::SharesTypeWithReference(TypeSet::ARTIFACT),
]);
/// "Target artifact you control and target permanent an opponent controls
/// that shares a type with it": a second target relative to the first.
static SWAP_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::spell!(
    &[Effect::GainLife {
        amount: baylee_cards_dsl::Amount::Fixed(1)
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(&YOUR_ARTIFACT))),
    second_targets = Some(TargetReq::one(TargetSpec::Object(&THEIR_LIKE)))
)];

const WALLS: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(baylee_core::generated::subtypes::creature::WALL),
]);
const NON_WALLS: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(
        baylee_core::generated::subtypes::creature::WALL,
    )),
]);
/// Wall of Caltrops' shape: "whenever this creature blocks a creature, if at
/// least one other Wall is blocking that creature and no non-Wall is, it
/// gains banding until end of turn" (the probe: its controller gains 1 life).
static CALTROPS_ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
    Trigger::BlocksCreature(&Filter::CREATURE),
    &[Effect::GainLife {
        amount: baylee_cards_dsl::Amount::Fixed(1)
    }],
    condition = Some(baylee_cards_dsl::Condition::EventBlockedByOthers {
        some: &WALLS,
        none: &NON_WALLS,
    })
)];

fn cards() -> Vec<&'static CardDef> {
    vec![
        card(
            MARTYR,
            creature_face("Martyr", "{1}", 0, 1),
            KeywordSet::HASTE,
            MARTYR_ABILITIES,
        ),
        card(
            OGRE,
            creature_face("Ogre", "{1}", 3, 5),
            KeywordSet::HASTE,
            &[],
        ),
        card(
            BYSTANDER,
            creature_face("Bystander", "{1}", 2, 2),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            RAIDER,
            creature_face("Raider", "{1}", 2, 2),
            KeywordSet::HASTE,
            RAIDER_ABILITIES,
        ),
        card(
            RELIC,
            face("Relic", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            PACIFIST,
            creature_face("Pacifist", "{1}", 4, 4),
            KeywordSet::HASTE,
            PACIFIST_ABILITIES,
        ),
        card(
            DRAINER,
            creature_face("Drainer", "{1}", 1, 1),
            KeywordSet::HASTE,
            DRAINER_ABILITIES,
        ),
        card(
            EXECUTIONER,
            face("Executioner", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            EXECUTIONER_ABILITIES,
        ),
        card(
            CALTROPS,
            FaceDef {
                subtypes: &[baylee_core::generated::subtypes::creature::WALL],
                ..creature_face("Caltrops", "{1}", 0, 4)
            },
            KeywordSet::EMPTY,
            CALTROPS_ABILITIES,
        ),
        card(
            WALL,
            FaceDef {
                subtypes: &[baylee_core::generated::subtypes::creature::WALL],
                ..creature_face("Wall", "{1}", 0, 4)
            },
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            FEINT,
            face("Feint", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            FEINT_ABILITIES,
        ),
        card(
            SWAP,
            face("Swap", "{0}", TypeSet::INSTANT),
            KeywordSet::EMPTY,
            SWAP_ABILITIES,
        ),
        card(
            TRINKET,
            face("Trinket", "{0}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            &[],
        ),
        card(
            DOOM_ENGINE,
            face("Doom Engine", "{1}", TypeSet::ARTIFACT),
            KeywordSet::EMPTY,
            DOOM_ENGINE_ABILITIES,
        ),
    ]
}

/// Walks to `seat`'s declaration of attackers and attacks `defender` with
/// `attackers`.
#[track_caller]
fn attack(engine: &mut Bench, seat: PlayerId, attackers: &[ObjectId], defender: PlayerId) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == seat),
    );
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(defender)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
}

/// Walks to `defender`'s declaration of blockers and declares `blocks`.
#[track_caller]
fn block(engine: &mut Bench, defender: PlayerId, blocks: &[(ObjectId, ObjectId)]) {
    walk_until(
        engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == defender),
    );
    engine
        .apply(
            defender,
            PlayerAction::DeclareBlockers {
                blockers: blocks.to_vec(),
            },
        )
        .expect("the blocks are legal");
}

fn zone_of(engine: &Bench, id: ObjectId) -> Option<crate::zone::Zone> {
    engine.state().object(id).map(|o| o.zone)
}

/// The Martyr attacks and both Ogres block it. Combat damage kills it and
/// nothing else; its dies trigger then destroys both Ogres, which were
/// blocking it as it last existed, and not the Bystander, which was never in
/// combat with it. Before the Martyr's partners were remembered as it left,
/// the trigger found it in no combat and destroyed nothing.
#[test]
fn a_dead_creature_s_blockers_are_read_as_it_last_existed() {
    let mut engine = bench(
        7300,
        cards(),
        [Seat::with(&[MARTYR]), Seat::with(&[OGRE, OGRE, BYSTANDER])],
    );
    let martyr = the(&engine, ZoneLocation::Battlefield, MARTYR);
    let ogres = objects(&engine, ZoneLocation::Battlefield, OGRE);
    assert_eq!(ogres.len(), 2, "two Ogres");
    let bystander = the(&engine, ZoneLocation::Battlefield, BYSTANDER);

    attack(&mut engine, me(), &[martyr], them());
    block(
        &mut engine,
        them(),
        &[(ogres[0], martyr), (ogres[1], martyr)],
    );
    walk_until(&mut engine, |e| {
        zone_of(e, martyr) == Some(crate::zone::Zone::Graveyard) && e.state().zones.stack_is_empty()
    });

    for ogre in &ogres {
        assert_eq!(
            zone_of(&engine, *ogre),
            Some(crate::zone::Zone::Graveyard),
            "each Ogre blocking the Martyr is destroyed"
        );
    }
    assert_eq!(
        zone_of(&engine, bystander),
        Some(crate::zone::Zone::Battlefield),
        "the Bystander was never in combat with it"
    );
}

/// Three seats, each of the other two with a Relic. The Raider attacks the
/// second seat; its attack trigger may target only that seat's Relic, the
/// defending player's (CR 508.5), and destroys it. Before, every opponent of
/// the attacking player counted as "defending player" during combat and the
/// third seat's Relic was offered too.
#[test]
fn an_attacking_creature_s_defending_player_is_the_one_it_attacks() {
    let third = PlayerId::new(2);
    let mut engine = bench(
        7301,
        cards(),
        [
            Seat::with(&[RAIDER]),
            Seat::with(&[RELIC]),
            Seat::with(&[RELIC]),
        ],
    );
    let raider = the(&engine, ZoneLocation::Battlefield, RAIDER);
    let relic_of = |seat: PlayerId| {
        objects(&engine, ZoneLocation::Battlefield, RELIC)
            .into_iter()
            .find(|r| {
                engine
                    .state()
                    .object(*r)
                    .is_some_and(|o| o.controller == seat)
            })
            .expect("a Relic of that seat's")
    };
    let attacked_relic = relic_of(them());
    let other_relic = relic_of(third);

    attack(&mut engine, me(), &[raider], them());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "the attack trigger asks for its target: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, me());
    assert_eq!(
        options,
        vec![attacked_relic],
        "only the attacked player's artifact (not {other_relic:?})"
    );
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![attacked_relic],
                players: vec![],
            },
        )
        .expect("the offered target is taken");
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());
    assert_eq!(
        zone_of(&engine, attacked_relic),
        Some(crate::zone::Zone::Graveyard)
    );
    assert_eq!(
        zone_of(&engine, other_relic),
        Some(crate::zone::Zone::Battlefield)
    );
}

/// The Pacifist (4/4) assigns no combat damage the turn it attacks. Blocked
/// by two Bystanders (2/2 each), its controller is asked to divide nothing
/// (the walk panics on any question it was not built for, a division
/// included) and both Bystanders live; the Pacifist itself is dealt their
/// 4 damage and dies.
#[test]
fn a_creature_that_assigns_no_combat_damage_is_left_out_of_the_step() {
    let mut engine = bench(
        7302,
        cards(),
        [Seat::with(&[PACIFIST]), Seat::with(&[BYSTANDER, BYSTANDER])],
    );
    let pacifist = the(&engine, ZoneLocation::Battlefield, PACIFIST);
    let bystanders = objects(&engine, ZoneLocation::Battlefield, BYSTANDER);

    attack(&mut engine, me(), &[pacifist], them());
    block(
        &mut engine,
        them(),
        &[(bystanders[0], pacifist), (bystanders[1], pacifist)],
    );
    walk_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });

    for bystander in &bystanders {
        let obj = engine.state().object(*bystander).expect("a Bystander");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(obj.damage, 0, "the Pacifist assigned it nothing");
    }
    assert_eq!(
        zone_of(&engine, pacifist),
        Some(crate::zone::Zone::Graveyard),
        "the blockers' damage is dealt as usual"
    );
}

/// Activates `index` of `source` for `seat` and names `target` as its one
/// object target.
#[track_caller]
fn activate_at(engine: &mut Bench, seat: PlayerId, source: ObjectId, index: u32, target: ObjectId) {
    activate(engine, seat, source, index);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the ability asks for its target: {:?}", engine.pending());
    };
    assert!(
        options.contains(&target),
        "{target:?} is offered: {options:?}"
    );
    engine
        .apply(
            seat,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the offered target is taken");
}

/// Three seats. The Drainer attacks the second seat; with its trigger on the
/// stack its controller destroys it. The trigger resolves after the Drainer
/// has left combat, and "defending player" is still the seat it was
/// attacking (CR 508.5): that seat loses 2, the third does not. Before the
/// defender was remembered, a creature no longer attacking named nobody at a
/// table of three and nobody lost life.
#[test]
fn a_creature_removed_from_combat_keeps_its_defending_player() {
    let third = PlayerId::new(2);
    let mut engine = bench(
        7303,
        cards(),
        [
            Seat::with(&[DRAINER, EXECUTIONER]),
            Seat::with(&[]),
            Seat::with(&[]),
        ],
    );
    let drainer = the(&engine, ZoneLocation::Battlefield, DRAINER);
    let executioner = the(&engine, ZoneLocation::Battlefield, EXECUTIONER);
    let (before_them, before_third) = (life(&engine, them()), life(&engine, third));

    attack(&mut engine, me(), &[drainer], them());
    walk_until(&mut engine, |e| {
        !e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    activate_at(&mut engine, me(), executioner, 0, drainer);
    walk_until(&mut engine, |e| e.state().zones.stack_is_empty());

    assert_eq!(
        zone_of(&engine, drainer),
        Some(crate::zone::Zone::Graveyard),
        "the Drainer left combat before its trigger resolved"
    );
    assert_eq!(
        life(&engine, them()),
        before_them - 2,
        "the player it was attacking loses 2"
    );
    assert_eq!(
        life(&engine, third),
        before_third,
        "the other opponent does not"
    );
}

/// "At this turn's next end of combat": activated in the first main phase
/// it destroys its creature as this turn's end of combat step begins;
/// activated after combat it never fires, not even at the next turn's end
/// of combat, where the plain "at end of combat" twin does destroy its
/// creature (CR 603.7b).
#[test]
fn this_turn_s_end_of_combat_lapses_with_the_turn() {
    let mut engine = bench(
        7304,
        cards(),
        [
            Seat::with(&[DOOM_ENGINE]),
            Seat::with(&[BYSTANDER, BYSTANDER, BYSTANDER]),
        ],
    );
    let doom = the(&engine, ZoneLocation::Battlefield, DOOM_ENGINE);
    let bystanders = objects(&engine, ZoneLocation::Battlefield, BYSTANDER);
    let (early, late, plain) = (bystanders[0], bystanders[1], bystanders[2]);

    to_main(&mut engine, me());
    activate_at(&mut engine, me(), doom, 0, early);
    settle(&mut engine, me());
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain
            && e.state().zones.stack_is_empty()
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
    });
    assert_eq!(
        zone_of(&engine, early),
        Some(crate::zone::Zone::Graveyard),
        "this turn's end of combat came after the activation"
    );

    activate_at(&mut engine, me(), doom, 0, late);
    settle(&mut engine, me());
    activate_at(&mut engine, me(), doom, 1, plain);
    settle(&mut engine, me());
    to_main(&mut engine, them());
    walk_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && e.state().zones.stack_is_empty()
    });

    assert_eq!(
        zone_of(&engine, plain),
        Some(crate::zone::Zone::Graveyard),
        "a plain \"at end of combat\" waits for the next turn's"
    );
    assert_eq!(
        zone_of(&engine, late),
        Some(crate::zone::Zone::Battlefield),
        "this turn's end of combat had passed, and the next turn's is not it"
    );
}

/// Feint cast in the declare attackers step, before any block: the
/// prevention is about the attacker and "each creature blocking it", an
/// effect that does not lock its set (CR 611.2c), so the Ogre that blocks
/// afterwards deals no combat damage either. Both survive: the Captain-sized
/// attacker deals none, the blocker deals none.
#[test]
fn a_prevention_about_an_attacker_covers_a_creature_that_blocks_it_later() {
    let mut engine = bench(
        7305,
        cards(),
        [Seat::with(&[OGRE]).holding(&[FEINT]), Seat::with(&[OGRE])],
    );
    let ogres = objects(&engine, ZoneLocation::Battlefield, OGRE);
    let mine = *ogres
        .iter()
        .find(|o| {
            engine
                .state()
                .object(**o)
                .is_some_and(|x| x.controller == me())
        })
        .expect("my Ogre");
    let theirs = *ogres
        .iter()
        .find(|o| {
            engine
                .state()
                .object(**o)
                .is_some_and(|x| x.controller == them())
        })
        .expect("their Ogre");
    attack(&mut engine, me(), &[mine], them());
    walk_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == me())
            && e.state().turn.step == crate::turn::Step::DeclareAttackers
    });
    let feint = the(&engine, ZoneLocation::Hand(me()), FEINT);
    engine
        .apply(me(), PlayerAction::CastSpell { card: feint })
        .expect("Feint is cast");
    engine
        .apply(
            me(),
            PlayerAction::ChooseTargets {
                objects: vec![mine],
                players: vec![],
            },
        )
        .expect("the attacker is the target");
    block(&mut engine, them(), &[(theirs, mine)]);
    walk_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    for ogre in [mine, theirs] {
        assert_eq!(
            engine.state().object(ogre).map(|o| o.damage),
            Some(0),
            "no combat damage from either Ogre"
        );
    }
}

/// A spell whose second target is relative to the first is castable only
/// when some legal first target leaves a legal second one (CR 601.2c): with
/// my Trinket and only their Ogre it is not (nothing of theirs shares the
/// artifact type); once they control a Trinket it is.
#[test]
fn a_second_target_relative_to_the_first_decides_whether_a_spell_is_castable() {
    for (theirs, castable) in [(&[OGRE][..], false), (&[OGRE, TRINKET][..], true)] {
        let mut engine = bench(
            7306,
            cards(),
            [Seat::with(&[TRINKET]).holding(&[SWAP]), Seat::with(theirs)],
        );
        to_main(&mut engine, me());
        let swap = the(&engine, ZoneLocation::Hand(me()), SWAP);
        let (_, legal) = offer(&engine);
        assert_eq!(legal.castable.contains(&swap), castable, "{theirs:?}");
    }
}

/// The co-blockers condition, an intervening "if" about the creature the
/// source blocks (CR 603.4): beside another Wall and no non-Wall it holds;
/// alone, or beside a non-Wall, it does not. The probe gains 1 life.
#[test]
fn the_co_blockers_of_the_blocked_creature_decide_the_trigger() {
    for (with, fires) in [(Some(WALL), true), (None, false), (Some(OGRE), false)] {
        let mut theirs = vec![CALTROPS];
        theirs.extend(with);
        let mut engine = bench(7307, cards(), [Seat::with(&[OGRE]), Seat::with(&theirs)]);
        let attacker = objects(&engine, ZoneLocation::Battlefield, OGRE)
            .into_iter()
            .find(|o| {
                engine
                    .state()
                    .object(*o)
                    .is_some_and(|x| x.controller == me())
            })
            .expect("my Ogre");
        let caltrops = the(&engine, ZoneLocation::Battlefield, CALTROPS);
        let mut blocks = vec![(caltrops, attacker)];
        if let Some(index) = with {
            let other = objects(&engine, ZoneLocation::Battlefield, index)
                .into_iter()
                .find(|o| {
                    engine
                        .state()
                        .object(*o)
                        .is_some_and(|x| x.controller == them())
                })
                .expect("the co-blocker");
            blocks.push((other, attacker));
        }
        let before = life(&engine, them());
        attack(&mut engine, me(), &[attacker], them());
        block(&mut engine, them(), &blocks);
        walk_until(&mut engine, |e| {
            e.state().turn.step == crate::turn::Step::CombatEnd
        });
        assert_eq!(life(&engine, them()) == before + 1, fires, "{with:?}");
    }
}
