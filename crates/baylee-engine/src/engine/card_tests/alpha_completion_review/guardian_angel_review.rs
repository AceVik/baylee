//! Independent Guardian Angel: CR 116.2c, 400.7, 608.2b, and 702.61b.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::GrantedActionKind;
use baylee_core::generated::index;
use baylee_core::ids::{DamageSourceRef, GrantedActionId, TargetRef};
use baylee_core::mana::ManaColor;

const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn setup() -> Engine<RegistryLookup> {
    let mut board = vec![plains(); 8];
    board.extend([index::WALL_OF_SWORDS, index::ROD_OF_RUIN]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[index::GUARDIAN_ANGEL, ephemerate(), index::JUGGERNAUT])
        .battlefield(1, &[mountain(), mountain(), forest(), forest(), forest()])
        .hand(1, &[lightning_bolt(), lightning_bolt(), index::KROSAN_GRIP])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    engine
}

fn tap_white(engine: &mut Engine<RegistryLookup>) {
    let source = all_on_battlefield(engine, USER, plains())
        .into_iter()
        .find(|id| !is_tapped(engine, *id))
        .unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source })
        .unwrap();
}

fn angel(engine: &mut Engine<RegistryLookup>, x: u32, creature: Option<ObjectId>) {
    priority(engine, USER);
    for _ in 0..=x {
        tap_white(engine);
    }
    cast_with_floating(engine, USER, index::GUARDIAN_ANGEL);
    engine.apply(USER, PlayerAction::ChooseNumber(x)).unwrap();
    if let Some(creature) = creature {
        aim(engine, vec![creature], vec![]);
    } else {
        aim(engine, vec![], vec![USER]);
    }
}

fn offer(engine: &Engine<RegistryLookup>) -> (GrantedActionId, TargetRef) {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority expected")
    };
    let action = legal
        .granted_actions
        .iter()
        .find(|action| {
            matches!(
                action.effect,
                GrantedActionKind::PreventNextDamage { amount: 1, .. }
            )
        })
        .unwrap();
    let GrantedActionKind::PreventNextDamage { target, .. } = action.effect else {
        unreachable!()
    };
    (action.id, target)
}

fn pay(engine: &mut Engine<RegistryLookup>, id: GrantedActionId) {
    priority(engine, USER);
    if engine.state().players[0]
        .mana_pool
        .available(ManaColor::White)
        == 0
    {
        tap_white(engine);
    }
    let stack = engine.state().zones.list(ZoneLocation::Stack).clone();
    engine
        .apply(USER, PlayerAction::TakeGrantedAction { id })
        .unwrap();
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack), &stack);
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
}

fn finish_damage(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..60 {
        if stack_is_empty(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseDamageEffect {
                player,
                choice,
                options,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseDamageEffect {
                            choice,
                            effect: options[0].id,
                        },
                    )
                    .unwrap();
            }
            Pending::AllocatePrevention {
                player,
                choice,
                damage,
                total,
                ..
            } => {
                assert_eq!(damage.len(), 1);
                engine
                    .apply(
                        player,
                        PlayerAction::AllocatePrevention {
                            choice,
                            allocation: vec![(damage[0].id, total.min(damage[0].amount))],
                        },
                    )
                    .unwrap();
            }
            pending => panic!("unexpected damage choice: {pending:?}"),
        }
    }
    panic!("damage did not finish");
}

fn bolt(engine: &mut Engine<RegistryLookup>, creature: Option<ObjectId>) {
    priority(engine, OTHER);
    cast_from_hand(engine, OTHER, lightning_bolt());
    if let Some(creature) = creature {
        aim(engine, vec![creature], vec![]);
    } else {
        aim(engine, vec![], vec![USER]);
    }
    finish_damage(engine);
}

#[test]
fn guardian_review_initial_x_and_repeated_paid_one_point_shields() {
    for (x, payments, life) in [(0, 2, 19), (2, 1, 20)] {
        let mut engine = setup();
        angel(&mut engine, x, None);
        pass_until(&mut engine, stack_is_empty);
        priority(&mut engine, USER);
        let (action, target) = offer(&engine);
        assert_eq!(target, TargetRef::Player(USER));
        for _ in 0..payments {
            pay(&mut engine, action);
        }
        bolt(&mut engine, None);
        assert_eq!(engine.state().players[0].life, life);
    }
}

#[test]
fn guardian_review_resolved_permission_keeps_old_recipient_after_real_blink() {
    let mut engine = setup();
    let wall = on_battlefield(&engine, USER, index::WALL_OF_SWORDS).unwrap();
    let old = DamageSourceRef {
        object: wall,
        version: engine.state().object(wall).unwrap().version,
    };
    angel(&mut engine, 2, Some(wall));
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, USER);
    let (action, target) = offer(&engine);
    assert_eq!(target, TargetRef::Object(old));
    cast_from_hand(&mut engine, USER, ephemerate());
    aim(&mut engine, vec![wall], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_ne!(engine.state().object(wall).unwrap().version, old.version);
    pay(&mut engine, action);
    assert_eq!(offer(&engine).1, TargetRef::Object(old));
    bolt(&mut engine, Some(wall));
    assert_eq!(engine.state().object(wall).unwrap().damage, 3);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn guardian_review_all_illegal_original_targets_grant_no_permission() {
    let mut engine = setup();
    let wall = on_battlefield(&engine, USER, index::WALL_OF_SWORDS).unwrap();
    angel(&mut engine, 0, Some(wall));
    priority(&mut engine, USER);
    cast_from_hand(&mut engine, USER, ephemerate());
    aim(&mut engine, vec![wall], vec![]);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, USER);
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(legal.granted_actions.is_empty());
    assert!(in_graveyard(&engine, USER, index::GUARDIAN_ANGEL).is_some());
    bolt(&mut engine, Some(wall));
    assert_eq!(engine.state().object(wall).unwrap().damage, 3);
}

#[test]
fn guardian_review_permission_and_paid_shield_expire_at_cleanup() {
    let mut engine = setup();
    angel(&mut engine, 0, None);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, USER);
    let action = offer(&engine).0;
    pay(&mut engine, action);
    reach_their_main_phase(&mut engine, OTHER);
    priority(&mut engine, USER);
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: action })
            .is_err()
    );
    bolt(&mut engine, None);
    assert_eq!(engine.state().players[0].life, 17);
}

#[test]
fn guardian_review_special_action_is_allowed_with_real_split_second_spell() {
    let mut engine = setup();
    angel(&mut engine, 0, None);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, USER);
    let action = offer(&engine).0;
    let rod = on_battlefield(&engine, USER, index::ROD_OF_RUIN).unwrap();
    priority(&mut engine, OTHER);
    for source in all_on_battlefield(&engine, OTHER, forest()) {
        engine
            .apply(OTHER, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    cast_with_floating(&mut engine, OTHER, index::KROSAN_GRIP);
    aim(&mut engine, vec![rod], vec![]);
    priority(&mut engine, USER);
    pay(&mut engine, action);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, USER, index::ROD_OF_RUIN).is_some());
    bolt(&mut engine, None);
    assert_eq!(engine.state().players[0].life, 18);
}

#[test]
fn guardian_review_instant_timing_permission_is_not_a_mana_payment_action() {
    let mut engine = setup();
    angel(&mut engine, 0, None);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, USER);
    let action = offer(&engine).0;
    engine
        .apply(USER, PlayerAction::TakeGrantedAction { id: action })
        .unwrap();
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == USER));
    assert!(
        engine
            .apply(USER, PlayerAction::TakeGrantedAction { id: action })
            .is_err()
    );
    tap_white(&mut engine);
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert!(stack_is_empty(&engine));
    priority(&mut engine, USER);
    assert_eq!(
        offer(&engine).0,
        action,
        "permission resumes at ordinary priority"
    );
    bolt(&mut engine, None);
    assert_eq!(engine.state().players[0].life, 18);
}
