//! Free copies still obey additional costs and explicit timing restrictions.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::choice::NumberPrompt;

#[test]
fn batch45_mastery_copy_pays_a_chosen_nonmana_x_for_toxic_deluge() {
    let mut e = prepared(
        &[index::MIZZIX_S_MASTERY, index::TOXIC_DELUGE],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let original = hand_to_graveyard(&mut e, P0, index::TOXIC_DELUGE);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast_with_floating(&mut e, P0, index::MIZZIX_S_MASTERY);
    mode(&mut e, CastModeKind::Mode(0));
    target(&mut e, vec![original], vec![]);
    pass_until(&mut e, exile_choice);
    let copy = cards_offered(&e)[0];
    let life = e.state.players[0].life;
    choose(&mut e, vec![copy]);
    assert!(
        matches!(e.pending(), Pending::ChooseNumber { reason: NumberPrompt::X, max, .. } if *max >= 3)
    );
    e.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    assert_eq!(e.state.players[0].life, life - 3);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state.object(original).unwrap().zone, Zone::Exile);
    assert!(e.state.object(copy).is_none());
}

#[test]
fn batch45_mastery_cannot_cast_a_copy_through_teferis_explicit_restriction() {
    let mut e = prepared(
        &[index::MIZZIX_S_MASTERY, index::COUNSEL_OF_THE_SORATAMI],
        &[],
        &[index::TEFERI_TIME_RAVELER],
    );
    let original = hand_to_graveyard(&mut e, P0, index::COUNSEL_OF_THE_SORATAMI);
    let mastery = in_hand(&e, P0, index::MIZZIX_S_MASTERY).unwrap();
    cast(&mut e, mastery);
    mode(&mut e, CastModeKind::Mode(0));
    target(&mut e, vec![original], vec![]);
    pass_until(&mut e, exile_choice);
    let copy = cards_offered(&e)[0];
    let draws = e.state.per_turn.draws[0];
    choose(&mut e, vec![copy]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.per_turn.spells_cast[0], 1);
    assert_eq!(e.state.per_turn.draws[0], draws);
    assert_eq!(e.state.object(original).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(mastery).unwrap().zone, Zone::Exile);
    assert!(e.state.object(copy).is_none());
}

#[test]
fn batch45_north_star_replication_pays_the_base_flexibly_and_each_extra_normally() {
    let mut e = prepared(
        &[index::OPEN_COMMUNICATIONS, index::LOSE_FOCUS],
        &[index::NORTH_STAR],
        &[],
    );
    activate(&mut e, P0, index::NORTH_STAR, 0);
    pass_until(&mut e, stack_is_empty);
    let original = in_hand(&e, P0, index::OPEN_COMMUNICATIONS).unwrap();
    cast(&mut e, original);
    yes(&mut e, false);
    assert_eq!(e.state.object(original).unwrap().zone, Zone::Stack);
    pool(&mut e, ManaColor::Red, 2);
    e.state.players[0].mana_pool.add(ManaColor::Blue, 1);
    let focus = in_hand(&e, P0, index::LOSE_FOCUS).unwrap();
    cast(&mut e, focus);
    yes(&mut e, true);
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            reason: NumberPrompt::Replicate { .. },
            max: 1,
            ..
        }
    ));
    e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
    target(&mut e, vec![original], vec![]);
    assert_eq!(e.state.object(focus).unwrap().zone, Zone::Stack);
    assert_eq!(e.state.object(focus).unwrap().replicated, 1);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(crate::casting::north_star_effect(&e.state, P0).is_none());
}
