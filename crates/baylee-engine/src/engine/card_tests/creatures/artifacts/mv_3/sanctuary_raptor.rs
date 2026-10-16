//! `cards/creatures/artifacts/mv_3/sanctuary_raptor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sanctuary_raptor() -> CardIndex {
    card_index("6af53351-ad10-4774-9b5c-7b58bea65bbc")
}

/// Three Blood Servitors, three Blood tokens.
fn three_tokens_and_a_raptor() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let servitor = card_index("5a3f9c4d-20a8-46aa-9cf3-3916f56a7906");
    let mut board = vec![sanctuary_raptor()];
    board.extend_from_slice(&[forest(); 9]);
    let mut engine = main_with(&[servitor; 3], &board);
    tap_all_mana(&mut engine, p0);
    for _ in 0..3 {
        cast_with_floating(&mut engine, p0, servitor);
        pass_until(&mut engine, stack_is_empty);
    }
    let raptor = on_battlefield(&engine, p0, sanctuary_raptor()).expect("seated");
    (engine, raptor)
}

/// Sanctuary Raptor — "Whenever this creature attacks, if you control three
/// or more tokens, this creature gets +2/+0 and gains first strike until end
/// of turn." With three tokens it is a 4/1 first striker at the block step.
#[test]
fn sanctuary_raptor_gets_plus_two_and_first_strike_with_three_tokens() {
    let p0 = PlayerId::new(0);
    let (mut engine, raptor) = three_tokens_and_a_raptor();
    assert_eq!(tokens_of(&engine, p0).len(), 3);
    assert_eq!(pt(&engine, raptor), (2, 1));
    assert!(keywords(&engine, raptor).contains(KeywordSet::FLYING));
    assert!(!keywords(&engine, raptor).contains(KeywordSet::FIRST_STRIKE));

    attack_to_blocks(&mut engine, &[raptor]);

    assert_eq!(pt(&engine, raptor), (4, 1));
    assert!(keywords(&engine, raptor).contains(KeywordSet::FIRST_STRIKE));
}

/// With only two tokens the condition fails and the Raptor is unchanged.
#[test]
fn sanctuary_raptor_gets_nothing_with_two_tokens() {
    let p0 = PlayerId::new(0);
    let (mut engine, raptor) = three_tokens_and_a_raptor();
    let third = tokens_of(&engine, p0)[2];
    kill(&mut engine, third);
    assert_eq!(tokens_of(&engine, p0).len(), 2);

    attack_to_blocks(&mut engine, &[raptor]);

    assert_eq!(pt(&engine, raptor), (2, 1));
    assert!(!keywords(&engine, raptor).contains(KeywordSet::FIRST_STRIKE));
}
