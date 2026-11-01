//! `cards/creatures/mv_3/frenetic_efreet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);

fn frenetic_efreet() -> CardIndex {
    card_index("c21d3360-8be0-477e-a592-7e64a76570d3")
}

/// Frenetic Efreet (c21d3360-8be0-477e-a592-7e64a76570d3) — {1}{U}{R} 2/1:
/// "Flying. {0}: Flip a coin. If you win the flip, this creature phases out.
/// If you lose the flip, sacrifice this creature."
#[test]
fn frenetic_efreet_flips_coin_to_phase_out_or_be_sacrificed() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[frenetic_efreet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let efreet = on_battlefield(&engine, P0, frenetic_efreet()).unwrap();
    assert!(
        engine
            .state()
            .object(efreet)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    activate(&mut engine, P0, frenetic_efreet(), 0);
    pass_until(&mut engine, stack_is_empty);

    let phased_out = engine
        .state()
        .object(efreet)
        .is_some_and(|o| o.status.contains(Status::PHASED_OUT));
    let sacrificed = on_battlefield(&engine, P0, frenetic_efreet()).is_none();
    assert!(
        phased_out || sacrificed,
        "Frenetic Efreet must either phase out or be sacrificed"
    );
}

/// Every flip the journal holds, in order, as "won".
fn flips(engine: &Engine<RegistryLookup>) -> Vec<bool> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .collect()
}

/// "If you win the flip, this creature phases out. If you lose the flip,
/// sacrifice this creature.": over sixteen seeds both flips occur; a won flip
/// phases it out (and it is back after P0's next untap step), a lost flip
/// puts it in the graveyard.
#[test]
fn frenetic_efreet_phases_out_exactly_when_the_flip_is_won_and_is_sacrificed_when_lost() {
    let mut won_seen = false;
    let mut lost_seen = false;
    for seed in 1..=16u64 {
        let mut engine = Duel::new(seed, island())
            .battlefield(0, &[frenetic_efreet()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let efreet = on_battlefield(&engine, P0, frenetic_efreet()).unwrap();

        activate(&mut engine, P0, frenetic_efreet(), 0);
        pass_until(&mut engine, stack_is_empty);
        let seen = flips(&engine);
        assert_eq!(seen.len(), 1, "one activation, one flip");

        let obj = engine.state().object(efreet).unwrap();
        if seen[0] {
            won_seen = true;
            assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "seed {seed}");
            assert!(obj.status.contains(Status::PHASED_OUT), "seed {seed}");

            pass_until(&mut engine, |e| {
                e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
            });
            assert!(
                !engine
                    .state()
                    .object(efreet)
                    .unwrap()
                    .status
                    .contains(Status::PHASED_OUT),
                "seed {seed}: phased in at P0's next untap step"
            );
        } else {
            lost_seen = true;
            assert_eq!(obj.zone, crate::zone::Zone::Graveyard, "seed {seed}");
        }
    }
    assert!(won_seen && lost_seen, "sixteen seeds show both sides");
}
