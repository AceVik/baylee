//! `cards/creatures/mv_2/broodhatch_nantuko.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn broodhatch_nantuko() -> CardIndex {
    card_index("b0a54050-3493-4ff6-8eca-e43b92c1f3d3")
}

/// Broodhatch Nantuko ({1}{G} 1/1 Insect Druid):
/// "Whenever this creature is dealt damage, you may create that many 1/1 green
/// Insect creature tokens. Morph {2}{G}"
#[test]
fn broodhatch_nantuko_creates_insect_tokens_equal_to_damage_dealt_when_accepted() {
    let mut e = lk::game(SEED, &[broodhatch_nantuko()], &[], &[lightning_bolt()], &[]);
    let nantuko = lk::obj(&e, lk::P0, broodhatch_nantuko());

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[nantuko], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, true);
    lk::settle(&mut e);

    let tokens = tokens_of(&e, lk::P0);
    assert_eq!(tokens.len(), 3, "3 damage created 3 Insect tokens");
    let printed = e.state().object(tokens[0]).unwrap().token.unwrap();
    assert_eq!(printed.name, "Insect");
    assert_eq!((printed.power, printed.toughness), (Some(1), Some(1)));
    assert!(printed.colors.contains(baylee_core::color::Color::Green));
}

#[test]
fn broodhatch_nantuko_morph_turns_face_up_and_may_decline_tokens() {
    let mut e = lk::game(
        SEED,
        &[forest(); 6],
        &[],
        &[broodhatch_nantuko(), lightning_bolt()],
        &[],
    );
    cast_from_hand(&mut e, lk::P0, broodhatch_nantuko());
    let slot = choose_cast_kind(&e, CastModeKind::Disguise);
    e.apply(lk::P0, PlayerAction::ChooseMode(slot)).unwrap();
    lk::settle(&mut e);

    let id = lk::obj(&e, lk::P0, broodhatch_nantuko());
    assert!(
        e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (2, 2));

    e.apply(
        lk::P0,
        PlayerAction::ActivateAbility {
            source: id,
            ability_index: baylee_engine::choice::TURN_FACE_UP,
        },
    )
    .expect("remaining {2}{G} pays Morph {2}{G}");
    assert!(
        !e.state()
            .object(id)
            .unwrap()
            .status
            .contains(Status::FACE_DOWN)
    );
    assert_eq!(pt(&e, id), (1, 1));

    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, lightning_bolt());
    lk::aim(&mut e, &[id], &[]);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    lk::yes_no(&mut e, false);
    lk::settle(&mut e);
    assert!(tokens_of(&e, lk::P0).is_empty());
}
