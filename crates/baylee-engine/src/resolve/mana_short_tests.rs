use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use baylee_cards_dsl::{Filter, PlayerRel};
use baylee_core::ids::SeatSet;
use baylee_core::mana::ManaColor;

static SHORT: &[Effect] = &[
    Effect::TapAllOf {
        who: PlayerRel::Chosen,
        filter: &Filter::Any,
    },
    Effect::LoseUnspentMana {
        who: PlayerRel::Chosen,
    },
];

#[test]
fn the_chosen_player_is_tapped_out_and_loses_their_mana_and_nobody_else() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut state = GameState::from_preset(&preset(17, &[]), &SyntheticLookup::new(vec![]))
        .expect("a two-seat game");
    let bare = |state: &mut GameState, owner, label: &str| {
        let name = state.names.intern(label);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    };
    let mine = bare(&mut state, me, "Mine");
    let theirs = bare(&mut state, them, "Theirs");
    let spell = bare(&mut state, me, "Mana Short");
    state.players[0].mana_pool.add(ManaColor::Blue, 1);
    state.players[1].mana_pool.add(ManaColor::Green, 2);
    let mut res = Resolution {
        source: spell,
        on_stack: spell,
        controller: me,
        effects: SHORT.to_vec(),
        pc: 0,
        targets: SmallVec::new(),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: Some(them),
        target_players: SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
    };
    assert!(matches!(run(&mut state, &mut res), Flow::Complete));
    let tapped = |id| {
        state
            .object(id)
            .is_some_and(|o| o.status.contains(crate::object::Status::TAPPED))
    };
    assert!(tapped(theirs), "the chosen player's permanent is tapped");
    assert!(!tapped(mine), "and the caster's is not");
    assert!(state.players[1].mana_pool.is_empty(), "their mana is lost");
    assert_eq!(
        state.players[0].mana_pool.available(ManaColor::Blue),
        1,
        "and the caster's stays"
    );
}
