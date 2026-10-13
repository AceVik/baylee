//! `cards/instants/mv_2/razorgrass_ambush.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The instant needs a creature in combat and deals three damage to it.
/// An empty battlefield must not offer a legal cast even with enough mana.
#[test]
fn razorgrass_ambush_burns_a_creature_that_is_attacking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(301, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Nothing is attacking yet, so there is no legal target and the spell
    // cannot be cast at all — the engine refuses it rather than offering an
    // empty target list. The mana is tapped first on purpose: without it the
    // refusal would be affordability and this assertion would prove nothing.
    let spell = in_hand(&engine, p0, razorgrass_ambush()).expect("the Ambush is in hand");
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0].mana_pool.total() >= 2,
        "two Plains pay {{1}}{{W}}, so what is refused below is the target"
    );
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "with no attacking creature the Ambush has nothing to target"
    );

    // A second game for the combat, because those two Plains are tapped now
    // and a seat's lands untap in its **own** untap step (CR 502.1) — on p1's
    // turn p0 would have had no mana, and "not castable" would have meant
    // something else entirely.
    let mut engine = Duel::new(302, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[razorgrass_ambush()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // p1's turn, and the Elf swings.
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        attackers.contains(&elf),
        "a creature that has been on the battlefield since before the game \
         may attack (CR 302.6): {attackers:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p0))],
            },
        )
        .expect("attacking the other seat is legal");

    // Now p0 has a target, and three damage on a 1/1 is lethal.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    cast_from_hand(&mut engine, p0, razorgrass_ambush());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Ambush asks for an attacking creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elf),
        "the attacking Elf is the legal target: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the attacking Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "three damage on a 1/1 is lethal (CR 704.5g)"
    );
}

#[test]
fn razorgrass_ambush_can_target_a_blocker() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        956,
        &[llanowar_elves()],
        &[llanowar_elves()],
        &[razorgrass_ambush()],
        &[],
    );
    let attacker = kit::obj(&engine, kit::P0, llanowar_elves());
    let blocker = kit::obj(&engine, kit::P1, llanowar_elves());
    kit::attackers_offered(&mut engine);
    kit::declare(&mut engine, &[attacker]);
    kit::block(&mut engine, &[(blocker, attacker)]);
    kit::float(&mut engine, kit::P0, &[(ManaColor::White, 2)]);
    kit::cast_at(&mut engine, kit::P0, razorgrass_ambush(), &[blocker], &[]);
    assert_eq!(
        engine.state().object(blocker).unwrap().zone,
        Zone::Graveyard
    );
}

/// "As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {W}." Both answers: paying leaves it untapped and 3 life lighter,
/// declining leaves life alone and the land tapped.
#[test]
fn razorgrass_field_pay_three_life_enters_untapped_and_declining_enters_tapped() {
    let p0 = PlayerId::new(0);
    for pay in [true, false] {
        let (mut engine, land) =
            play_land_face(razorgrass_ambush(), 1).expect("plays as Razorgrass Field");
        assert!(matches!(engine.pending(), Pending::YesNo {
            player, prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount: 3 }, ..
        } if *player == p0));
        engine.apply(p0, PlayerAction::YesNo(pay)).unwrap();
        assert_eq!(engine.state().object(land).unwrap().zone, Zone::Battlefield);
        assert_eq!(engine.state().object(land).unwrap().face_index, 1);

        assert_eq!(
            is_tapped(&engine, land),
            !pay,
            "paying 3 life enters untapped; declining enters tapped (pay = {pay})"
        );
        assert_eq!(
            engine.state().players[0].life,
            if pay { 17 } else { 20 },
            "3 life is lost exactly when it is paid (pay = {pay})"
        );

        if !pay {
            engine.dev_state_mut(p0).unwrap().set_tapped(land, false);
            engine.refresh_offer();
        }
        activate(&mut engine, p0, razorgrass_ambush(), 0);

        assert!(is_tapped(&engine, land), "{{T}} is the cost of the ability");
        let pool = &engine.state().players[0].mana_pool;
        assert_eq!(
            pool.available(ManaColor::White),
            1,
            "\"{{T}}: Add {{W}}\" adds one white mana (pay = {pay})"
        );
        assert_eq!(pool.total(), 1, "and no other mana (pay = {pay})");
    }
}
