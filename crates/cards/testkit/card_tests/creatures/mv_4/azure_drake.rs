//! `cards/creatures/mv_4/azure_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Azure Drake is `{3}{U}` for a 2/4 Drake with flying and no other printed
/// word, so the card is exactly two claims: the body and the keyword. The
/// keyword is the half worth a bystander — a Llanowar Elves stands beside it
/// under the same seat as a printed 1/1 with no keyword of its own, so flying
/// that had leaked off the Drake (or off the layer projection generally) shows
/// up on a creature that never printed it. The four Islands are read as a pool
/// before the cast (`castable` is filtered through `can_afford`, which reads
/// the pool and not the untapped lands) and are named as the only sources
/// tapped, so the {3}{U} is a real payment rather than a label.
#[test]
fn azure_drake_resolves_as_a_flying_two_four_and_leaves_the_bystander_grounded() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .hand(0, &[azure_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "the bystander is a printed 1/1 with no keyword of its own"
    );

    // The Elf is named as the source kept back: its whole price is its own
    // {T}, so `tap_all_mana` would drink it and there would be nothing left to
    // read the keyword against. Four Islands are the whole cost of the Drake.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands, four blue, and no Elf in the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, azure_drake()).expect("the Drake is in hand");
    assert!(
        legal.castable.contains(&card),
        "{{3}}{{U}} is payable out of the pool that was just filled: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, azure_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, azure_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (2, 4), "the body the card prints");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "Azure Drake — flying"
    );
    assert!(
        !keywords(&engine, elf).contains(KeywordSet::FLYING),
        "and the Elf beside it is still a printed 1/1 on the ground"
    );
}
