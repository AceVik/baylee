//! `cards/lands/manlands/ghitu_encampment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ghitu Encampment prints three lines — it enters tapped, it taps for {R}, and
/// `{1}{R}` animates it into a 2/1 red Warrior with first strike that "is still
/// a land" — and one board reads all three in order. The land drop is a real
/// `PlayLand`, because `starting_battlefield` places a permanent without an
/// entry and would arrive untapped whatever the card says; the turn cycle is
/// what lets the Encampment pay its own `{T}` (an arrives-tapped land gives
/// nothing in its arrival turn), with a single Mountain beside it so the two
/// red floating afterwards are exactly its own mana plus the Mountain's; and
/// the activation is then read off the layer projection, where a land becomes a
/// creature without stopping being a land.
#[test]
fn ghitu_encampment_enters_tapped_taps_for_red_and_animates_into_a_first_striking_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[ghitu_encampment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // "This land enters tapped" — a replacement effect on a real entry, so the
    // permanent is standing and down the moment it is played.
    let encampment = play_land(&mut engine, p0, ghitu_encampment());
    assert!(
        is_tapped(&engine, encampment),
        "the printed entry clause taps it as it arrives"
    );
    assert!(
        !types(&engine, encampment).contains(TypeSet::CREATURE),
        "and it is a plain land until something animates it"
    );

    // An arrives-tapped land gives nothing in its arrival turn, so the untap
    // step is the only thing that can make its own `{T}` payable.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, encampment),
        "the untap step stands it back up"
    );

    // {{R}} from the Encampment and {{R}} from the Mountain: exactly the
    // {{1}}{{R}} the animation charges, off a board with no other mana source,
    // so the second red is the printed `{T}: Add {R}`.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "one red from each of the two lands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and nothing else is in the pool"
    );
    assert!(
        is_tapped(&engine, encampment),
        "its own {{T}} is what made the mana"
    );
    assert!(
        !keywords(&engine, encampment).contains(KeywordSet::FIRST_STRIKE),
        "and it prints no keyword of its own"
    );

    // Ability 0 is the printed `{T}: Add {R}`; ability 1 is the animation.
    activate(&mut engine, p0, ghitu_encampment(), 1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{R}} is paid at CR 601.2h, and it is the whole price"
    );
    assert!(
        !stack_is_empty(&engine),
        "animating is no mana ability, so it waits on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, encampment);
    assert!(
        kinds.contains(TypeSet::CREATURE) && kinds.contains(TypeSet::LAND),
        "\"it becomes a 2/1 red Warrior creature ... It's still a land\": {kinds:?}"
    );
    assert_eq!(pt(&engine, encampment), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, encampment).contains(KeywordSet::FIRST_STRIKE),
        "and the keyword it prints with it"
    );
}
