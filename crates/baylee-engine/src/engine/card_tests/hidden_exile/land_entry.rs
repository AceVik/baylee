// New child of hidden_exile; inherits its exact helpers.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn hidden_exile_mdfc_life_payment_suspends_and_resumes_remaining_linked_cards() {
    for pay in [false, true] {
        let (mut e, source, land) = hidden_with_board(
            index::SHELLDOCK_ISLE,
            index::SHATTERSKULL_SMASHING,
            &[index::GRIZZLY_BEARS],
        );
        // Transparent second linked-card fixture, matching the existing multi-link
        // regression: the source ability and both subsequent plays are real actions.
        let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
        e.state
            .move_object(
                bear,
                ZoneLocation::Exile(P0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let reference = e.state.source_identity(source).unwrap();
        e.state
            .hide_exiled_card(bear, reference, 0, [P0].into_iter().collect(), true);
        satisfy(&mut e, index::SHELLDOCK_ISLE);
        e.state.players[0].lands_played_this_turn = 0;
        activate(&mut e, P0, index::SHELLDOCK_ISLE, 2);
        pass_until(&mut e, playing);
        choose(&mut e, vec![land]);
        let Pending::ChooseCastMode { options, .. } = e.pending() else {
            panic!("MDFC face menu: {:?}", e.pending())
        };
        let mode = options
            .iter()
            .find(|o| o.kind == CastModeKind::PlayLandFace(1))
            .unwrap()
            .index;
        e.apply(P0, PlayerAction::ChooseMode(mode as usize))
            .unwrap();
        assert!(
            matches!(e.pending(),Pending::YesNo{player,prompt:crate::choice::YesNoPrompt::PayLifeOrEnterTapped{amount:3},..} if *player==P0)
        );
        assert_eq!(
            e.state.object(land).unwrap().zone,
            Zone::Exile,
            "entry choice precedes the land's battlefield move"
        );
        let life = e.state.players[0].life;
        e.apply(P0, PlayerAction::YesNo(pay)).unwrap();
        assert_eq!(e.state.object(land).unwrap().zone, Zone::Battlefield);
        assert_eq!(e.state.object(land).unwrap().face_index, 1);
        assert_eq!(is_tapped(&e, land), !pay);
        assert_eq!(e.state.players[0].life, life - if pay { 3 } else { 0 });
        assert_eq!(e.state.players[0].lands_played_this_turn, 1);
        assert!(
            playing(&e),
            "the linked parent resumes exactly once, after land entry, and offers the other card"
        );
        choose(&mut e, vec![bear]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state.object(bear).unwrap().zone, Zone::Battlefield);
        assert!(
            e.effect_plays.is_empty(),
            "both nested continuations have completed"
        );
    }
}
