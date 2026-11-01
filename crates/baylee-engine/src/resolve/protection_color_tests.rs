//! Self-protection applies to the source incarnation captured by its ability.
use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::{Duration, Filter, Modifier};
use baylee_core::ids::SeatSet;

fn captured_ability() -> (GameState, Resolution) {
    let player = PlayerId::new(0);
    let mut state =
        GameState::from_preset(&preset(419, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let name = state.names.intern("self-protection source");
    let source = state.create_bare(
        player,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let version = state.object(source).unwrap().version;
    let ability = state.create_bare(
        player,
        ObjectKind::AbilityOnStack,
        name,
        ZoneLocation::Stack,
    );
    state
        .object_mut(ability)
        .unwrap()
        .riders
        .push(crate::object::Rider::AbilitySourceVersion(version));
    let res = Resolution {
        source,
        on_stack: ability,
        controller: player,
        effects: vec![Effect::ProtectionFromChosenColor {
            duration: Duration::UntilEndOfTurn,
        }],
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
    (state, res)
}

#[test]
fn unchanged_source_chooses_a_color_and_gets_protection_on_its_captured_incarnation() {
    let (mut state, mut res) = captured_ability();
    let version = state.object(res.source).unwrap().version;
    let Flow::Wait(Pending::ChooseColor { player, options }) = run(&mut state, &mut res) else {
        panic!("an unchanged source chooses its protection color");
    };
    assert_eq!(player, res.controller);
    assert!(options.contains(&ManaColor::Red));
    assert!(matches!(
        resume_with_color(&mut state, &mut res, ManaColor::Red),
        Flow::Complete
    ));
    assert!(state.effects.iter().any(|effect| {
        matches!(effect.filter, crate::effects::EffectFilter::ObjectIs(id, captured) if id == res.source && captured == version)
            && effect.duration == Duration::UntilEndOfTurn
            && matches!(effect.modifier, Modifier::ProtectionFrom(filter) if *filter == Filter::HasColor(ColorSet::of(baylee_core::color::Color::Red)))
    }));
}

#[test]
fn source_blinked_in_response_does_not_offer_or_grant_protection_to_its_new_incarnation() {
    let (mut state, mut res) = captured_ability();
    let captured = state.object(res.source).unwrap().version;
    for zone in [
        ZoneLocation::Exile(res.controller),
        ZoneLocation::Battlefield,
    ] {
        state
            .move_object(res.source, zone, ZonePosition::Top, Cause::Effect)
            .unwrap();
    }
    assert_ne!(state.object(res.source).unwrap().version, captured);
    assert_eq!(
        state.object(res.source).unwrap().zone,
        crate::zone::Zone::Battlefield
    );
    assert!(
        matches!(run(&mut state, &mut res), Flow::Complete),
        "the departed source has no color choice for the returned object"
    );
    assert!(
        !state
            .effects
            .iter()
            .any(|effect| matches!(effect.modifier, Modifier::ProtectionFrom(_)))
    );
}
