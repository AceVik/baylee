use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

/// Two seats, seat 1 gone, and a spell of seat 0's on the stack.
fn state() -> (GameState, ObjectId) {
    let mut state = GameState::from_preset(&preset(278, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let name = state.names.intern("Spell");
    let spell = state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
    crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);
    (state, spell)
}

/// A resolution of seat 1's, which has left, with `effect` next.
fn theirs(effect: Effect, target: ObjectId) -> Resolution {
    Resolution {
        source: ObjectId::NO_SOURCE,
        on_stack: ObjectId::NO_SOURCE,
        controller: them(),
        effects: vec![effect],
        pc: 0,
        targets: SmallVec::from_slice(&[target]),
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
    }
}

/// An emblem is owned by the player who gets it (CR 114.2).
#[test]
fn a_player_who_has_left_gets_no_emblem() {
    let (mut state, spell) = state();
    let mut res = theirs(Effect::CreateEmblem { abilities: &[] }, spell);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert!(state.zones.list(ZoneLocation::Command(them())).is_empty());
}

/// A copy of a spell is owned by the player under whose control it was
/// put on the stack (CR 707.10).
#[test]
fn a_player_who_has_left_gets_no_copy_of_a_spell() {
    let (mut state, spell) = state();
    let mut res = theirs(Effect::CopyTargetSpell { mods: &[] }, spell);
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    assert_eq!(state.zones.list(ZoneLocation::Stack)[..], [spell]);
}

/// A permanent of seat 0's.
fn permanent(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Permanent");
    state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
}

/// A creature card in seat 0's graveyard.
fn buried(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Creature");
    let id = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Graveyard(me()));
    let obj = state.object_mut(id).expect("fresh");
    let mut base = (*obj.base).clone();
    base.types = baylee_core::types::TypeSet::CREATURE;
    obj.base = std::sync::Arc::new(base);
    state.invalidate_projections();
    id
}

fn control_effects(state: &GameState) -> usize {
    state
        .effects
        .iter()
        .filter(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl)
        .count()
}

/// Nothing changes to the control of a player who has left (CR 800.4b):
/// their "gain control of target creature" resolves to nothing, and no
/// indefinite change is registered for them either.
#[test]
fn a_player_who_has_left_gains_control_of_nothing() {
    let (mut state, _) = state();
    let it = permanent(&mut state);
    let mut res = theirs(
        Effect::continuous(
            &baylee_cards_dsl::Filter::This,
            baylee_cards_dsl::Modifier::GainControl,
            baylee_cards_dsl::Duration::UntilEndOfTurn,
        ),
        it,
    );
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    gain_control(&mut state, &[(it, them())]);
    assert_eq!(control_effects(&state), 0);
    assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
}

/// And a control effect for them that got into the table anyway gives
/// them nothing.
#[test]
fn a_control_effect_for_a_player_who_has_left_applies_to_nothing() {
    let (mut state, _) = state();
    let it = permanent(&mut state);
    let filter = crate::effects::EffectFilter::object(&state, it);
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: them(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Control,
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter,
        modifier: baylee_cards_dsl::Modifier::GainControl,
    });
    state.refresh_characteristics();
    assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
}

/// A card that would be put onto the battlefield under the control of a
/// player who has left stays where it is (CR 800.4b). Under its owner's
/// control it goes, since its owner is still in the game.
#[test]
fn nothing_is_put_onto_the_battlefield_under_a_player_who_has_left() {
    let (mut state, _) = state();
    let card = buried(&mut state);
    let spec =
        TargetSpec::CardInGraveyard(&baylee_cards_dsl::Filter::CREATURE, PlayerRel::EachPlayer);
    for effect in [
        Effect::reanimate(spec),
        Effect::AllGraveyardCreaturesToBattlefield,
    ] {
        let mut res = theirs(effect, card);
        assert!(matches!(run(&mut state, &mut res), Flow::Complete));
        assert_eq!(
            state.zones.list(ZoneLocation::Graveyard(me()))[..],
            [card],
            "{effect:?}"
        );
    }

    let mut res = theirs(
        Effect::GraveyardToBattlefield {
            target: spec,
            owner_control: true,
            counters: None,
        },
        card,
    );
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    let obj = state.object(card).expect("the same card");
    assert_eq!(
        (obj.zone, obj.controller),
        (crate::zone::Zone::Battlefield, me())
    );
}

/// A player who has left controls no player (CR 800.4b): an Opposition
/// Agent's takeover of theirs that is still in the table leaves seat 0
/// to make its own search.
#[test]
fn a_player_who_has_left_takes_over_no_search() {
    let (mut state, _) = state();
    let takeover = baylee_cards_dsl::Modifier::SearchTakeover;
    let timestamp = state.next_timestamp();
    state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: them(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: takeover.layer(),
        timestamp,
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
        modifier: takeover,
    });
    let name = state.names.intern("Card");
    let card = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me()));
    let mut res = theirs(
        Effect::SearchLibrary {
            filter: &baylee_cards_dsl::Filter::Any,
            finds: &[baylee_cards_dsl::effect::Find::HAND],
            optional: false,
        },
        card,
    );
    res.controller = me();
    let Flow::Wait(Pending::ChooseCards {
        player, options, ..
    }) = run(&mut state, &mut res)
    else {
        panic!("a search asks");
    };
    assert_eq!(player, me());
    assert!(options.contains(&card));
}

/// What a player who has left controls and does not own (CR 800.4a): a
/// spell and a permanent are exiled, into their owner's exile, and an
/// ability ceases to exist rather than become a card there.
#[test]
fn what_a_departed_player_controls_is_exiled_or_ceases_to_exist() {
    let (mut state, spell) = state();
    let name = state.names.intern("Ability");
    let ability = state.create_bare(me(), ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
    let it = permanent(&mut state);
    for id in [spell, ability, it] {
        state
            .object_mut(id)
            .expect("made above")
            .set_controller(them());
    }

    let exiled = crate::sba::exile_what_the_departed_control(&mut state);
    assert_eq!(exiled, [it, spell]);
    assert!(state.object(ability).is_none());
    assert!(state.zones.list(ZoneLocation::Stack).is_empty());
    assert_eq!(state.zones.list(ZoneLocation::Exile(me())).len(), 2);
    for id in [it, spell] {
        let obj = state.object(id).expect("in exile");
        assert_eq!(
            (obj.zone, obj.kind),
            (crate::zone::Zone::Exile, ObjectKind::Card)
        );
    }
}
