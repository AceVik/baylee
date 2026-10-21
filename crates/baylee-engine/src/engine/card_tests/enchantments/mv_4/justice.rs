//! `cards/enchantments/mv_4/justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn justice() -> CardIndex {
    card_index("0586fdc7-3025-4040-8eaf-f8a57a21aff0")
}

/// Justice (0586fdc7-3025-4040-8eaf-f8a57a21aff0) — {2}{W}{W} Enchantment:
/// - "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}."
#[test]
fn justice_stays_when_two_white_paid_at_upkeep_and_sacrifices_when_declined() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[justice(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);

    // First upkeep: tap two Plains and pay {W}{W} -> Justice stays on the battlefield.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    lk::yes_no(&mut engine, true);
    tap_all_mana_but(&mut engine, P0, None);
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, P0);

    assert!(
        on_battlefield(&engine, P0, justice()).is_some(),
        "paying {{W}}{{W}} keeps Justice on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{W}}{{W}} was spent"
    );

    // Next upkeep: decline payment -> Justice is sacrificed.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    lk::yes_no(&mut engine, false);
    lk::settle(&mut engine);

    assert!(
        in_graveyard(&engine, P0, justice()).is_some(),
        "declining to pay {{W}}{{W}} sacrifices Justice"
    );
}
