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
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
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
