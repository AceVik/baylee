//! `cards/creatures/mv_1/llanowar_elves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Borg Queen, Perfection Manifest ({4}{B}{B}, 1/4): "Artifact creatures you
/// control get +2/+0. When Borg Queen enters, assimilate target creature card
/// from an opponent's graveyard. (Put it onto the battlefield under your
/// control with a +1/+1 counter. It's a Borg artifact creature and loses all
/// other creature types.)"
///
/// One landing plays both printed sentences at once, and they are read on the
/// same permanent on purpose: assimilate makes its victim an *artifact*
/// creature, so the anthem has to catch a creature that was not an artifact
/// when the anthem started. That is CR 613.1 — layer 4 hands the type change
/// to layer 7c — and the Elf's 4/2 is the only number on the board that could
/// not come out of any one clause alone: 1/1 printed, +2/+0 from the anthem,
/// then the +1/+1 counter on top.
///
/// Three counter-proofs keep each assertion from passing for the wrong reason.
/// The bystander Elf p0 already controls is a creature the anthem must *not*
/// touch, so a 1/1 there is the word "artifact" in the filter doing work
/// rather than a blanket pump. A creature card is seeded into p0's **own**
/// graveyard beside the one in p1's, so "an opponent's graveyard" is a choice
/// the engine makes and not the only card there was. And the victim is read on
/// the `ObjectId` it had while it was still a card in that graveyard, which is
/// what says it *moved* — `GraveyardToBattlefield` keeps the id across the
/// zone change, so the three effects behind the target all land on one object.
///
/// The last block is the other half of `Coverage::Partial`. The card's
/// `NOT SUPPORTED` note says the one clause it does not write is "and loses
/// all other creature types": nothing in `Modifier` subtracts a subtype. So
/// the assimilated Elf is asserted to still be an Elf Druid beside its new
/// Borg type — the gap spelled out, and written to **fail** the day a modifier
/// can set a creature-type set, which is the day this card stops being
/// `Partial`.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn a_landing_borg_queen_assimilates_their_creature_card_and_pumps_only_artifact_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[borg_queen_perfection_manifest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The creature the anthem must not reach: p0 controls it, and it is not
    // an artifact.
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("a plain Elf is out");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the bystander is an ordinary 1/1 before the Queen lands"
    );

    // A creature card in each graveyard, so "an opponent's" is a choice.
    seed_graveyard(&mut engine, p0, 1);
    seed_graveyard(&mut engine, p1, 1);
    let mine = in_graveyard(&engine, p0, llanowar_elves()).expect("one of mine is buried");
    let theirs = in_graveyard(&engine, p1, llanowar_elves()).expect("and one of theirs");

    cast_from_hand(&mut engine, p0, borg_queen_perfection_manifest());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the loop above waited for exactly this")
    };
    assert!(
        options.contains(&theirs),
        "the creature card in the opponent's graveyard is offered: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "and assimilate never reaches into my own graveyard: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the one card the trigger may point at");
    pass_until(&mut engine, stack_is_empty);

    // The victim moved, and it kept its id across the move.
    let assimilated = engine
        .state()
        .object(theirs)
        .expect("the assimilated card is still an object");
    assert_eq!(
        assimilated.zone,
        crate::zone::Zone::Battlefield,
        "it was put onto the battlefield"
    );
    assert_eq!(
        assimilated.controller, p0,
        "under the Queen's controller, not its owner's"
    );
    assert_eq!(
        assimilated
            .counters
            .get(baylee_cards_dsl::CounterKind::P1P1),
        1,
        "with a +1/+1 counter on it"
    );

    let c = assimilated.characteristics();
    assert!(
        c.types.contains(baylee_core::types::TypeSet::ARTIFACT),
        "it is an artifact creature now"
    );
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::BORG),
        "and a Borg"
    );

    // The anthem, read on both halves of its filter.
    let queen =
        on_battlefield(&engine, p0, borg_queen_perfection_manifest()).expect("the Queen landed");
    assert_eq!(
        pt(&engine, queen),
        (3, 4),
        "the Queen is an artifact creature she controls, so she pumps herself"
    );
    assert_eq!(
        pt(&engine, theirs),
        (4, 2),
        "1/1 printed, +2/+0 because assimilate made it an artifact, +1/+1 from \
         the counter"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and the Elf that is only a creature is left alone — the anthem's \
         subject is artifact creatures"
    );

    // `Coverage::Partial`: the clause that is not written.
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ELF)
            && c.subtypes
                .contains(baylee_core::generated::subtypes::creature::DRUID),
        "`and loses all other creature types` is the card's NOT SUPPORTED \
         note: no `Modifier` subtracts a subtype, so the assimilated Elf Druid \
         keeps both tribes beside Borg. When this fires, the clause has become \
         expressible and the card is no longer Coverage::Partial"
    );
}

/// Brazen Borrower // Petty Theft ({1}{U}{U}, 3/1): "Flash. Flying. This
/// creature can block only creatures with flying." — in front of Petty
/// Theft, an Adventure instant for {1}{U}: "Return target nonland permanent
/// an opponent controls to its owner's hand."
///
/// The card is played here the way it is played at a table, which is the
/// only way its four printed lines are all in one scene: the adventure goes
/// off on the **opponent's** turn, the permanent it names goes back to its
/// owner's hand, the card is exiled on its adventure (CR 715), and the
/// creature is cast out of that exile — still on the opponent's turn, which
/// is what flash buys (CR 702.8a against CR 117.1a). Every press goes
/// through the engine's own offer: `legal.castable` before each cast, the
/// `ChooseCastMode` list for which face, the `ChooseTargets` list for which
/// permanent.
///
/// The targeting list is where the printed restrictions are struck. The
/// opponent's artifact and the opponent's creature are both on it;
/// **their land** is not, which is `nonland`, and **my own creature** is
/// not, which is `an opponent controls`. A filter that lost either word
/// would still bounce the artifact and still pass a test that only looked
/// at the artifact.
///
/// Then the other half of `Coverage::Partial`, and the reason the card
/// carries it. "This creature can block only creatures with flying" is not
/// enforced: `combat::can_block` reads the attacker's flying and
/// unblockable and the blocker's flying and reach, and nothing in the
/// engine names the attackers a given blocker may be paired with. (Menace
/// is not in that list and was never a pairing question: CR 702.111b
/// restricts the whole declaration, so it is counted in
/// `Engine::declare_blockers` — #156.) So a 1/1
/// ground Elf attacks and the Borrower is offered against it, which the
/// printed line forbids. The assertion is written to say so and to break
/// the day it stops being true — a blocker with no legal attacker is
/// dropped from the offer entirely, so when the restriction lands, this
/// test fails and `Coverage::Partial` is what gets flipped.
#[test]
#[allow(clippy::too_many_lines)] // scenario script — step-by-step readability
fn the_borrower_flashes_out_of_its_own_adventure_and_then_blocks_a_ground_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(47, island())
        .hand(0, &[brazen_borrower()])
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[forest(), llanowar_elves(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("a creature of my own");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("and one of theirs");
    let their_land = on_battlefield(&engine, p1, forest()).expect("a land of theirs");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("the permanent to steal");

    // Held through their turn and cast in it: the stack is empty, the
    // active player is the other seat, and the card is a creature.
    walk_the_game_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let card = in_hand(&engine, p0, brazen_borrower()).expect("still in hand on their turn");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "flash puts the card on offer while the other seat is the active player"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("the offer is honoured");

    // Six Islands pay for either face, so the engine asks which is being
    // cast — the {1}{U}{U} creature or the {1}{U} adventure.
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected the face choice, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        options
            .iter()
            .any(|o| matches!(o.kind, crate::choice::CastModeKind::Normal)),
        "the creature is one way to cast the card: {options:?}"
    );
    let adventure = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Face(1)))
        .expect("and Petty Theft is the other");
    engine
        .apply(p0, PlayerAction::ChooseMode(adventure))
        .expect("the mode came out of the list that was offered");

    // "Target nonland permanent an opponent controls": both of their
    // nonland permanents, their land not, my own creature not.
    walk_the_game_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the walk waited for exactly this")
    };
    assert!(
        options.contains(&ring) && options.contains(&theirs),
        "every nonland permanent the opponent controls is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&their_land),
        "`nonland` keeps their Forest off the list: {options:?}"
    );
    assert!(
        !options.contains(&mine),
        "`an opponent controls` keeps my own creature off it: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .expect("the permanent the theft is aimed at");
    let stack = engine.state().zones.list(crate::zone::ZoneLocation::Stack);
    let spell = stack.last().copied().expect("a spell on the stack");
    let obj = engine.state().object(spell).expect("the spell exists");
    assert_eq!(obj.face_index, 1, "the adventure is the back face");
    assert_eq!(
        engine.state().names.get(obj.characteristics().name),
        "Petty Theft"
    );

    // It resolves, and the walk stops at this seat's next quiet priority —
    // still inside the same step, which is what keeps the pool alive.
    walk_the_game_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "the permanent Petty Theft named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, quiet_artifact()).is_some(),
        "and it is in its owner's hand"
    );
    let exiled = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == brazen_borrower()))
        })
        .expect("the card itself went on its adventure (CR 715)");
    assert!(
        engine
            .state()
            .object(exiled)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Adventure)),
        "wearing the rider that says it may be cast from there"
    );

    // Still the same step, so what the theft did not spend is still in the
    // pool — a step's end is what empties it (CR 500.5).
    assert!(
        engine.state().players[0].mana_pool.total() >= 3,
        "three of the six Islands are unspent, which is the creature's price"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&exiled),
        "the creature is on offer out of the exile its own adventure made"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: exiled })
        .expect("the offer is honoured");
    walk_the_game_until(&mut engine, |e| {
        on_battlefield(e, p0, brazen_borrower()).is_some()
    });
    let borrower = on_battlefield(&engine, p0, brazen_borrower()).expect("the creature landed");
    assert_eq!(pt(&engine, borrower), (3, 1), "a 3/1 Faerie Rogue");
    assert!(
        keywords(&engine, borrower).contains(baylee_cards_dsl::KeywordSet::FLYING),
        "and flying, the half of the evasion the engine does read"
    );
    assert_eq!(
        engine.state().turn.active,
        p1,
        "and all of it on the other seat's turn, which is what flash is for"
    );

    // Their ground creature attacks. It is summoning sick on the turn it
    // began the game under, so the loop walks to the combat where the
    // engine itself offers it (CR 508.1a).
    let mut attacked = false;
    for _ in 0..8 {
        walk_the_game_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = engine.pending().clone()
        else {
            unreachable!("the walk waited for exactly this")
        };
        if player == p1 && attackers.contains(&theirs) {
            engine
                .apply(
                    p1,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(theirs, baylee_core::ids::Defender::Player(p0))],
                    },
                )
                .expect("the attacker came out of the offer");
            attacked = true;
            break;
        }
        engine
            .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
            .expect("an empty attack is always legal");
    }
    assert!(
        attacked,
        "their creature attacks once it is no longer summoning sick"
    );

    let blockers = loop {
        match engine.pending().clone() {
            Pending::ChooseBlockers {
                player, blockers, ..
            } => {
                assert_eq!(player, p0, "the attack is aimed at me, so I am blocking");
                break blockers;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while reaching blockers: {other:?}"),
        }
    };
    let Some(offered) = blockers.iter().find(|o| o.blocker == borrower) else {
        panic!(
            "the Borrower was offered no attacker at all — either it is not being \
             read as an untapped creature, or the restriction below has been \
             implemented and this half of the test is the one to rewrite: \
             {blockers:?}"
        )
    };
    assert!(
        offered.attackers.contains(&theirs),
        "the printed line is `This creature can block only creatures with flying` \
         and the attacker on offer is a ground Elf. `combat::can_block` reads the \
         attacker's flying and unblockable and the blocker's flying and reach, \
         and no `Modifier` names the attackers one blocker may be paired \
         with — which is exactly what `Coverage::Partial` promises a player here. \
         The day this fires, the pairing has learned to say it and the card is no \
         longer Partial: {blockers:?}"
    );
}

/// Derevi, Empyrial Tactician ({G}{W}{U}, 2/3): "When Derevi enters **and
/// whenever a creature you control deals combat damage to a player**, you
/// may tap or untap target permanent."
///
/// One printed sentence and two trigger conditions, which is why this test
/// does not stop when she lands. The second half is the pool's only
/// `Trigger::DealsCombatDamageToPlayer` pointed at a *filter* rather than at
/// the equipped creature, so until now nothing had fired one off a creature
/// that was not the ability's own source — and Derevi deliberately never
/// attacks here. The Llanowar Elves does the connecting, which is the whole
/// of what "a creature you control" claims.
///
/// Both modes are taken, one per trigger, and each is struck against a
/// control: the enters trigger untaps the Island that just paid for her and
/// the Plains beside it stays down, so "target permanent" is one permanent
/// rather than a sweep.
#[test]
fn derevi_asks_on_both_her_triggers_and_moves_only_the_permanent_she_named() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(97, forest())
        .battlefield(0, &[forest(), plains(), island(), llanowar_elves()])
        .battlefield(1, &[plains()])
        .hand(0, &[derevi_empyrial_tactician()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let my_island = on_battlefield(&engine, p0, island()).expect("an Island of her own");
    let my_plains = on_battlefield(&engine, p0, plains()).expect("a Plains of her own");
    let their_plains = on_battlefield(&engine, p1, plains()).expect("the opponent's land");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves waited");

    // Exactly {G}{W}{U}: all three of her lands are spent, which is what
    // gives the untap mode something of consequence to point at.
    cast_from_hand(&mut engine, p0, derevi_empyrial_tactician());
    assert!(
        is_tapped(&engine, my_island) && is_tapped(&engine, my_plains),
        "both lands paid for her",
    );
    // Stopping at a quiet priority as well is what turns "the trigger never
    // fired" into a sentence instead of a hundred wasted passes.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && on_battlefield(e, p0, derevi_empyrial_tactician()).is_some())
    });
    tap_or_untap(&mut engine, p0, 1, my_island);
    assert!(
        !is_tapped(&engine, my_island),
        "\"you may ... untap target permanent\" — the Island she named is back up",
    );
    assert!(
        is_tapped(&engine, my_plains),
        "and only the one she named: the Plains beside it is still down",
    );

    // Her second trigger condition. The Elves has been there since before
    // turn one and swings on its controller's second turn.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(attackers.contains(&elves), "the Elves may attack");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elves, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();
    let life_before = engine.state().players[1].life;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCastMode { .. })
            || (stack_is_empty(e) && e.state().players[1].life < life_before)
    });
    assert_eq!(
        engine.state().players[1].life,
        life_before - 1,
        "the unblocked Elves connected, which is the event the second half of \
         the sentence listens for",
    );
    assert!(
        !is_tapped(&engine, their_plains),
        "the opponent's land is up before the trigger resolves",
    );
    tap_or_untap(&mut engine, p0, 0, their_plains);
    assert!(
        is_tapped(&engine, their_plains),
        "\"you may tap ... target permanent\" — and Derevi herself never \
         attacked, so the trigger read the Elves as \"a creature you control\"",
    );
}

/// Liliana the Repentant ({1}{B}, 2/2): "Whenever **another** creature or
/// planeswalker you control enters, mill two cards."
///
/// `Effect::Mill` had never been written with `PlayerRel::You` anywhere in
/// this pool — every other one names `Chosen` or `ControllerOfTarget` — so
/// *whose* library loses the two cards is the half worth striking, and the
/// opponent's is read as well: `You` is one seat and not the table.
///
/// Both arms of the filter are entered, a creature and a planeswalker, and
/// Liliana is **cast** rather than seeded so that her own entry is a real
/// one. `Filter::Another` is the word that keeps the trigger off it, and a
/// board built with her already standing on it could never say so.
#[test]
fn another_creature_or_planeswalker_entering_mills_you_two_and_liliana_herself_mills_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(71, forest())
        .battlefield(
            0,
            &[
                forest(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(
            0,
            &[
                liliana_the_repentant(),
                llanowar_elves(),
                karn_the_great_creator(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let library_before = library_size(&engine, p0);
    let their_library = library_size(&engine, p1);
    let graveyard_before = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .len();

    // Her own entry first. The Forest is the one land kept back, because it
    // is what pays for the Elves below.
    let wood = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    tap_mana_except(&mut engine, p0, wood);
    let her = in_hand(&engine, p0, liliana_the_repentant()).expect("she is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: her })
        .expect("the Swamps pay {1}{B}");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, liliana_the_repentant()).is_some(),
        "she resolved onto the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "`another` keeps her own trigger off her own entry"
    );

    // A creature, which is one arm of the filter.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elves arrived"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "another creature you control entering mills you two"
    );

    // And a planeswalker, which is the other.
    cast_from_hand(&mut engine, p0, karn_the_great_creator());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, karn_the_great_creator()).is_some(),
        "Karn arrived"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 4,
        "and so does another planeswalker you control"
    );
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .len(),
        graveyard_before + 4,
        "milled, so the four are in your graveyard and nowhere else"
    );
    assert_eq!(
        library_size(&engine, p1),
        their_library,
        "`PlayerRel::You` is you, and the opponent mills nothing"
    );
}

/// Liliana's second printed line: "Exhaust — {5}{B}: Return target creature
/// or planeswalker card from your graveyard to the battlefield. Put a +1/+1
/// counter on Liliana. Activate only as a sorcery."
///
/// Two effects in one resolution, and the second is what the card's own note
/// is about: `AddCounter` puts its counters on the *first target*, which here
/// is the card coming back, so the printed counter is placed through a filter
/// naming the source instead. Both ends are struck — the card that came back
/// keeps the body it prints, and Liliana is the one that grew.
///
/// The tail is the `Coverage::Partial`, which a test of the working mechanism
/// alone would keep quiet about. "(Activate each exhaust ability only once.)"
/// is not enforced: nothing on an object records what an ability has already
/// done, so the ability is offered — and pressed, and resolved — a second
/// time in the same main phase. Twelve Swamps is two activations' worth of
/// mana on purpose, and this test **fails the day exhaust is implemented**,
/// which is the day the card stops being `Partial`.
#[test]
fn liliana_reanimates_a_creature_takes_the_counter_herself_and_may_exhaust_twice() {
    let p0 = PlayerId::new(0);
    let mut field = vec![liliana_the_repentant()];
    field.extend([swamp(); 12]);
    let mut engine = Duel::new(72, llanowar_elves())
        .battlefield(0, &field)
        .start();
    keep_mulligans(&mut engine);
    // Two creature cards, so the second activation has something to point at
    // whether or not the first one's own arrival milled any.
    seed_graveyard(&mut engine, p0, 2);
    reach_main_phase(&mut engine, p0);

    let liliana = on_battlefield(&engine, p0, liliana_the_repentant()).expect("she is out");
    assert_eq!(pt(&engine, liliana), (2, 2), "the body she prints");
    let corpse = in_graveyard(&engine, p0, llanowar_elves()).expect("a creature card waits");

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, liliana_the_repentant(), 1);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the reanimation asks which card: {:?}", engine.pending())
    };
    assert!(
        options.contains(&corpse),
        "the creature card in your own graveyard is offered: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![corpse],
            },
        )
        .expect("a target taken out of the offer");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let back = on_battlefield(&engine, p0, llanowar_elves())
        .expect("the creature card came back under your control");
    assert_eq!(pt(&engine, liliana), (3, 3), "the +1/+1 counter is hers");
    assert_eq!(
        pt(&engine, back),
        (1, 1),
        "and not the reanimated card's, which keeps the body it prints"
    );

    // The gap the `Partial` names, read off an offer that should not be there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = legal.abilities;
    assert!(
        offered.contains(&(liliana, 1)),
        "exhaust is unenforced, so the once-per-game ability is offered a \
         second time with the mana still floating: {offered:?}"
    );
    let second = in_graveyard(&engine, p0, llanowar_elves()).expect("the other creature card");
    activate(&mut engine, p0, liliana_the_repentant(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![second],
            },
        )
        .expect("and the second activation goes through");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        mine(
            &engine,
            p0,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .len(),
        2,
        "two creature cards off one exhaust ability in one turn — when this \
         assertion fires, exhaust is enforced and the card is no longer Partial"
    );
    assert_eq!(pt(&engine, liliana), (4, 4), "and a second counter with it");
}

/// Marionette Apprentice ({1}{B}, 1/2): "Fabricate 1" and "Whenever
/// **another** creature **or artifact you control** is put into a graveyard
/// from the battlefield, each opponent loses 1 life."
///
/// The drain is one trigger reading a four-word filter, and three of those
/// words are only visible as a *difference* — a trigger that fired on every
/// death would pass any test that watched one permanent die. So four deaths
/// are played out on one board and the life total is read after each:
///
/// * the Mind Stone sacrificed to its own ability — "or artifact", which no
///   creature death can show;
/// * a Toxic Deluge for X=1, which kills the Llanowar Elves on **both**
///   sides at once: one drain, not two, because `Filter::ControlledByYou`
///   is what separates them, and a board where only my creature died could
///   not tell the two readings apart;
/// * the Apprentice itself, Vindicated — `Filter::Another` is `obj.id !=
///   this` and the id survives the move to the graveyard, so its own corpse
///   must take nothing. A dies trigger that read itself would drain here,
///   and the Deluge is deliberately X=1 rather than X=2 so that this death
///   stands alone instead of arriving in the same batch as the Elves'.
///
/// The other half is what the card stands at `Coverage::Partial` for.
/// `Fabricate 1` is not written — there is no Servo in `tokens::ALL` to
/// create and half a "choose one" would be a mandatory counter the player
/// never agreed to — so the Apprentice must arrive as the plain 1/2 the
/// face prints: no +1/+1 counter, no token, and no question asked on the
/// way (a mode choice would stop `pass_until` dead). The day fabricate is
/// written, those two assertions are what has to be deleted.
#[test]
#[allow(clippy::too_many_lines)] // three deaths on two sides, and the point is which of them drains
fn the_apprentice_skips_fabricate_and_drains_only_for_another_permanent_of_yours_that_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                mind_stone(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[marionette_apprentice(), toxic_deluge()])
        .battlefield(1, &[plains(), swamp(), plains(), llanowar_elves()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // ---- The Apprentice is cast, and fabricate is not written. ----
    let opening = engine.state().players[1].life;
    cast_from_hand(&mut engine, p0, marionette_apprentice());
    pass_until(&mut engine, stack_is_empty);
    let apprentice =
        on_battlefield(&engine, p0, marionette_apprentice()).expect("the Apprentice resolved");
    assert_eq!(
        pt(&engine, apprentice),
        (1, 2),
        "no +1/+1 counter: fabricate is the clause this card is Partial for",
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and no Servo either — the whole `Fabricate 1` is dropped, so the \
         Apprentice enters as the 1/2 its face prints",
    );
    assert_eq!(
        engine.state().players[1].life,
        opening,
        "an Apprentice merely entering takes nothing from anybody",
    );

    // ---- "or artifact": the Mind Stone eats itself for a card. ----
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let stone = on_battlefield(&engine, p0, mind_stone()).expect("the Stone is out");
    // Everything but the Stone, which still owes its own `{T}` and would
    // otherwise have been spent making the `{1}` it costs.
    tap_mana_except(&mut engine, p0, stone);
    let before_stone = engine.state().players[1].life;
    activate(&mut engine, p0, mind_stone(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, mind_stone()).is_some(),
        "the sacrifice is the activation cost, so the Stone is in the yard",
    );
    assert_eq!(
        engine.state().players[1].life,
        before_stone - 1,
        "an artifact of yours going to a graveyard from the battlefield \
         drains exactly as readily as a creature does",
    );

    // ---- Both Elves die in one sweep; only mine is mine. ----
    let before_deluge = engine.state().players[1].life;
    let deluge = in_hand(&engine, p0, toxic_deluge()).expect("the Deluge is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: deluge })
        .expect("a sorcery in an open main phase, off the mana still floating");
    let Pending::ChooseNumber { .. } = engine.pending().clone() else {
        panic!("the Deluge asks for X, got {:?}", engine.pending())
    };
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none()
            && on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "-1/-1 killed the 1/1 on each side, which is what makes the count \
         below a reading of the filter rather than of the board",
    );
    assert_eq!(
        engine.state().players[1].life,
        before_deluge - 1,
        "two creatures died and one of them was theirs: `you control` is \
         worth exactly one life here, and 2 would mean it is not read",
    );
    assert!(
        on_battlefield(&engine, p0, marionette_apprentice()).is_some(),
        "the 1/2 survived X=1, so its own death below is an event of its own",
    );

    // ---- `another`: the Apprentice's own corpse takes nothing. ----
    reach_their_main_phase(&mut engine, p1);
    let before_vindicate = engine.state().players[1].life;
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![apprentice],
            },
        )
        .expect("their removal may point at the Apprentice");
    pass_until(&mut engine, |e| {
        at_rest(e, p1) && in_graveyard(e, p0, marionette_apprentice()).is_some()
    });
    assert_eq!(
        engine.state().players[1].life,
        before_vindicate,
        "`another` is the word: the Apprentice keeps its id through the move \
         to the graveyard, so its own death is the one death it never reads",
    );
}

#[test]
fn an_apprentice_sees_simultaneous_deaths_but_not_later_deaths_from_the_graveyard() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(928, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                marionette_apprentice(),
                llanowar_elves(),
                llanowar_elves(),
                mind_stone(),
            ],
        )
        .hand(0, &[toxic_deluge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me));
    let stone = on_battlefield(&engine, me, mind_stone()).unwrap();
    tap_mana_except(&mut engine, me, stone);
    let life = engine.state().players[1].life;
    let deluge = in_hand(&engine, me, toxic_deluge()).unwrap();
    engine
        .apply(me, PlayerAction::CastSpell { card: deluge })
        .unwrap();
    engine.apply(me, PlayerAction::ChooseNumber(2)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, me, marionette_apprentice()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        life - 2,
        "the dying Apprentice sees both other creatures die with it"
    );
    activate(&mut engine, me, mind_stone(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, me, mind_stone()).is_some());
    assert_eq!(
        engine.state().players[1].life,
        life - 2,
        "a later sacrifice cannot trigger an Apprentice already in the graveyard"
    );
}

/// Stingcaster Mage ({1}{R}, 2/1): "Haste" and "When this creature enters,
/// target instant or sorcery card in your graveyard gains flashback until
/// end of turn. The flashback cost is equal to its mana cost."
///
/// One main phase holds the whole card. A Swords to Plowshares is spent on
/// the first Elf the ordinary way, so the card the trigger will point at got
/// into the graveyard by being *played*; the Mage then arrives, the trigger
/// offers that one card and nothing else, and afterwards `legal.castable`
/// names a card in a graveyard — an offer nothing in the pool had ever made,
/// because no test played `Effect::GrantFlashback` at all. The second cast
/// eats the second Elf, and the card is exiled rather than buried again
/// (CR 702.34a).
///
/// The attack at the end is the other printed line. The Mage entered this
/// very turn, so without haste it would be summoning sick (CR 302.6) and
/// `ChooseAttackers` would not name it — the offer is the assertion, and the
/// two life the defender loses is the same claim read off the board.
#[test]
#[allow(clippy::too_many_lines)] // scenario script — one main phase, read in order
fn a_hasty_wizard_flashes_back_a_spent_swords_and_swings_the_turn_it_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(37, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[swords_to_plowshares(), stingcaster_mage()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let elves = mine(
        &engine,
        p1,
        llanowar_elves(),
        crate::zone::Zone::Battlefield,
    );
    assert_eq!(elves.len(), 2, "one Elf per cast of the same Swords");
    let life_before = engine.state().players[1].life;

    // Six lands, tapped once. A mana pool empties at the end of a step
    // (CR 500.5) and nothing between here and the attack ends one, so all
    // three casts — {W}, {1}{R}, {W} again — are paid out of this pool.
    tap_all_mana(&mut engine, p0);

    // Swords to Plowshares on the first Elf.
    let swords_in_hand =
        in_hand(&engine, p0, swords_to_plowshares()).expect("the Swords is in hand");
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: swords_in_hand,
            },
        )
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("the Swords targets a creature, got {:?}", engine.pending())
    };
    assert!(options.contains(&elves[0]), "the Elf is a legal target");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        mine(
            &engine,
            p1,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .len(),
        1,
        "the first Elf is exiled",
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before + 1,
        "its controller gains life equal to its power",
    );

    // The Mage, cast the same way. Its ETB trigger asks for a target once
    // the creature has resolved onto the battlefield.
    let mage_card = in_hand(&engine, p0, stingcaster_mage()).expect("the Mage is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: mage_card })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let mage = on_battlefield(&engine, p0, stingcaster_mage()).expect("the Mage landed");
    let swords_card = in_graveyard(&engine, p0, swords_to_plowshares())
        .expect("the spent Swords is in its owner's graveyard");
    let Pending::ChooseTargets { options, min, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    assert_eq!(
        options,
        vec![swords_card],
        "the instant in your own graveyard, and nothing on the battlefield",
    );
    assert_eq!(min, 1, "the trigger is not an optional one");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![swords_card],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // The grant is an *offer*: the engine now lists a card in a graveyard
    // among the things this seat may cast.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(
        legal.castable.contains(&swords_card),
        "the granted flashback makes the graveyard card castable",
    );

    // Cast it from the graveyard, at the second Elf. "The flashback cost is
    // equal to its mana cost" is a claim about a number, so the pool is what
    // reads it: one white mana leaves it and nothing else does.
    let pool_before = engine.state().players[0].mana_pool.total();
    engine
        .apply(p0, PlayerAction::CastSpell { card: swords_card })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the flashed-back Swords targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&elves[1]),
        "the surviving Elf is a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves[1]],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        pool_before - 1,
        "the flashback cost is the printed mana cost, one white and no more",
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        mine(
            &engine,
            p1,
            llanowar_elves(),
            crate::zone::Zone::Battlefield
        )
        .is_empty(),
        "one Swords, cast twice, exiled both Elves",
    );
    assert_eq!(
        engine.state().players[1].life,
        life_before + 2,
        "and gained its controller one life each time",
    );
    assert!(
        in_graveyard(&engine, p0, swords_to_plowshares()).is_none(),
        "a card cast for flashback does not go back to the graveyard",
    );
    let exiled = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Exile(p0))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == swords_to_plowshares()))
        });
    assert!(exiled, "it is exiled instead (CR 702.34a)");

    // Haste: the Mage entered this turn and attacks anyway.
    let life_after_flashback = engine.state().players[1].life;
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("just checked")
    };
    assert!(
        attackers.contains(&mage),
        "a creature that entered this turn is offered as an attacker: haste",
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(mage, baylee_core::ids::Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().players[1].life < life_after_flashback
    });
    assert_eq!(
        engine.state().players[1].life,
        life_after_flashback - 2,
        "an unblocked 2/1 that was never summoning sick",
    );
}

/// "If you don't cast it, put that card into your hand."
#[test]
fn trumpeting_carnosaur_declined_puts_the_card_into_the_hand() {
    let p0 = PlayerId::new(0);
    let (mut engine, ids) = carnosaur_discovers(&[llanowar_elves()], &[]);
    assert!(matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::Discover { .. },
            ..
        }
    ));
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_hand(&engine, p0, llanowar_elves()), Some(ids[0]));
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());
}
