use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::Filter;
use baylee_core::ids::SeatSet;

static UNTAP: &[Effect] = &[Effect::UntapAll {
    filter: &Filter::AttachedToBySource,
}];
static REGENERATE: &[Effect] = &[Effect::RegenerateAll {
    filter: &Filter::AttachedToBySource,
}];

/// An Aura on one tapped creature, another tapped creature beside it,
/// and `effects` resolving from the Aura.
fn resolved(effects: &'static [Effect]) -> (GameState, ObjectId, ObjectId) {
    let me = PlayerId::new(0);
    let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let bare = |state: &mut GameState, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(me, ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    };
    let host = bare(&mut state, "Host");
    let other = bare(&mut state, "Other");
    let aura = bare(&mut state, "Aura");
    state.object_mut(aura).expect("here").attached_to = Some(host);
    state.set_tapped(host, true);
    state.set_tapped(other, true);
    let mut res = Resolution {
        source: aura,
        on_stack: aura,
        controller: me,
        effects: effects.to_vec(),
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        this_way_from: None,
        coin_stake: None,
    };
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    (state, host, other)
}

fn tapped(state: &GameState, id: ObjectId) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
}

#[test]
fn an_auras_ability_reaches_its_host_and_nothing_else() {
    let (state, host, other) = resolved(UNTAP);
    assert!(!tapped(&state, host), "the enchanted creature untaps");
    assert!(tapped(&state, other), "and only it");

    let (state, host, other) = resolved(REGENERATE);
    let shields = |id| state.object(id).map(|o| o.regeneration_shields);
    assert_eq!(shields(host), Some(1));
    assert_eq!(shields(other), Some(0));
}

#[test]
fn convenient_target_suspects_only_its_departed_auras_original_host() {
    use crate::choice::PlayerAction;
    use crate::engine::testkit::{
        Duel, card_index, cast_from_hand, keep_mulligans, on_battlefield, pass_until,
        reach_main_phase, stack_is_empty,
    };
    use crate::object::Rider;
    use crate::zone::ZonePosition;

    let player = PlayerId::new(0);
    let convenient_target = card_index("2f214dcf-9601-4ccb-bdac-2f9b9f6dff99");
    let mountain = card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685");
    let elves = card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3");
    let cleric = card_index("f4232466-dd6a-49bf-be6c-95905c3ded17");
    for blink_host in [false, true] {
        let mut engine = Duel::new(31, mountain)
            .hand(0, &[convenient_target])
            .battlefield(0, &[mountain, elves, cleric])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, player);
        let host = on_battlefield(&engine, player, elves).unwrap();
        let other = on_battlefield(&engine, player, cleric).unwrap();
        cast_from_hand(&mut engine, player, convenient_target);
        engine
            .apply(
                player,
                PlayerAction::ChooseTargets {
                    objects: vec![host],
                    players: vec![],
                },
            )
            .unwrap();
        pass_until(&mut engine, |engine| {
            on_battlefield(engine, player, convenient_target).is_some()
        });
        assert!(!stack_is_empty(&engine), "the Aura's trigger is waiting");

        let aura = on_battlefield(&engine, player, convenient_target).unwrap();
        let state = engine.dev_state_mut(player).unwrap();
        state
            .move_object(
                aura,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        if blink_host {
            for destination in [ZoneLocation::Exile(player), ZoneLocation::Battlefield] {
                state
                    .move_object(host, destination, ZonePosition::Top, Cause::Effect)
                    .unwrap();
            }
        }

        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine
                .state()
                .object(host)
                .unwrap()
                .riders
                .contains(&Rider::Suspected),
            !blink_host
        );
        assert!(
            !engine
                .state()
                .object(other)
                .unwrap()
                .riders
                .contains(&Rider::Suspected)
        );
    }
}
