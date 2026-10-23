//! Apocalypse Chime — `{2}` Artifact (Homelands)
//!
//! `{2}`, `{T}`, Sacrifice Apocalypse Chime: Destroy all nontoken permanents
//! with a name originally printed in the Homelands expansion. They can't be
//! regenerated.
//!
//! (`Coverage::Partial`: `OriginalExpansion` has no Homelands for
//! `Filter::NameOriginallyPrintedIn`; activating the `{2}, {T}, sacrifice`
//! ability sacrifices Apocalypse Chime and resolves with no effect.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn apocalypse_chime() -> CardIndex {
    card_index("995e2412-3f69-4aaf-a5d6-ae0ecbb95061")
}

#[test]
fn apocalypse_chime_casts_and_activates_two_tap_sacrifice_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[apocalypse_chime()])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let chime_hand = in_hand(&engine, p0, apocalypse_chime()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: chime_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chime = on_battlefield(&engine, p0, apocalypse_chime()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: chime,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(chime).unwrap().zone,
        Zone::Graveyard,
        "Apocalypse Chime is sacrificed to pay its activation cost"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
