//! `cards/instants/mv_3/seething_song.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Seething Song prints one line — "Add {R}{R}{R}{R}{R}" — on an instant that
/// costs {2}{R}, so the card is a price and a colour and nothing else. The end
/// step is where the two halves meet: a sorcery could not be cast there at all,
/// so the engine naming the Song in `legal.castable` with an empty stack is the
/// printed "Instant" rather than a reading of the card file, and the five red
/// that land in the pool afterwards come out of exactly the three Mountains the
/// {2}{R} cost — which is why the pool reads five and not eight.
#[test]
fn seething_song_trades_three_mana_for_five_red_in_the_end_step() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[seething_song()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let song = in_hand(&engine, p0, seething_song()).expect("the Song is in hand");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the board is three untapped Mountains and nothing floating"
    );

    // Through combat and into the end step of p0's own turn: the active player
    // opens that priority round (CR 117.3a) and no sorcery may be cast in it.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Mountains, and nothing else on this board makes mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&song),
        "\"Instant\": with an empty stack in the end step the Song is castable, \
         which is precisely the moment a sorcery is not: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, seething_song());
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        5,
        "\"Add {{R}}{{R}}{{R}}{{R}}{{R}}\" — five of the one colour the card names"
    );
    assert_eq!(
        pool.total(),
        5,
        "and nothing beside them: the {{2}}{{R}} was spent out of the three \
         Mountains rather than left floating under the new mana"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Green,
    ] {
        assert_eq!(
            pool.available(color),
            0,
            "a Song that added \"one mana of any color\" five times would leave \
             {color:?} in the pool here"
        );
    }
    assert!(
        in_graveyard(&engine, p0, seething_song()).is_some(),
        "an instant that resolved is in its owner's graveyard, not merely gone \
         from the hand"
    );
    assert!(
        stack_is_empty(&engine),
        "and nothing it started is still waiting to resolve"
    );
}
