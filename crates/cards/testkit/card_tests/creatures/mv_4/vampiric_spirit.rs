//! `cards/creatures/mv_4/vampiric_spirit.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vampiric Spirit prints two lines — flying and "When this creature enters,
/// you lose 4 life" — and the life total is read twice off one cast: once
/// while the card is still a spell on the stack, where nothing has been lost
/// yet, and once after the entry trigger has resolved, which is what tells the
/// printed loss from a board that was already low. The card is read on the
/// stack by `on_stack` rather than by the battlefield it has not reached. The
/// seat across the table is the control for the word "you": the life belongs
/// to the Spirit's controller and not to the table.
#[test]
fn vampiric_spirit_costs_its_controller_four_life_as_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[vampiric_spirit()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{B}{B} off four Swamps, and the printed loss is an *enters* trigger:
    // paying for the spell costs no life at all.
    cast_from_hand(&mut engine, p0, vampiric_spirit());
    assert!(
        on_stack(&engine, vampiric_spirit()).is_some(),
        "the card is still a spell waiting to resolve"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "nothing is lost to cast it — the loss is the entry trigger"
    );

    pass_until(&mut engine, stack_is_empty);

    let spirit = on_battlefield(&engine, p0, vampiric_spirit()).expect("the Spirit resolved");
    assert_eq!(pt(&engine, spirit), (4, 3), "the body the card prints");
    assert!(
        keywords(&engine, spirit).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].life,
        16,
        "\"you lose 4 life\" — four, and not a point per mana spent on it"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the loss belongs to the Spirit's controller, not to the opponent"
    );
}
