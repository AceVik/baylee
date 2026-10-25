use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::{Duration, Filter, Modifier, ZoneRef};

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

/// An ability of seat 0's with `targets`, resolving `effects` in order.
fn resolve(state: &mut GameState, targets: &[ObjectId], effects: Vec<Effect>) {
    let mut res = Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: me(),
        effects,
        pc: 0,
        targets: SmallVec::from_slice(targets),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        this_way_from: None,
        coin_stake: None,
    };
    assert!(matches!(run(state, &mut res), Flow::Complete));
    state.refresh_characteristics();
}

/// A 3/3 seat 1 owns and seat 0 has taken, on a board where every
/// refresh reaches every zone: Past in Flames' filter names the
/// graveyard (`state::filter_reaches_other_zones`).
fn taken() -> (GameState, ObjectId) {
    static FLASHBACK_REACH: Filter = Filter::And(&[
        Filter::INSTANT_OR_SORCERY,
        Filter::InZone(ZoneRef::Graveyard),
        Filter::OwnedByYou,
    ]);
    let mut state = GameState::from_preset(&preset(619, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let name = state.names.intern("Creature");
    let it = state.create_bare(
        them(),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let obj = state.object_mut(it).expect("fresh");
    let mut base = (*obj.base).clone();
    base.types = baylee_core::types::TypeSet::CREATURE;
    base.power = Some(3);
    base.toughness = Some(3);
    obj.base = std::sync::Arc::new(base);
    state.invalidate_projections();
    gain_control(&mut state, &[(it, me())]);
    resolve(
        &mut state,
        &[],
        vec![Effect::continuous(
            &FLASHBACK_REACH,
            Modifier::GrantsFlashback,
            Duration::UntilEndOfTurn,
        )],
    );
    let obj = state.object(it).expect("still there");
    assert_eq!((obj.owner, obj.controller), (them(), me()), "taken");
    (state, it)
}

/// Swords to Plowshares: "Exile target creature. Its controller gains
/// life equal to its power." The second sentence is read after the
/// first has exiled the creature, so "its controller" is the one it had
/// as it last existed on the battlefield (CR 608.2h): seat 0, who had
/// taken it. The field on the exiled card had been settled to seat 1's
/// default by a refresh that reaches every zone.
#[test]
fn its_controller_is_the_one_it_had_on_the_battlefield() {
    let (mut state, it) = taken();
    let life = |state: &GameState| (state.players[0].life, state.players[1].life);
    let (mine, theirs) = life(&state);
    resolve(
        &mut state,
        &[it],
        vec![
            Effect::exile(TargetSpec::Object(&Filter::CREATURE)),
            Effect::GainLifeFor {
                amount: Amount::TargetPower,
                who: PlayerRel::ControllerOfTarget,
            },
        ],
    );
    assert_eq!(
        state.object(it).map(|o| o.zone),
        Some(crate::zone::Zone::Exile)
    );
    assert_eq!(life(&state), (mine + 3, theirs));
}

/// Choosing who decides does not change the resolving controller: "you"
/// remains seat zero, while only the chosen recipient may accept the gift.
#[test]
fn chosen_player_may_accept_control_without_becoming_effect_controller() {
    static ACCEPT: &[Effect] = &[
        Effect::GainControlAll {
            filter: &Filter::Reference,
            to: PlayerRel::Chosen,
        },
        Effect::GainLifeFor {
            amount: Amount::Fixed(2),
            who: PlayerRel::You,
        },
    ];
    for yes in [false, true] {
        let (mut state, it) = taken();
        let before = (state.players[0].life, state.players[1].life);
        let mut res = Resolution {
            source: ObjectId::NO_SOURCE,
            on_stack: ObjectId::NO_SOURCE,
            controller: me(),
            effects: vec![
                Effect::ChoosePlayerFrom {
                    among: PlayerRel::EachOtherPlayer,
                },
                Effect::MayDoFor {
                    player: PlayerRel::Chosen,
                    effects: ACCEPT,
                },
            ],
            pc: 0,
            targets: SmallVec::from_slice(&[it]),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            subject: SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
            loot: None,
            draw_watch: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
            this_way_from: None,
            coin_stake: None,
        };
        assert!(
            matches!(run(&mut state, &mut res), Flow::Wait(Pending::YesNo { player, .. }) if player == them())
        );
        assert!(matches!(
            resume_may_do(&mut state, &mut res, yes),
            Flow::Complete
        ));
        state.refresh_characteristics();
        assert_eq!(
            state.object(it).unwrap().controller,
            if yes { them() } else { me() }
        );
        assert_eq!(state.object(it).unwrap().owner, them());
        assert_eq!(state.players[0].life, before.0 + if yes { 2 } else { 0 });
        assert_eq!(state.players[1].life, before.1);
    }
}

#[test]
fn greatest_exchange_is_atomic_when_one_permanent_cannot_change_control() {
    for guarded in [false, true] {
        let (mut state, first) = taken();
        let name = state.names.intern("exchange partner");
        let second = state.create_bare(
            them(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let base = state.object(first).unwrap().base.clone();
        state.object_mut(second).unwrap().base = base;
        state.invalidate_projections();
        state.refresh_characteristics();
        if guarded {
            resolve(
                &mut state,
                &[first],
                vec![Effect::continuous(
                    &Filter::Reference,
                    Modifier::OtherPlayersCantGainControl,
                    Duration::Indefinitely,
                )],
            );
        }
        resolve(
            &mut state,
            &[],
            vec![
                Effect::ChoosePlayerFrom {
                    among: PlayerRel::EachOtherPlayer,
                },
                Effect::ExchangeGreatestManaValue {
                    filter: &Filter::CREATURE,
                    with: PlayerRel::Chosen,
                },
            ],
        );
        assert_eq!(
            state.object(first).unwrap().controller,
            if guarded { me() } else { them() }
        );
        assert_eq!(
            state.object(second).unwrap().controller,
            if guarded { them() } else { me() }
        );
        assert_eq!(state.object(first).unwrap().owner, them());
        assert_eq!(state.object(second).unwrap().owner, them());
    }
}

#[test]
fn greatest_exchange_ties_are_chosen_in_active_player_order() {
    let (mut state, first) = taken();
    let base = state.object(first).unwrap().base.clone();
    let mut mine = vec![first];
    let mut theirs = Vec::new();
    for owner in [me(), them(), them()] {
        let name = state.names.intern("tied exchange candidate");
        let id = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(id).unwrap().base = base.clone();
        if owner == me() {
            mine.push(id);
        } else {
            theirs.push(id);
        }
    }
    state.invalidate_projections();
    state.refresh_characteristics();
    state.turn.active = them();
    let mut res = Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: me(),
        effects: vec![Effect::ExchangeGreatestManaValue {
            filter: &Filter::CREATURE,
            with: PlayerRel::Chosen,
        }],
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: Some(them()),
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        this_way_from: None,
        coin_stake: None,
    };

    let Flow::Wait(Pending::ChooseCards {
        player, options, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("active player's tied maximum must be chosen first");
    };
    assert_eq!(player, them());
    assert_eq!(options, theirs);
    let mut alternative_state = state.clone();
    let mut alternative = res.clone();
    assert!(matches!(
        resume(&mut alternative_state, &mut alternative, &[theirs[1]]),
        Flow::Wait(_)
    ));
    let Flow::Wait(Pending::ChooseCards {
        player, options, ..
    }) = resume(&mut state, &mut res, &[theirs[0]])
    else {
        panic!("nonactive caster must choose second");
    };
    assert_ne!(
        res.awaiting.as_ref().unwrap().search_fingerprint(),
        alternative.awaiting.as_ref().unwrap().search_fingerprint(),
        "the same second question retains a different first choice"
    );
    assert_eq!(player, me());
    assert_eq!(options, mine);
    assert!(matches!(
        resume(&mut state, &mut res, &[mine[0]]),
        Flow::Complete
    ));
    assert_eq!(state.object(mine[0]).unwrap().controller, them());
    assert_eq!(state.object(theirs[0]).unwrap().controller, me());
    assert_eq!(state.object(mine[1]).unwrap().controller, me());
    assert_eq!(state.object(theirs[1]).unwrap().controller, them());
}
