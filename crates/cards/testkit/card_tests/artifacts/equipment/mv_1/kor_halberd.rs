//! `cards/artifacts/equipment/mv_1/kor_halberd.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kor_halberd() -> CardIndex {
    card_index("450e84c7-afbe-4b30-a3e9-1dff968a5200")
}

/// Kor Halberd — "Equipped creature gets +1/+1 and has vigilance." and
/// "Equip {1}". The equipped Elf attacks, deals 2, and is still untapped
/// afterwards; the bare Elf beside it would have tapped.
#[test]
fn kor_halberd_gives_vigilance_and_a_bonus() {
    plain_pump(kor_halberd(), 1, 1, 1);

    let elf = llanowar_elves();
    let mut engine = rig(1, &[], &[kor_halberd(), elf, elf], &[]);
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let elves = all_on_battlefield(&engine, p0, elf);
    let (host, other) = (elves[0], elves[1]);
    equip(&mut engine, kor_halberd(), 1, host);
    assert!(keywords(&engine, host).contains(KeywordSet::VIGILANCE));
    assert!(!keywords(&engine, other).contains(KeywordSet::VIGILANCE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(host, Defender::Player(p1)), (other, Defender::Player(p1))],
            },
        )
        .expect("both Elves attack");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 20 - 2 - 1);
    assert!(
        !is_tapped(&engine, host),
        "vigilance: attacking did not tap it"
    );
    assert!(is_tapped(&engine, other), "the bare Elf tapped");
}
