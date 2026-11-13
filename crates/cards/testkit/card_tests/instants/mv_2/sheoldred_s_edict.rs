//! `cards/instants/mv_2/sheoldred_s_edict.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cast_edict(engine: &mut Engine<RegistryLookup>, mode: usize) {
    cast_from_hand(engine, P0, index::SHEOLDRED_S_EDICT);
    engine.apply(P0, PlayerAction::ChooseMode(mode)).unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { player: P1, .. })
    });
}

#[test]
fn sheoldreds_edict_sacrifices_a_token_without_touching_nontokens() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[index::SHEOLDRED_S_EDICT])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let token = baylee_engine::resolve::tokens::create_tokens(
        engine.dev_state_mut(P0).unwrap(),
        P1,
        &baylee_cards::tokens::SOLDIER_1_1_WHITE,
        None,
        1,
    )[0];
    engine.refresh_offer();

    assert_eq!(tokens_of(&engine, P1), vec![token]);
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_edict(&mut engine, 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(
        options,
        vec![token],
        "only a creature token is a legal choice"
    );
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![token],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(tokens_of(&engine, P1).is_empty());
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn sheoldreds_edict_lets_the_opponent_choose_which_planeswalker_to_sacrifice() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[index::SHEOLDRED_S_EDICT])
        .battlefield(
            1,
            &[
                index::JACE_THE_MIND_SCULPTOR,
                index::SERRA_THE_BENEVOLENT,
                grizzly_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let jace = on_battlefield(&engine, P1, index::JACE_THE_MIND_SCULPTOR).unwrap();
    let serra = on_battlefield(&engine, P1, index::SERRA_THE_BENEVOLENT).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    cast_edict(&mut engine, 2);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(options, vec![jace, serra]);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![serra],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(serra).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(jace).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        engine.state().object(bears).unwrap().zone,
        Zone::Battlefield
    );
}
