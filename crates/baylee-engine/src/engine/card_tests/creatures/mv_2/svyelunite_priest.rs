//! `cards/creatures/mv_2/svyelunite_priest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn svyelunite_priest() -> CardIndex {
    card_index("fb7adbee-a3b0-42bf-988f-9d4faf6bd006")
}

fn float(engine: &mut Engine<RegistryLookup>, color: ManaColor, n: u32) {
    engine.state.players[0].mana_pool.add(color, n);
    engine.refresh_offer();
}

fn board() -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[svyelunite_priest(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Svyelunite Priest (fb7adbee-a3b0-42bf-988f-9d4faf6bd006) — "{U}{U}, {T}:
/// Target creature gains shroud until end of turn. Activate only during your
/// upkeep."
#[test]
fn svyelunite_priest_cannot_be_activated_outside_your_upkeep() {
    let mut engine = board();
    let priest = on_battlefield(&engine, P0, svyelunite_priest()).expect("the Priest is out");
    float(&mut engine, ManaColor::Blue, 2);
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: priest,
                    ability_index: 0
                }
            )
            .is_err(),
        "the main phase is not the upkeep, with the mana in the pool"
    );
    assert!(!is_tapped(&engine, priest));
}

/// During my own upkeep it works: the target gains shroud, the Priest taps,
/// and the shroud ends with the turn.
#[test]
fn svyelunite_priest_grants_shroud_during_your_upkeep() {
    let mut engine = board();
    let priest = on_battlefield(&engine, P0, svyelunite_priest()).expect("the Priest is out");
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("the Ogre is out");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::SHROUD));

    // The opponent's upkeep is not mine either; walk through it to mine.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.step == Step::Upkeep
            && e.state().turn.number > 1
    });
    float(&mut engine, ManaColor::Blue, 2);
    activate(&mut engine, P0, svyelunite_priest(), 0);
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, priest), "{{T}} was paid");
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::SHROUD),
        "the Ogre has shroud"
    );
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, ogre).contains(KeywordSet::SHROUD),
        "shroud lasted until end of turn only"
    );
}
