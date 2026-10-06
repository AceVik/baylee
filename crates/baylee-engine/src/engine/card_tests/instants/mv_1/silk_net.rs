//! `cards/instants/mv_1/silk_net.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Silk Net is one sentence — "Target creature gets +1/+1 and gains reach
/// until end of turn" — and both halves of it land on the creature the offer
/// named and on no other. That is why the board holds two Elves under the
/// caster and one across the table: the target reads `(2, 2)` with `REACH`
/// out of the layer projection, which is the only reading that can see a
/// *granted* keyword, while the two Elves nobody aimed at stay printed
/// `(1, 1)`s without it. The offer is the other half of "target creature" —
/// the Elf across the table is on it, the Forest beside it is not, and the
/// `{G}` is read as spent at the end.
#[test]
fn silk_net_pumps_and_grants_reach_to_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[silk_net()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which the Net never names"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Net");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "reach is granted and not printed, so it is not there yet"
    );

    // The Forest pays and the Elves are kept standing: their own `{T}: Add
    // {G}` is a mana route `tap_all_mana` would take, and a host tapped for
    // mana reads like a host the Net never reached.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest tapped and neither Elf: the pool is the {{G}} and nothing else"
    );
    cast_with_floating(&mut engine, p0, silk_net());

    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that cast the Net is the one that aims it"
    );
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a Forest is a permanent and no creature: {options:?}"
    );
    assert_eq!(options.len(), 3, "and those three are the whole menu");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered is the one it is aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "+1/+1 on the creature the Net targets"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "and the printed reach reaches it through the layers"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody aimed at is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "\"target creature\" is not \"creatures you control\""
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the Elf across the table was a legal target and was not the one \
         named, so the pump never crossed on its own"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "nor did the keyword"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{G}} came out of the pool the Forest filled"
    );
}
