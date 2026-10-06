//! `cards/enchantments/auras/mv_1/lance.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lance is a {W} Aura: "Enchant creature" and "Enchanted creature has first
/// strike." The keyword is a static over `AttachedToBySource`, so the only
/// reading worth playing is the one that separates the creature the Aura
/// holds from every other creature on the table — the Elf across the table is
/// the control, and the Aura itself must not keep what it grants. And first
/// strike is a word the combat step reads: two 1/1s trade in one simultaneous
/// damage step, so an enchanted 1/1 blocker-killer walking away alone is the
/// card doing the thing its text says.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn lance_gives_first_strike_to_the_creature_it_enchants_and_wins_that_combat() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[lance()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "nothing is enchanted yet"
    );

    // {W} off the Plains, with both creatures kept standing: the host has to
    // be untapped to attack with below, and a creature that tapped for mana
    // may not (the Elves print `{T}: Add {G}`).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one white mana, and the only source touched is the Plains"
    );
    cast_with_floating(&mut engine, p0, lance());

    // "Enchant creature" is the target question the card prints, and it is
    // asked of the whole table: the Elf across it is a legal answer to look
    // at and the wrong one to give.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let aura = on_battlefield(&engine, p0, lance()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FIRST_STRIKE),
        "enchanted creature has first strike"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FIRST_STRIKE),
        "the static reaches the enchanted creature and never across the table"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::FIRST_STRIKE),
        "the Aura grants the keyword, it does not keep it"
    );

    let their_life = engine.state().players[1].life;

    // Played out into the step that reads the keyword: the enchanted 1/1
    // attacks, the other 1/1 blocks, and only the block dies.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(host, Defender::Player(p1))],
            },
        )
        .unwrap();

    let mut blocked = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { blockers, .. } => {
                let pair = blockers.iter().find(|o| o.blocker == theirs);
                assert!(
                    pair.is_some_and(|o| o.attackers.contains(&host)),
                    "the untapped Elf across the table may block the attacker: {blockers:?}"
                );
                engine
                    .apply(
                        p1,
                        PlayerAction::DeclareBlockers {
                            blockers: vec![(theirs, host)],
                        },
                    )
                    .unwrap();
                blocked = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while declaring blockers: {other:?}"),
        }
    }
    assert!(blocked, "the block was declared");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the blocker died to the first-strike damage step"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the enchanted attacker is still standing — without first strike \
         the two 1/1s would have traded in one simultaneous damage step"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life,
        "a blocked attacker deals its damage to the blocker, not to the player"
    );
}
