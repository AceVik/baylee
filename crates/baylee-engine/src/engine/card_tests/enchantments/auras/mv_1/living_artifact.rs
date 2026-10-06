//! `cards/enchantments/auras/mv_1/living_artifact.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Living Artifact's Enchant line: offered only an artifact, never the
/// Elves, and ends attached to Sol Ring; its two triggers are played under
/// the Living Artifact heading below.
#[test]
fn living_artifact_attaches_only_to_an_artifact() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), sol_ring(), llanowar_elves()])
        .hand(0, &[living_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rock = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    attaches_only_to(&mut engine, p0, living_artifact(), rock, elf);
}

/// "Whenever you're dealt damage, put that many vitality counters on this
/// Aura": the Bear (2) and the Elephant (3) go through unblocked for one
/// trigger worth 5 (CR 510.2, 603.2c — one damage event, one trigger for
/// all of it); a Lightning Bolt in the same turn is a separate event and a
/// second trigger for 3 more; the third attacker's damage, blocked by
/// Tyrranax Rex, lands on a creature and adds nothing; and a later, huge hit
/// the *opponent* takes is not "you" and adds nothing either.
#[allow(clippy::too_many_lines)] // one printed card, played end to end
#[test]
fn living_artifact_counts_one_trigger_per_damage_event_and_none_for_a_creature_or_the_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), sol_ring(), tyrranax_rex()])
        .hand(0, &[living_artifact()])
        .battlefield(
            1,
            &[
                grizzly_bears(),
                wild_elephant(),
                llanowar_elves(),
                mountain(),
            ],
        )
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    cast_from_hand(&mut engine, p0, living_artifact());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, living_artifact()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(ring),
        "attached to the artifact it enchanted"
    );

    // p1's turn: the Bear (2) and the Elephant (3) go through unblocked; the
    // Elf (1) is blocked by Tyrranax Rex, so its one point lands on a
    // creature instead of on p0.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bear is seated");
    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("the Elephant is seated");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");
    let rex = on_battlefield(&engine, p0, tyrranax_rex()).expect("Tyrranax Rex is seated");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (bear, Defender::Player(p0)),
                    (elephant, Defender::Player(p0)),
                    (elf, Defender::Player(p0)),
                ],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareBlockers {
                blockers: vec![(rex, elf)],
            },
        )
        .expect("Tyrranax Rex may block the Elf");
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        5,
        "the Bear's and the Elephant's total"
    );

    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        5,
        "that many: the Bear's and the Elephant's total"
    );
    assert_eq!(
        counters_on(&engine, ring, counters::VITALITY),
        0,
        "the counters land on the Aura, never on the artifact it enchants"
    );
    assert_eq!(
        vitality_counter_changes(&engine, aura),
        1,
        "one trigger for the whole damage step"
    );
    assert_eq!(
        engine.state().object(rex).map(|o| o.damage),
        Some(1),
        "the Elf's one point landed on the blocker instead of on p0"
    );

    // Still p1's turn: a Lightning Bolt at p0 is a second, separate event.
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("Lightning Bolt may name a player");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        8,
        "5 from combat plus the Bolt's 3"
    );
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        8,
        "5 plus the Bolt's 3"
    );
    assert_eq!(
        counters_on(&engine, ring, counters::VITALITY),
        0,
        "still nothing on the enchanted artifact itself"
    );
    assert_eq!(
        vitality_counter_changes(&engine, aura),
        2,
        "a second trigger for a second event"
    );

    // p0's own upkeep comes first, with 8 counters standing: decline the
    // Aura's own removal so this board keeps exactly the number the combat
    // and the Bolt put there, for what the attack below is about.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::MayDo,
                    ..
                }
            )
    });
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is one of the two answers");

    // p0's turn: Tyrranax Rex, unblocked, hits the opponent for 8 — real
    // damage, just not damage dealt to "you" (the Aura's controller).
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().turn.active, p0, "p0's own turn now");
    attack_and_collect_blocks(&mut engine, rex, p1);
    let Pending::ChooseBlockers { player, .. } = engine.pending().clone() else {
        panic!("expected the block question, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("declining every block is legal");
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[1],
        8,
        "Tyrranax Rex, unblocked, on the opponent"
    );

    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        8,
        "the opponent's damage is not \"you\": nothing added"
    );
    assert_eq!(
        counters_on(&engine, ring, counters::VITALITY),
        0,
        "still nothing on the enchanted artifact itself"
    );
    assert_eq!(
        vitality_counter_changes(&engine, aura),
        2,
        "no third trigger"
    );
}

/// First strike opens a combat damage step of its own (CR 510.4): Serra
/// Zealot's first-strike point and Grizzly Bears' regular two are two
/// separate damage events, so the Aura reads two triggers rather than
/// folding both into the one that "two unblocked attackers" would otherwise
/// suggest.
#[test]
fn living_artifact_gets_a_separate_trigger_for_each_combat_damage_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), sol_ring()])
        .hand(0, &[living_artifact()])
        .battlefield(1, &[serra_zealot(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    cast_from_hand(&mut engine, p0, living_artifact());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, living_artifact()).expect("the Aura resolved");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    let zealot = on_battlefield(&engine, p1, serra_zealot()).expect("the Zealot is seated");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bear is seated");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(zealot, Defender::Player(p0)), (bear, Defender::Player(p0))],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamageFirst && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        1,
        "the first-strike step, alone"
    );
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        1,
        "the first-strike step, alone"
    );
    assert_eq!(
        vitality_counter_changes(&engine, aura),
        1,
        "one trigger for it"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatDamage && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        3,
        "1 plus the regular step's 2"
    );
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        3,
        "1 plus the regular step's 2"
    );
    assert_eq!(
        vitality_counter_changes(&engine, aura),
        2,
        "a second trigger for the second step, not folded into the first"
    );
}

/// "At the beginning of your upkeep, you may remove a vitality counter from
/// this Aura. If you do, you gain 1 life.": asked only in its controller's
/// own upkeep, and only while a counter stands to be removed (CR 118.12,
/// 608.2d); a no changes neither the counters nor the life, and a yes
/// removes exactly one counter and gains exactly 1.
#[allow(clippy::too_many_lines)] // one printed card, played end to end
#[test]
fn living_artifact_offers_to_remove_a_vitality_counter_only_in_its_controllers_upkeep_and_only_with_one_standing()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), sol_ring()])
        .hand(0, &[living_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    cast_from_hand(&mut engine, p0, living_artifact());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, living_artifact()).expect("the Aura resolved");

    let reach_upkeep = |engine: &mut Engine<RegistryLookup>, seat: PlayerId| {
        pass_until(engine, |e| {
            e.state().turn.active == seat
                && e.state().turn.step == crate::turn::Step::Upkeep
                && (stack_is_empty(e)
                    || matches!(
                        e.pending(),
                        Pending::YesNo {
                            prompt: YesNoPrompt::MayDo,
                            ..
                        }
                    ))
        });
    };

    // Turn 2, p1's upkeep, no counters yet: nothing is asked. With zero
    // counters standing this cannot yet tell "never your upkeep" apart from
    // "nothing to remove" — the turn-4 check below, with 3 counters
    // standing and still p1's upkeep, is the one that isolates the reason.
    reach_upkeep(&mut engine, p1);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "nothing asked yet, for either possible reason"
    );

    // Turn 3, p0's own upkeep, still no counters: the removal can't be
    // paid, so nobody is asked.
    reach_upkeep(&mut engine, p0);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "no counters to remove: nobody is asked"
    );

    // Seed 3 counters by hand for what follows.
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .object_mut(aura)
        .expect("the Aura is on the battlefield")
        .counters
        .set(counters::VITALITY, 3);
    engine.refresh_offer();

    // Turn 4, p1's upkeep, counters standing: still not this Aura's
    // controller's upkeep, so still nothing asked — the counters were never
    // what gated the question.
    reach_upkeep(&mut engine, p1);
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "the opponent's upkeep, even with counters standing"
    );

    // Turn 5, p0's own upkeep, counters standing: asked, and a no changes
    // neither the counters nor the life.
    reach_upkeep(&mut engine, p0);
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        ),
        "p0's own upkeep, a counter standing: asked"
    );
    let life_before_no = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        3,
        "a no takes nothing off"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before_no,
        "and gains nothing"
    );

    // Escape this same upkeep before asking for the next one, or the pass
    // would stop on the very spot it already stands on.
    pass_until(&mut engine, |e| {
        !(e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep)
    });

    // Turn 7, p0's own upkeep again (turn 6 is p1's), still 3 counters: this
    // time a yes removes exactly one and gains exactly 1 life.
    reach_upkeep(&mut engine, p0);
    assert!(
        matches!(
            engine.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        ),
        "p0's own upkeep again, still a counter standing: asked again"
    );
    let life_before_yes = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        2,
        "one came off"
    );
    assert_eq!(engine.state().players[0].life, life_before_yes + 1);
}

/// A Lightning Bolt at one of the caster's own creatures, off the stack
/// entirely and apart from combat, is not damage dealt to the Aura's
/// controller: no vitality counters are added. (The blocked-Elf leg above
/// already reads this for combat damage; this reads it for a direct spell.)
#[test]
fn living_artifact_gets_no_counter_when_a_bolt_hits_a_creature_instead_of_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), sol_ring(), llanowar_elves()])
        .hand(0, &[living_artifact()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    cast_from_hand(&mut engine, p0, living_artifact());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, living_artifact()).expect("the Aura resolved");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves is seated");
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("a creature is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "damage to a creature, not its controller"
    );
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        0,
        "nothing added: the Bolt never hit \"you\""
    );
}

/// Healing Salve's "prevent the next 3 damage" (mode two) shields the
/// Aura's own controller before a Lightning Bolt lands: prevented damage is
/// not dealt (CR 615.6, 120.8), so no vitality counters are added.
#[test]
fn living_artifact_gets_no_counter_when_damage_to_its_controller_is_fully_prevented() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains(), sol_ring()])
        .hand(0, &[living_artifact(), healing_salve()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, sol_ring()).expect("Sol Ring is seated");
    cast_from_hand(&mut engine, p0, living_artifact());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ring],
                players: vec![],
            },
        )
        .expect("Sol Ring is an artifact");
    pass_until(&mut engine, stack_is_empty);
    let aura = on_battlefield(&engine, p0, living_artifact()).expect("the Aura resolved");

    cast_with_floating(&mut engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("mode 1 is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("p0 is any target");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().per_turn.damage_dealt_to[0],
        0,
        "the shield absorbed all 3: nothing was dealt"
    );
    assert_eq!(
        counters_on(&engine, aura, counters::VITALITY),
        0,
        "no damage, no trigger, no counters"
    );
}
