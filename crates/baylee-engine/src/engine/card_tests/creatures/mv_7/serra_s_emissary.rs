//! `cards/creatures/mv_7/serra_s_emissary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::types::TypeSet;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn emissary() -> CardIndex {
    card_index("7a56c6e1-0509-4783-9b29-cf3163977166")
}

/// Cast the seven-drop for its printed cost and answer its actual entry question.
fn cast_emissary(engine: &mut Engine<RegistryLookup>, chosen: TypeSet) -> ObjectId {
    pay_exactly_and_cast(engine, emissary());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardType { .. })
    });
    let Pending::ChooseCardType { options, .. } = engine.pending().clone() else {
        unreachable!("the entry question is pending")
    };
    assert!(options.contains(&chosen));
    // The fifteen card types of CR 205.2a; Attraction is an artifact type.
    assert_eq!(options.len(), 15);
    assert!(!options.contains(&TypeSet::ATTRACTION));
    engine
        .apply(P0, PlayerAction::ChooseCardType(chosen))
        .expect("the offered card type is a legal choice");
    pass_until(engine, stack_is_empty);
    let id = on_battlefield(engine, P0, emissary()).expect("the Emissary entered");
    assert!(keywords(engine, id).contains(KeywordSet::FLYING));
    id
}

#[test]
fn serra_s_emissary_protects_its_controller_and_creatures_from_the_chosen_type() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                grizzly_bears(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[emissary()])
        .battlefield(1, &[gray_ogre()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let protected = cast_emissary(&mut engine, TypeSet::INSTANT);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let ogre = on_battlefield(&engine, P1, gray_ogre()).unwrap();
    reach_their_main_phase(&mut engine, P1);
    engine.state.players[1].mana_pool.add(ManaColor::Red, 1);
    engine.refresh_offer();
    cast_with_floating(&mut engine, P1, lightning_bolt());
    let (objects, players) = target_menu(&engine);
    assert!(!objects.contains(&protected));
    assert!(!objects.contains(&bears));
    assert!(objects.contains(&ogre));
    assert!(!players.contains(&P0));
    assert!(players.contains(&P1));
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![P0],
                },
            )
            .is_err(),
        "the offered targets and apply enforce the same protection"
    );
    point_at(&mut engine, P1, &[], &[P1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state.players[0].life, 20);
    assert_eq!(engine.state.players[1].life, 17);
}

#[test]
fn serra_s_emissary_creature_choice_prevents_combat_damage_and_keeps_flying() {
    let mut engine = table7(
        &[
            grizzly_bears(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
        ],
        &[emissary()],
        &[gray_ogre(), gray_ogre()],
    );
    let angel = cast_emissary(&mut engine, TypeSet::CREATURE);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let attackers = all_on_battlefield(&engine, P1, gray_ogre());
    assert_eq!(attackers.len(), 2);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|attacker| (*attacker, Defender::Player(P0)))
                    .collect(),
            },
        )
        .expect("both Ogres attack");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == P0),
    );
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(angel, attackers[0]), (bears, attackers[1])],
            },
        )
        .expect("both protected creatures may block");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().object(angel).unwrap().damage, 0);
    assert_eq!(engine.state().object(bears).unwrap().damage, 0);
    assert_eq!(all_on_battlefield(&engine, P1, gray_ogre()).len(), 0);

    // A separate ground attacker cannot block the flying Emissary even when
    // the chosen type does not supply protection from that attacker.
    let mut flying = table7(
        &[
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
            plains(),
        ],
        &[emissary()],
        &[gray_ogre()],
    );
    let angel = cast_emissary(&mut flying, TypeSet::SORCERY);
    let turn = flying.state().turn.number;
    pass_until(&mut flying, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
    });
    let blocker = on_battlefield(&flying, P1, gray_ogre()).unwrap();
    pass_until(
        &mut flying,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P0),
    );
    flying
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(P1))],
            },
        )
        .expect("the Emissary attacks");
    pass_until(&mut flying, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        flying
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(blocker, angel)]
                }
            )
            .is_err(),
        "a ground Ogre cannot block the flying Emissary"
    );
}
