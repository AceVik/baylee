#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{ALERT_SHU_INFANTRY, METALLIC_SLIVER};

fn subject() -> CardIndex {
    card_index("e0f05979-daba-43eb-b356-24284e7ca99c")
}

// Scenario: source and another own Sliver have haste on entry; a non-Sliver
// and an opposing Sliver do not. The fresh source may attack this turn.
#[test]
fn blur_sliver_gives_only_our_slivers_immediate_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[METALLIC_SLIVER])
        .hand(0, &[subject(), METALLIC_SLIVER, ALERT_SHU_INFANTRY])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&e, p1, METALLIC_SLIVER).unwrap();
    tap_all_mana_but(&mut e, p0, Some(plains()));
    cast_with_floating(&mut e, p0, subject());
    pass_until(&mut e, stack_is_empty);
    cast_with_floating(&mut e, p0, METALLIC_SLIVER);
    pass_until(&mut e, stack_is_empty);
    tap_all_mana(&mut e, p0);
    cast_with_floating(&mut e, p0, ALERT_SHU_INFANTRY);
    pass_until(&mut e, stack_is_empty);
    let ours = on_battlefield(&e, p0, subject()).unwrap();
    let ally = on_battlefield(&e, p0, METALLIC_SLIVER).unwrap();
    let fresh_non_sliver = on_battlefield(&e, p0, ALERT_SHU_INFANTRY).unwrap();
    assert!(keywords(&e, ours).contains(KeywordSet::HASTE));
    assert!(keywords(&e, ally).contains(KeywordSet::HASTE));
    assert!(!keywords(&e, elf).contains(KeywordSet::HASTE));
    assert!(!keywords(&e, fresh_non_sliver).contains(KeywordSet::HASTE));
    assert!(!keywords(&e, theirs).contains(KeywordSet::HASTE));
    pass_until(&mut e, |game| {
        matches!(game.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(attackers.contains(&ours));
    assert!(attackers.contains(&ally));
    assert!(!attackers.contains(&fresh_non_sliver));
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(ours, Defender::Player(p1)), (ally, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert!(is_tapped(&e, ours));
    assert!(is_tapped(&e, ally));
}
