//! `cards/enchantments/auras/mv_2/fishliver_oil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fishliver Oil prints two sentences: "Enchant creature (Target a creature
/// as you cast this. This card enters attached to that creature.)" and
/// "Enchanted creature has islandwalk."
///
/// The grant is a `Filter::AttachedToBySource` static, so the board is built
/// to tell the host from every other creature that might get it by mistake:
/// a second Elf under the same seat and one across the table. The keyword is
/// then read through its rules meaning (CR 702.14c) rather than as a label —
/// the defending player controls an Island, so the host cannot be blocked
/// and the declare-blockers offer holds no pairing against it. That last
/// half is what a card that merely listed "islandwalk" without the combat
/// rule would lose.
#[test]
fn fishliver_oil_grants_islandwalk_to_its_host_and_only_its_host() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves(), island()])
        .hand(0, &[fishliver_oil()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "the host and the Elf that must stay bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::ISLANDWALK),
        "nothing is enchanted yet"
    );

    // {1}{U} off the two Islands, with both Elves kept back: the host has to
    // be untapped to attack later.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, fishliver_oil());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf is a legal host");
    pass_until(&mut engine, stack_is_empty);

    let oil = on_battlefield(&engine, p0, fishliver_oil()).expect("the Aura resolved");
    assert_eq!(
        engine
            .state()
            .object(oil)
            .expect("the Aura is an object")
            .attached_to,
        Some(host),
        "the Aura enters attached to the creature it was cast on"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::ISLANDWALK),
        "enchanted creature has islandwalk"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::ISLANDWALK),
        "the static reaches the creature it is attached to and no other: \
         {bystander:?} is a creature under the same seat and stayed bare"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::ISLANDWALK),
        "nor across the table"
    );

    // CR 702.14c: the defending player controls an Island, so the host cannot
    // be blocked and the offer the engine publishes names no blocker for it.
    let blocks = attack_and_collect_blocks(&mut engine, host, p1);
    assert!(
        blocks
            .iter()
            .all(|option| !option.attackers.contains(&host)),
        "the defending player controls an Island, so islandwalk leaves no \
         legal block against the host: {blocks:?}"
    );
}
