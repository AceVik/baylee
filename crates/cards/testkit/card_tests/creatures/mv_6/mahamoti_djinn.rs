//! `cards/creatures/mv_6/mahamoti_djinn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "c39ea5f9-6ec0-4697-897b-779e326754a7"

/// Mahamoti Djinn is `{4}{U}{U}` for a 5/6 Djinn and one printed word:
/// flying. Both halves of that are the engine's answer rather than the
/// card's, so both are played: six Islands pay the cost down to an empty
/// pool and the creature lands as the body it prints with the keyword on
/// it, and then — a turn later, past summoning sickness (CR 302.6) — the
/// blocking offer is read as the rule itself, because the keyword is worth
/// nothing until somebody tries to block. The Angel is the half of "except
/// by creatures with flying or reach" that the word grants, and the ground
/// 1/1 beside it is the half the word withholds (CR 702.9b).
#[test]
fn mahamoti_djinn_lands_as_a_five_six_flier_and_only_a_flier_may_block_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), island(), island()],
        )
        .hand(0, &[mahamoti_djinn()])
        // A flier and a ground creature across the table, so the offer has
        // one pairing to make and one to decline.
        .battlefield(1, &[serra_angel(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, mahamoti_djinn());
    pass_until(&mut engine, stack_is_empty);
    let djinn = on_battlefield(&engine, p0, mahamoti_djinn()).expect("the Djinn resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "six Islands paid the whole {{4}}{{U}}{{U}}, so the cast was a real \
         payment and not a card that arrived for free"
    );
    assert_eq!(pt(&engine, djinn), (5, 6), "the body the card prints");
    assert!(
        types(&engine, djinn).contains(TypeSet::CREATURE),
        "and what arrived is that creature"
    );
    assert!(
        keywords(&engine, djinn).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent through the layers"
    );

    // A turn round the table, because the Djinn was cast this turn and a
    // creature that has not begun under its controller's control since that
    // player's most recent turn began may not attack (CR 302.6).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, djinn),
        "the untap step stood the Djinn back up"
    );

    let blocks = attack_and_collect_blocks(&mut engine, djinn, p1);
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("their Angel is out");
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert!(
        blocks
            .iter()
            .any(|o| o.blocker == angel && o.attackers.contains(&djinn)),
        "\"can't be blocked except by creatures with flying or reach\": the \
         Angel has flying, so the pairing is offered: {blocks:?}"
    );
    assert!(
        blocks
            .iter()
            .filter(|o| o.blocker == elves)
            .all(|o| !o.attackers.contains(&djinn)),
        "and the untapped ground 1/1 with neither flying nor reach is offered \
         no such pairing, which is the half the keyword is about: {blocks:?}"
    );
}
