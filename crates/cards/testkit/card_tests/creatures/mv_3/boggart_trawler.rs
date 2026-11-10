//! `cards/creatures/mv_3/boggart_trawler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "When this creature enters, exile target player's graveyard."
///
/// The front face is cast as an ordinary creature spell, and its enters
/// trigger points at a *player* — `PlayerRel::Chosen`, the relation
/// `eval::players` cannot answer on its own. Both graveyards are seeded and
/// only one is named, because a resolution that read `Chosen` as "each
/// player" would empty the caster's own graveyard too and would otherwise
/// pass unnoticed.
///
/// The P/T assertion at the end is the other half of the same sentence: the
/// card that entered has to be the Goblin, not the land on its back.
#[test]
fn boggart_trawler_exiles_only_the_graveyard_its_enters_trigger_points_at() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(11, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[boggart_trawler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 3);
    seed_graveyard(&mut engine, p1, 3);
    let mine_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    assert_eq!(
        mine_before, 3,
        "both graveyards start with something in them"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        3
    );

    // The spell resolves, the Goblin enters, and the trigger stops the game
    // to ask. Without the target requirement the trigger stacks unasked, the
    // passing runs out of turns and this is where the test goes red.
    cast_from_hand(&mut engine, p0, boggart_trawler());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player_options,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a player choice, got {:?}", engine.pending())
    };
    assert!(
        options.is_empty(),
        "the exile points at a player, not an object"
    );
    assert_eq!(
        player_options,
        vec![p0, p1],
        "\"target player\" is anyone at the table (CR 115.1), the caster included"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the Goblin points at the opponent");

    pass_until(&mut engine, |e| {
        e.state().zones.list(ZoneLocation::Graveyard(p1)).is_empty()
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        3,
        "the cards are exiled, not merely gone"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        mine_before,
        "one graveyard was named and only that one is emptied"
    );
    let goblin = on_battlefield(&engine, p0, boggart_trawler()).expect("the Goblin landed");
    assert_eq!(
        pt(&engine, goblin),
        (3, 1),
        "the front face is what was cast and what entered"
    );
}

/// "As this land enters, you may pay 3 life. If you don't, it enters
/// tapped." — and then "{T}: Add {B}."
///
/// The back face is a land, so it is *played* rather than cast (CR 712.12),
/// and it is the card's only land face: the engine switches to it with no
/// mode question at all, which is what the face assertion pins down. What
/// follows is the `as … enters` replacement (CR 614.1c) being a real
/// question rather than a silent default, and the land the answer bought
/// actually making the black mana it prints.
#[test]
fn boggart_bog_pays_three_life_to_land_untapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7, forest()).hand(0, &[boggart_trawler()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, boggart_trawler());

    // One land face means no face choice: the card goes straight to the Bog
    // and the only thing standing between it and the battlefield is the
    // three life.
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Bog never asked about its three life: {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();

    assert_eq!(
        engine.state().players[0].life,
        life_before - 3,
        "three life is what the printed line asks for"
    );
    let obj = engine
        .state()
        .object(land)
        .expect("the Bog is on the board");
    assert_eq!(
        obj.face_index, 1,
        "the back face is the one that was played"
    );
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Boggart Bog"
    );
    assert!(obj.characteristics().types.contains(TypeSet::LAND));
    assert!(
        !entered_tapped(&engine, land),
        "the life was paid, so the land is untapped"
    );

    // "{T}: Add {B}." — a *printed* mana ability, offered at index 0 in
    // `legal.abilities`. `legal.mana_abilities` is CR 305.6's intrinsic
    // shortcut and the Bog prints no basic land type to take it, which is
    // why it is empty here and why the press below is `ActivateAbility`.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "expected priority after the answer, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert!(
        legal.mana_abilities.is_empty(),
        "Boggart Bog prints no basic land type, so it has no intrinsic mana"
    );
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped Boggart Bog is offered its own mana ability"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one land, one mana");
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the Bog taps for {{B}} and nothing else"
    );
}

/// The other side of the same choice: "If you don't, it enters tapped."
///
/// Declining has to cost nothing and tap the land — a replacement effect
/// that only ever fired on "yes" would look identical on the board a turn
/// later, and the tapped assertion is what tells the two apart.
///
/// The second half is what keeps the negative assertion honest. "The ability
/// is not offered" is also what a Bog with no mana ability at all would say,
/// so the test stays on the board until the next untap step (CR 502.3) and
/// presses the same index: what withheld it was the {T} in its own cost and
/// nothing else.
#[test]
fn boggart_bog_that_declines_the_three_life_waits_a_turn_for_its_black() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7, forest()).hand(0, &[boggart_trawler()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, boggart_trawler());
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!(
            "the Bog never asked about its three life: {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, YesNoPrompt::PayLifeOrEnterTapped { amount: 3 });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();

    assert_eq!(
        engine.state().players[0].life,
        life_before,
        "nothing was paid"
    );
    let obj = engine
        .state()
        .object(land)
        .expect("the Bog is on the board");
    assert_eq!(obj.face_index, 1, "still the land face");
    assert!(
        entered_tapped(&engine, land),
        "declining the three life is what puts it onto the battlefield tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "expected priority after the answer, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped cannot pay the {{T}} in its own ability"
    );

    // One turn cycle later the same land, the same index, and the {B} the
    // card prints.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !entered_tapped(&engine, land),
        "the untap step untaps what entered tapped"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "the ability was there all along; the tap was what withheld it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 0,
            },
        )
        .unwrap();
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the Bog taps for {{B}}"
    );
}
