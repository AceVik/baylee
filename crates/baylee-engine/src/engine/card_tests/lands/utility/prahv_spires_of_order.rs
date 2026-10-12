//! `cards/lands/utility/prahv_spires_of_order.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Prahv, Spires of Order: "{T}: Add {C}." / "{4}{W}{U}, {T}: Prevent all damage a source of your choice would deal this turn."
/// The land taps for its base mana ability to add {C} to the mana pool.
#[test]
fn prahv_spires_of_order_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(117, forest())
        .battlefield(0, &[prahv_spires_of_order()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let prahv = on_battlefield(&engine, p0, prahv_spires_of_order()).expect("Prahv deployed");
    activate(&mut engine, p0, prahv_spires_of_order(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, prahv));
}

use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn q_prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    q_mana(&mut e);
    e
}
fn q_mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn q_obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn q_priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn q_cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    q_priority(e, p);
    q_mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn q_targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn q_cast_target(e: &mut Engine<RegistryLookup>, c: CardIndex, target: ObjectId) {
    q_cast(e, P0, c);
    q_targets(e, vec![target], vec![]);
    pass_until(e, stack_is_empty);
}
fn q_activate(e: &mut Engine<RegistryLookup>, c: CardIndex, i: u32) {
    q_priority(e, P0);
    q_mana(e);
    let source = q_obj(e, P0, c);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: i,
        },
    )
    .unwrap();
}
fn q_next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p)
    });
}
fn q_answer(e: &mut Engine<RegistryLookup>, yes: bool) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::YesNo(yes)).unwrap();
}

#[test]
fn prahv_chooses_shrouded_source_on_resolution_and_prevents_noncombat() {
    let mut e = q_prepared(
        &[],
        &[index::PRAHV_SPIRES_OF_ORDER],
        &[index::PRODIGAL_SORCERER],
    );
    let sorcerer = q_obj(&e, P1, index::PRODIGAL_SORCERER);
    // Explicit shroud fixture isolates the untargeted source choice.
    let original = keywords(&e, sorcerer);
    e.state.object_mut(sorcerer).unwrap().base_mut().keywords = original.union(KeywordSet::SHROUD);
    e.state.invalidate_projections();
    e.sync_static_effects();
    q_activate(&mut e, index::PRAHV_SPIRES_OF_ORDER, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
        ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    let source = baylee_core::ids::DamageSourceRef {
        object: sorcerer,
        version: e.state.object(sorcerer).unwrap().version,
    };
    assert_eq!(player, P0);
    assert!(options.contains(&source));
    e.apply(P0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    q_priority(&mut e, P1);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: sorcerer,
            ability_index: 0,
        },
    )
    .unwrap();
    q_targets(&mut e, vec![], vec![P0]);
    let life = e.state.players[0].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life);
}

#[test]
fn prahv_prevents_repeated_damage_from_one_source_but_not_its_new_incarnation() {
    let mut e = q_prepared(
        &[index::TWIDDLE, index::UNSUMMON],
        &[index::PRAHV_SPIRES_OF_ORDER],
        &[index::PRODIGAL_SORCERER],
    );
    q_next_main(&mut e, P1);
    let sorcerer = q_obj(&e, P1, index::PRODIGAL_SORCERER);
    q_activate(&mut e, index::PRAHV_SPIRES_OF_ORDER, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    let source = baylee_core::ids::DamageSourceRef {
        object: sorcerer,
        version: e.state.object(sorcerer).unwrap().version,
    };
    assert!(options.contains(&source));
    e.apply(P0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    let life = e.state.players[0].life;
    for attempt in 0..2 {
        if attempt == 1 {
            q_cast(&mut e, P0, index::TWIDDLE);
            q_targets(&mut e, vec![sorcerer], vec![]);
            pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
            q_answer(&mut e, true);
            pass_until(&mut e, stack_is_empty);
        }
        q_priority(&mut e, P1);
        e.apply(
            P1,
            PlayerAction::ActivateAbility {
                source: sorcerer,
                ability_index: 0,
            },
        )
        .unwrap();
        q_targets(&mut e, vec![], vec![P0]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.players[0].life, life);
    }
    q_cast_target(&mut e, index::UNSUMMON, sorcerer);
    // The real bounce changes incarnation; recast the same card through normal casting.
    // Give haste only as an explicit fixture so this timing edge stays within the shield's turn.
    q_cast(&mut e, P1, index::PRODIGAL_SORCERER);
    pass_until(&mut e, stack_is_empty);
    let new_version = e.state.object(sorcerer).unwrap().version;
    assert_ne!(source.version, new_version);
    let original = keywords(&e, sorcerer);
    e.state.object_mut(sorcerer).unwrap().base_mut().keywords = original.union(KeywordSet::HASTE);
    e.state.invalidate_projections();
    e.refresh_offer();
    q_priority(&mut e, P1);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: sorcerer,
            ability_index: 0,
        },
    )
    .unwrap();
    q_targets(&mut e, vec![], vec![P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.players[0].life, life - 1);
}

// Append to lands/utility/prahv_spires_of_order.rs (existing q_* helpers).
#[test]
fn prahv_chosen_permanent_spell_prevents_combat_and_sacrificed_sources_damage() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::PRAHV_SPIRES_OF_ORDER, index::GRIZZLY_BEARS])
        .hand(1, &[index::MOGG_FANATIC])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    q_next_main(&mut e, P1);
    let fanatic = in_hand(&e, P1, index::MOGG_FANATIC).unwrap();
    q_cast(&mut e, P1, index::MOGG_FANATIC);
    let chosen = baylee_core::ids::DamageSourceRef {
        object: fanatic,
        version: e.state.object(fanatic).unwrap().version,
    };
    q_activate(&mut e, index::PRAHV_SPIRES_OF_ORDER, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        choice, options, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert!(options.contains(&chosen));
    e.apply(
        P0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: chosen,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(fanatic).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    // Haste fixture avoids changing turns (the actual shield expires at cleanup).
    let original = keywords(&e, fanatic);
    e.state.object_mut(fanatic).unwrap().base_mut().keywords = original.union(KeywordSet::HASTE);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    let life = e.state.players[0].life;
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..}if *player==P1),
    );
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(fanatic, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(),Pending::Priority{player,..}if *player==P1)
    });
    assert_eq!(
        e.state.players[0].life, life,
        "chosen spell continuity prevents combat damage from resolved permanent"
    );
    let bear = q_obj(&e, P0, index::GRIZZLY_BEARS);
    e.apply(
        P1,
        PlayerAction::ActivateAbility {
            source: fanatic,
            ability_index: 0,
        },
    )
    .unwrap();
    q_targets(&mut e, vec![bear], vec![]);
    assert_eq!(
        e.state.object(fanatic).unwrap().zone,
        crate::zone::Zone::Graveyard
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.object(bear).unwrap().damage,
        0,
        "all-from shield also covers different recipient and source LKI after sacrifice"
    );
    assert_eq!(e.state.players[0].life, life);
}
