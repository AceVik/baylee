//! `cards/enchantments/mv_3/arenson_s_aura.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arenson's Aura prints two lines: "{W}, Sacrifice an enchantment:
/// Destroy target enchantment" and "{3}{U}{U}: Counter target enchantment
/// spell." This scenario plays both. The first half sacrifices one of the
/// two own auras to destroy the opponent's enchantment — the sacrifice
/// menu shows that "an enchantment" means both own auras and neither the
/// creature nor the opponent's battlefield —, and the second half counters
/// the opponent's enchantment spell on the opponent's turn, which afterwards
/// lies in the graveyard instead of in play.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn arenson_s_aura_sacrifices_an_enchantment_to_destroy_one_and_counters_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                arenson_s_aura(),
                arenson_s_aura(),
                plains(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(
            1,
            &[luminarch_ascension(), baleful_strix(), plains(), plains()],
        )
        .hand(1, &[luminarch_ascension()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 erreicht seine eigene Hauptphase"
    );

    let auras = all_on_battlefield(&engine, p0, arenson_s_aura());
    assert_eq!(auras.len(), 2, "zwei eigene Auren, eine zahlt gleich");
    let (aura, fodder) = (auras[0], auras[1]);
    let theirs = on_battlefield(&engine, p1, luminarch_ascension()).expect("ihre Verzauberung");
    let strix = on_battlefield(&engine, p1, baleful_strix()).expect("their creature");

    // Only the Plain: the five Islands must remain for the second line,
    // and the pool is the only place where `can_afford` reads.
    let land = on_battlefield(&engine, p0, plains()).expect("ein Plain");
    assert_eq!(
        tap_mana_where(&mut engine, p0, |id| id == land),
        1,
        "das Plain und sonst nichts"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(aura, 0)),
        "{{W}} and an enchantment are both there, so the line is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, arenson_s_aura(), 0);

    // CR 601.2c before CR 601.2h: both questions are answered in the order
    // they actually arrive.
    let mut targets: Vec<ObjectId> = Vec::new();
    let mut menu: Vec<ObjectId> = Vec::new();
    loop {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                targets = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![theirs],
                        },
                    )
                    .expect("ihre Verzauberung war eine der Optionen");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "a price and no search, what it all says to a client"
                );
                assert_eq!((min, max), (1, 1), "exactly one enchantment");
                menu = options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![fodder],
                        },
                    )
                    .expect("die eigene Aura war eine der Optionen");
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected, while the Aura is being resolved: {other:?}"),
        }
    }

    assert!(
        targets.contains(&theirs),
        "\"Destroy target enchantment\" reaches the opponent's battlefield: {targets:?}"
    );
    assert!(
        !targets.contains(&strix),
        "a creature is not an enchantment — the filter is read: {targets:?}"
    );
    assert_eq!(menu.len(), 2, "die beiden eigenen Auren: {menu:?}");
    assert!(
        menu.contains(&aura) && menu.contains(&fodder),
        "\"eine Verzauberung, die du kontrollierst\" meint beide: {menu:?}"
    );
    assert!(
        !menu.contains(&strix),
        "a creature is not an enchantment: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "CR 701.21a: what belongs to the opponent, this seat does not sacrifice: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "die benannte Verzauberung ist zerstört"
    );
    assert!(
        in_graveyard(&engine, p1, luminarch_ascension()).is_some(),
        "and lies in its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, arenson_s_aura()).len(),
        1,
        "die geopferte Aura ist weg, die andere steht noch"
    );
    assert!(
        in_graveyard(&engine, p0, arenson_s_aura()).is_some(),
        "a sacrificed enchantment goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} is paid, and nothing more was floating"
    );

    // Second line: on the opponent's turn cast an enchantment and counter
    // it while it is on the stack.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].mana_pool.total(),
        2,
        "zwei Plains, und der Strix macht kein Mana"
    );
    cast_with_floating(&mut engine, p1, luminarch_ascension());
    let spell =
        on_stack(&engine, luminarch_ascension()).expect("die Verzauberung liegt auf dem Stapel");
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // First mana into the pool, then the claim about the offer.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "the five Islands; the Plains has been tapped since the first line"
    );
    let theirs_grave = engine.state().zones.list(ZoneLocation::Graveyard(p1)).len();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the opponent has passed");
    assert!(
        legal.abilities.contains(&(aura, 1)),
        "{{3}}{{U}}{{U}} is payable, so the second line is in the offer: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, arenson_s_aura(), 1);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"Target: enchantment spell\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "die aktivierende Sitz wählt");
    assert!(
        options.contains(&spell),
        "the spell on the stack is what \"enchantment spell\" names: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .expect("the spell was one of the options");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, luminarch_ascension()).is_none(),
        "the spell never arrived"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        theirs_grave + 1,
        "CR 701.6: a countered spell is the one extra card in its owner's \
         graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{U}}{{U}} ist bezahlt"
    );
    assert!(
        on_battlefield(&engine, p0, arenson_s_aura()).is_some(),
        "the second Aura counters the spell without sacrificing itself"
    );
}
