//! Wall of Shadows: actual targeting and blocked-source damage actions.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player:p,..}if *p==player),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, c: CardIndex) {
    priority(e, P0);
    mana(e);
    let card = in_hand(e, P0, c).unwrap();
    e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
}
fn target(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    let p = e.pending().asked().unwrap();
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![id],
            players: vec![],
        },
    )
    .unwrap();
}
fn tap_with_twiddle(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    cast(e, index::TWIDDLE);
    target(e, id);
    pass_until(e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(e, stack_is_empty);
}
#[test]
fn wall_only_spell_cannot_choose_shadows_but_broad_spell_can_when_only_walls_exist() {
    let mut e = prepared(
        &[index::TUNNEL, index::LIGHTNING_BOLT],
        &[index::WALL_OF_SHADOWS, index::WALL_OF_STONE],
        &[],
    );
    let shadow = obj(&e, P0, index::WALL_OF_SHADOWS);
    let stone = obj(&e, P0, index::WALL_OF_STONE);
    cast(&mut e, index::TUNNEL);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(options.contains(&stone));
    assert!(!options.contains(&shadow));
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![shadow],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    target(&mut e, stone);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, index::LIGHTNING_BOLT);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(
        options.contains(&shadow),
        "broad domain must not become Wall-only when every surviving creature is a Wall"
    );
    target(&mut e, shadow);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(shadow).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
}

#[test]
fn wall_only_activated_ability_excludes_shadows() {
    let mut e = prepared(
        &[],
        &[
            index::DWARVEN_DEMOLITION_TEAM,
            index::WALL_OF_SHADOWS,
            index::WALL_OF_STONE,
        ],
        &[],
    );
    let shadow = obj(&e, P0, index::WALL_OF_SHADOWS);
    let stone = obj(&e, P0, index::WALL_OF_STONE);
    activate(&mut e, P0, index::DWARVEN_DEMOLITION_TEAM, 0);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(!options.contains(&shadow));
    assert!(options.contains(&stone));
    target(&mut e, stone);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(shadow).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn wall_prevents_all_damage_from_actual_creature_it_blocks() {
    let mut e = prepared(
        &[index::TWIDDLE],
        &[index::PRODIGAL_SORCERER],
        &[index::WALL_OF_SHADOWS],
    );
    let sorcerer = obj(&e, P0, index::PRODIGAL_SORCERER);
    let wall = obj(&e, P1, index::WALL_OF_SHADOWS);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(sorcerer, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, sorcerer)],
        },
    )
    .unwrap();
    priority(&mut e, P0);
    tap_with_twiddle(&mut e, sorcerer);
    activate(&mut e, P0, index::PRODIGAL_SORCERER, 0);
    target(&mut e, wall);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(wall).unwrap().damage,
        0,
        "blocked source's activated noncombat damage is also prevented"
    );
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
    });
    assert_eq!(
        e.state.object(wall).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn wall_only_spell_copy_retains_restriction_when_retargeted() {
    let mut e = prepared(
        &[index::TUNNEL, index::FORK],
        &[
            index::WALL_OF_SHADOWS,
            index::WALL_OF_STONE,
            index::WALL_OF_STONE,
        ],
        &[],
    );
    let shadow = obj(&e, P0, index::WALL_OF_SHADOWS);
    let stone = obj(&e, P0, index::WALL_OF_STONE);
    cast(&mut e, index::TUNNEL);
    target(&mut e, stone);
    let tunnel = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, index::FORK);
    target(&mut e, tunnel);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(!options.contains(&shadow));
    assert!(
        options.iter().any(|id| *id != stone),
        "another legal Wall makes a genuine retarget choice"
    );
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![shadow],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    // Empty answer retains copied target, rather than selecting an illegal one.
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(shadow).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

#[test]
fn wall_only_original_spell_cannot_be_retargeted_to_shadows() {
    let mut e = prepared(
        &[index::TUNNEL, index::DEFLECTING_SWAT],
        &[
            index::WALL_OF_SHADOWS,
            index::WALL_OF_STONE,
            index::WALL_OF_STONE,
        ],
        &[],
    );
    let shadow = obj(&e, P0, index::WALL_OF_SHADOWS);
    let stone = obj(&e, P0, index::WALL_OF_STONE);
    cast(&mut e, index::TUNNEL);
    target(&mut e, stone);
    let tunnel = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    cast(&mut e, index::DEFLECTING_SWAT);
    target(&mut e, tunnel);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("{:?}", e.pending())
    };
    assert!(!options.contains(&shadow));
    assert!(
        options.iter().any(|id| *id != stone),
        "another legal Wall makes a genuine retarget choice"
    );
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![shadow],
                players: vec![]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
    e.apply(
        P0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(shadow).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
}

mod permission_fixture;
