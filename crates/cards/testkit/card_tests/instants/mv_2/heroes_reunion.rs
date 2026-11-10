//! `cards/instants/mv_2/heroes_reunion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heroes' Reunion costs {G}{W} and prints one sentence: "Target player gains
/// 7 life." Both halves of that need a witness. The *player* half is read by
/// aiming the two copies this seat holds at opposite seats — the question has
/// to offer both, so "target player" is not "target opponent" — and the
/// printed seven is read off two seats that start at different totals, so
/// 30 → 37 and 20 → 27 are numbers neither seat could have reached through
/// the other's cast. The four lands are tapped before anything is claimed
/// (CR 500.5 keeps the pool through this one main phase), and two cards in
/// the graveyard are what says each cast resolved rather than waiting.
#[test]
fn heroes_reunion_gives_seven_life_to_the_player_it_names_and_no_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), forest(), forest()])
        .hand(0, &[heroes_reunion(), heroes_reunion()])
        .life(0, 30)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `castable` is read off the pool and not off the
    // untapped lands, so the spell is this seat's to cast only once the four
    // lands have paid in.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "two Plains and two Forests: {{G}}{{W}} twice over"
    );

    // A target that is a player is one question in either of the shapes the
    // engine gives it — a target choice whose object list is empty and whose
    // player list is the whole of it, or the bare player choice — and both
    // enumerate the same answers (CR 115.4), so the answer is read out of
    // whichever shape arrives.
    let aim_at = |engine: &mut Engine<RegistryLookup>, at: PlayerId| -> Vec<PlayerId> {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![at],
                        },
                    )
                    .expect("the player aimed at was one of the options offered");
                player_options
            }
            Pending::ChoosePlayer { player, options } => {
                engine
                    .apply(player, PlayerAction::ChoosePlayer(at))
                    .expect("the player aimed at was one of the options offered");
                options
            }
            other => panic!("\"target player\" is a question about players, got {other:?}"),
        }
    };

    cast_with_floating(&mut engine, p0, heroes_reunion());
    let offered = aim_at(&mut engine, p1);
    assert!(
        offered.contains(&p0) && offered.contains(&p1),
        "\"target player\" is any player, on either side of the table: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        27,
        "the named seat gained exactly seven"
    );
    assert_eq!(
        engine.state().players[0].life,
        30,
        "and the seat that was not named did not"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{G}}{{W}} came out of the pool"
    );

    // The second copy, aimed the other way: "target player" is not "target
    // opponent", so the same card has to be able to hand its seven back to
    // the seat that cast it.
    cast_with_floating(&mut engine, p0, heroes_reunion());
    let offered = aim_at(&mut engine, p0);
    assert!(
        offered.contains(&p0),
        "the caster is a legal target for their own spell: {offered:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 37, "seven for the caster");
    assert_eq!(
        engine.state().players[1].life,
        27,
        "and the first seven are still the other seat's"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{G}}{{W}} came out of the pool"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2,
        "one resolved instant per cast — a spell stuck on the stack would \
         leave this at one"
    );
    assert!(
        in_graveyard(&engine, p0, heroes_reunion()).is_some(),
        "and the cards are where a resolved instant goes"
    );
}
