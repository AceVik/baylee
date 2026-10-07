//! `cards/creatures/mv_3/alert_shu_infantry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Alert Shu Infantry is a `{2}{W}` 2/2 Human Soldier whose whole printed
/// text is vigilance, and vigilance is only readable in the combat step:
/// CR 508.1f taps every attacking creature *except* one that has it.
///
/// So the board carries a Llanowar Elves beside it as the control — a
/// creature with no vigilance under the same seat, declared in the same
/// attack — and the two readings are one: the Elf comes back tapped and the
/// Infantry stands. That the 3 damage actually reaches the defending player
/// is what says the declaration was real combat and not merely an offer the
/// engine had listed.
#[test]
fn alert_shu_infantry_attacks_without_tapping_where_the_elf_beside_it_taps() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), llanowar_elves()])
        .hand(0, &[alert_shu_infantry()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The three Plains pay the {2}{W}, and the Elf is named as the printing
    // kept back: it is the control this test turns on, and a creature tapped
    // for mana may not attack (rule 11 — a mana creature counts in the pool
    // and would have left the board, not filled it).
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, alert_shu_infantry());
    pass_until(&mut engine, stack_is_empty);
    let soldier = on_battlefield(&engine, p0, alert_shu_infantry()).expect("the Infantry resolved");
    assert_eq!(pt(&engine, soldier), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, soldier).contains(KeywordSet::VIGILANCE),
        "the one printed word, projected onto the permanent"
    );

    // A creature cast this turn has summoning sickness (CR 302.6), so the
    // attack belongs to its controller's next turn — the turn cycle is what
    // makes the declaration below legal at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert!(
        !is_tapped(&engine, soldier) && !is_tapped(&engine, elves),
        "both stood up in the untap step, so neither is tapped for another reason"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&soldier) && attackers.contains(&elves),
        "two untapped, unsick creatures of mine: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (soldier, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, soldier),
        "\"Vigilance\": the Infantry attacked and is still standing"
    );
    assert!(
        is_tapped(&engine, elves),
        "the Elf beside it has no vigilance and tapped to attack — the \
         counter-half that makes the sentence above a filter and not a board"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        17,
        "2 from the Infantry and 1 from the Elf reached the player they \
         attacked, so the declaration was no mere offer"
    );
}
