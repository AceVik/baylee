//! `cards/lands/utility/immersturm_skullcairn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Immersturm Skullcairn prints three sentences that are all engine answers
/// rather than card-file facts: it enters tapped, it taps for {B}, and
/// "{1}{B}{R}{R}, {T}, Sacrifice this land: It deals 3 damage to target
/// player. That player discards a card. Activate only as a sorcery."
///
/// Two copies stand on purpose, because that is the only way to tell the
/// printed entry clause from the harness: the seeded one was *placed* and is
/// untapped, the one played as this turn's land drop came in through a real
/// entry and is tapped — so the tap is the card's and not the setup's, and
/// the same reading says which copy can still pay the printed {T} in the
/// cost. The whole price is then played out for real: four lands tapped for
/// {1}{B}{R}{R}, the land sacrificed to itself, and the damage and the
/// discard read off p1 afterwards.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn immersturm_skullcairn_enters_tapped_and_trades_itself_for_damage_and_a_discard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                immersturm_skullcairn(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[immersturm_skullcairn()])
        // The seat the ability is aimed at needs a card it can be made to
        // lose, whatever the opening hand the harness deals out looks like.
        .hand(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches a main phase of its own"
    );

    let seeded = on_battlefield(&engine, p0, immersturm_skullcairn())
        .expect("the Skullcairn the harness placed is on the table");

    // The land drop, and the only thing that can read "This land enters
    // tapped": a permanent the harness placed never enters at all.
    let dropped = play_land(&mut engine, p0, immersturm_skullcairn());
    assert!(
        entered_tapped(&engine, dropped),
        "the Skullcairn came in through a land drop, so the printed clause \
         applies to it"
    );
    assert!(
        !is_tapped(&engine, seeded),
        "and the copy that merely was placed is still standing, so the tap \
         above is the card's clause and not the setup's"
    );

    // {1}{B}{R}{R} off two Swamps and two Mountains. Both Skullcairns are
    // kept back: one is the source whose printed {T} still has to be payed,
    // the other is this turn's land drop and already tapped.
    tap_all_mana_but(&mut engine, p0, Some(immersturm_skullcairn()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the four lands the cost is paid from, read off the pool"
    );

    let their_life = engine.state().players[1].life;
    let our_life = engine.state().players[0].life;
    let their_hand = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    // Ability 1 is the printed spell: ability 0 is the {T} for {B}.
    activate(&mut engine, p0, immersturm_skullcairn(), 1);

    // "target player" is a choice of seats, and a target that is only a
    // player is never an object.
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, p0, "the activating seat does the aiming");
            assert!(options.contains(&p1), "both seats are legal: {options:?}");
            engine
                .apply(player, PlayerAction::ChoosePlayer(p1))
                .expect("p1 was one of the options");
        }
        Pending::ChooseTargets {
            player,
            player_options,
            ..
        } => {
            assert_eq!(player, p0, "the activating seat does the aiming");
            assert!(
                player_options.contains(&p1),
                "both seats are legal targets: {player_options:?}"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![p1],
                    },
                )
                .expect("p1 was one of the options");
        }
        other => panic!("`target player` has to ask for a player, got {other:?}"),
    }

    let rest = drive_to_rest(&mut engine, p0);
    assert!(
        matches!(rest, Rest::Reached),
        "the ability resolves back to a quiet priority: {rest:?}"
    );

    assert_eq!(
        engine.state().players[1].life,
        their_life - 3,
        "\"it deals 3 damage to target player\" — the seat that was aimed at"
    );
    assert_eq!(
        engine.state().players[0].life,
        our_life,
        "and the seat that aimed it takes nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        their_hand - 1,
        "\"that player discards a card\""
    );
    assert_eq!(
        mine(&engine, p0, immersturm_skullcairn(), Zone::Battlefield).len(),
        1,
        "the land was sacrificed to pay its own cost, leaving only the copy \
         that was played and never tapped for anything"
    );
    assert_eq!(
        mine(&engine, p0, immersturm_skullcairn(), Zone::Graveyard).len(),
        1,
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{1}}{{B}}{{R}}{{R}} came out of the pool"
    );
}
