//! `cards/enchantments/rooms/mv_3/walk_in_closet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Walk-In Closet's front door is one sentence — "You may play lands from
/// your graveyard" — and it is played from both sides of the permission,
/// because a static that is always on and a static that never fires look the
/// same from inside one game. The same board with the enchantment absent
/// refuses the land drop; with it on the battlefield, the graveyard's Forest
/// is a land drop and is taken.
///
/// Cast as its left half, the Room enters with that door unlocked and the
/// other locked (CR 709.5d), so Forgotten Cellar's trigger is not there to
/// hear anything: the Closet resolves to an empty stack.
#[test]
fn walk_in_closet_lets_its_controller_play_a_land_out_of_the_graveyard() {
    let p0 = PlayerId::new(0);

    // Without it, to show the offer is the enchantment's and not the board's.
    let mut bare = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    seed_graveyard(&mut bare, p0, 1);
    let buried = bare.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let Pending::Priority { legal, .. } = bare.pending().clone() else {
        panic!("expected priority, got {:?}", bare.pending())
    };
    assert!(
        !legal.lands.contains(&buried),
        "a land in a graveyard is not a land drop by itself"
    );

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[walk_in_closet()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, walk_in_closet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let room = on_battlefield(&engine, p0, walk_in_closet())
        .expect("the left half resolves to a Room on the battlefield");
    assert_eq!(
        doors_of(&engine, room),
        crate::object::Doors::room(0b01),
        "cast as Walk-In Closet, that door is unlocked and Forgotten Cellar's is not"
    );
    assert!(
        stack_is_empty(&engine),
        "the locked door's trigger heard nothing"
    );

    seed_graveyard(&mut engine, p0, 1);
    let buried = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&buried),
        "\"You may play lands from your graveyard\""
    );
    engine
        .apply(p0, PlayerAction::PlayLand { card: buried })
        .expect("the land drop the enchantment granted");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&buried),
        "and the Forest left the graveyard for the battlefield"
    );
}

/// Forgotten Cellar, cast as its own half: "When you unlock this door, ...
/// if a card would be put into your graveyard from anywhere this turn,
/// exile it instead."
///
/// Cast as the right half, the Room enters with that door unlocked
/// (CR 709.5d), and the unlock is what the trigger hears: CR 709.5h makes
/// entering that way an unlock. The permanent is Forgotten Cellar and not
/// Walk-In Closet, a five and not a three, with the left door's static
/// absent. Once the trigger resolves, the seat's own Giant Growth is exiled
/// as it would reach the graveyard, the other seat's reaches theirs ("your
/// graveyard"), and next turn a second Giant Growth reaches the graveyard
/// again ("this turn"). Then the left door is unlocked for its {2}{G}, and
/// that fires nothing: the trigger is Forgotten Cellar's door's, not any
/// door's. Only then is the graveyard's Forest a land drop.
///
/// The clause "you may cast spells from your graveyard this turn" is the
/// next two tests'.
#[allow(clippy::too_many_lines)] // One Room, played across two turns.
#[test]
fn forgotten_cellar_cast_as_its_half_opens_that_door_and_exiles_what_would_reach_the_graveyard_this_turn()
 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[walk_in_closet(), giant_growth(), giant_growth()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    float_green(&mut engine, p0, 6);
    cast_forgotten_cellar(&mut engine, p0);
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, walk_in_closet()).is_some()
    });
    let room = on_battlefield(&engine, p0, walk_in_closet()).expect("the Room entered");
    assert_eq!(
        doors_of(&engine, room),
        crate::object::Doors::room(0b10),
        "cast as Forgotten Cellar, that door enters unlocked and the other locked (CR 709.5d)"
    );
    assert_eq!(
        room_abilities_on_stack(&engine, room),
        [0],
        "\"When you unlock this door\" heard the door the Room entered with (CR 709.5h)"
    );
    assert_eq!(
        name_value_colors(&engine, room),
        (
            "Forgotten Cellar".to_string(),
            5,
            baylee_core::color::ColorSet::of(baylee_core::color::Color::Green)
        ),
        "a locked half has no name or mana cost (CR 709.5): the Room is the Cellar alone"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    // The seat's own card, on its way to the graveyard from the stack.
    grow_own_elf(&mut engine, p0, p0);
    assert_eq!(
        in_graveyard(&engine, p0, giant_growth()),
        None,
        "\"if a card would be put into your graveyard from anywhere this turn\""
    );
    assert!(
        exiled_card(&engine, p0, giant_growth()).is_some(),
        "\"exile it instead\""
    );

    // The other seat's, cast while p0's main phase is open.
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("p0 passes with the stack empty");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "p1 holds priority in p0's main phase, got {:?}",
        engine.pending()
    );
    float_green(&mut engine, p1, 1);
    grow_own_elf(&mut engine, p1, p0);
    assert!(
        in_graveyard(&engine, p1, giant_growth()).is_some(),
        "the Cellar's controller's graveyard, and not every graveyard"
    );

    // Next turn: the effect lasted this turn only.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);
    float_green(&mut engine, p0, 1);
    grow_own_elf(&mut engine, p0, p0);
    assert!(
        in_graveyard(&engine, p0, giant_growth()).is_some(),
        "\"this turn\" was last turn"
    );

    // The left door is still locked, so its static is not there.
    seed_graveyard(&mut engine, p0, 1);
    let buried = *engine
        .state()
        .zones
        .list(ZoneLocation::Graveyard(p0))
        .iter()
        .find(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .expect("a Forest in the graveyard");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.lands.contains(&buried),
        "a locked half has no rules text (CR 709.5): no land drop from the graveyard"
    );

    float_green(&mut engine, p0, 3);
    assert_eq!(
        unlocks_offered(&engine, room),
        [0],
        "the Closet's door is offered"
    );
    unlock(&mut engine, p0, room, 0).expect("the left door unlocks for {2}{G}");
    assert_eq!(doors_of(&engine, room), crate::object::Doors::room(0b11));
    assert!(
        stack_is_empty(&engine),
        "unlocking Walk-In Closet is not unlocking Forgotten Cellar: the trigger is that door's"
    );
    assert_eq!(
        name_value_colors(&engine, room).1,
        8,
        "both halves' mana costs, {{2}}{{G}} and {{3}}{{G}}{{G}}"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&buried),
        "\"You may play lands from your graveyard\", now that its door is open"
    );
}

/// Forgotten Cellar's other clause: "When you unlock this door, you may cast
/// spells from your graveyard this turn".
///
/// A Giant Growth in the seat's graveyard is not castable before the door
/// opens, with its {G} floating, and is once the trigger resolves: an
/// instant, which the permanent-only permissions beside this one never let
/// through. Cast on the seat's Elf, it is not a flashback (CR 702.34a), so
/// it carries no flashback rider on the stack; it resolves (+3/+3) and is
/// exiled by the Cellar's own replacement as it would reach the graveyard.
/// The other seat's Giant Growth, in their own graveyard, is not theirs to
/// cast: the permission is its controller's. Next turn a Giant Growth in
/// the seat's graveyard is not castable again ("this turn").
#[allow(clippy::too_many_lines)] // One permission, from both sides and across a turn.
#[test]
fn forgotten_cellar_lets_its_controller_cast_spells_from_the_graveyard_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[walk_in_closet(), giant_growth(), giant_growth()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .hand(1, &[giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let growth = hand_to_graveyard(&mut engine, p0, giant_growth());
    let theirs = hand_to_graveyard(&mut engine, p1, giant_growth());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");

    float_green(&mut engine, p0, 1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&growth),
        "before the door opens, an instant in the graveyard is not castable, its {{G}} floating"
    );

    float_green(&mut engine, p0, 5);
    cast_forgotten_cellar(&mut engine, p0);
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, walk_in_closet()).is_some()
    });
    pass_until(&mut engine, |e| at_rest(e, p0));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&growth),
        "\"you may cast spells from your graveyard this turn\": an instant too"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: growth })
        .expect("the Giant Growth in the graveyard is cast");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    let spell = on_stack(&engine, giant_growth()).expect("Giant Growth is on the stack");
    assert!(
        !engine
            .state()
            .object(spell)
            .expect("the spell exists")
            .riders
            .contains(&crate::object::Rider::Flashback),
        "a permission to cast from the graveyard is not flashback (CR 702.34a)"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(power_of(&engine, elf), Some(4), "and it resolved: +3/+3");
    assert_eq!(
        in_graveyard(&engine, p0, giant_growth()),
        None,
        "it did not go back to the graveyard"
    );
    assert!(
        exiled_card(&engine, p0, giant_growth()).is_some(),
        "the Cellar's replacement exiled it"
    );

    // The other seat, holding priority in p0's main phase with {G} floating.
    engine
        .apply(p0, PlayerAction::PassPriority)
        .expect("p0 passes with the stack empty");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "p1 holds priority in p0's main phase, got {:?}",
        engine.pending()
    );
    float_green(&mut engine, p1, 1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&theirs),
        "the permission is the Cellar's controller's, and not every player's"
    );

    // Next turn: the permission lasted this turn only.
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    reach_their_main_phase(&mut engine, p0);
    let again = hand_to_graveyard(&mut engine, p0, giant_growth());
    float_green(&mut engine, p0, 1);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&again),
        "\"this turn\" was last turn"
    );
}

/// A card with printed flashback in the graveyard, under Forgotten Cellar's
/// permission, is offered at its mana cost beside its flashback cost: the
/// permission casts any spell from there, and a flashback cost is an
/// alternative cost the caster "may pay rather than paying the spell's mana
/// cost" (CR 118.9), not the only price. Memory Deluge, with seven
/// floating: `Normal` for {2}{U}{U} and `Flashback` for {5}{U}{U}. Cast at
/// its mana cost, the flashback cost was not paid, so the card carries no
/// flashback rider on the stack.
#[test]
fn under_forgotten_cellar_a_flashback_card_is_offered_at_its_mana_cost_too() {
    let p0 = PlayerId::new(0);
    // Memory Deluge: {2}{U}{U}, flashback {5}{U}{U}.
    let deluge = card_index("e6fd55f2-7e26-469c-a44a-ea2eb90e19a9");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                island(),
                island(),
            ],
        )
        .hand(0, &[walk_in_closet(), deluge])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let buried = hand_to_graveyard(&mut engine, p0, deluge);
    float_green(&mut engine, p0, 5);
    cast_forgotten_cellar(&mut engine, p0);
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, walk_in_closet()).is_some()
    });
    pass_until(&mut engine, |e| at_rest(e, p0));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: buried })
        .expect("Memory Deluge is cast from the graveyard");
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "two ways to cast it, so the seat is asked, got {:?}",
            engine.pending()
        )
    };
    let offered: Vec<_> = options.iter().map(|o| (o.kind, o.cost)).collect();
    assert_eq!(
        offered,
        [
            (
                CastModeKind::Normal,
                baylee_core::mana::ManaCost::parse("{2}{U}{U}")
            ),
            (
                CastModeKind::Flashback,
                baylee_core::mana::ManaCost::parse("{5}{U}{U}")
            ),
        ],
        "its mana cost under the permission, and its flashback cost"
    );
    let normal = options
        .iter()
        .position(|o| o.kind == CastModeKind::Normal)
        .expect("the mana cost is offered");
    engine
        .apply(p0, PlayerAction::ChooseMode(normal))
        .expect("cast for its mana cost");
    let spell = on_stack(&engine, deluge).expect("Memory Deluge is on the stack");
    assert!(
        !engine
            .state()
            .object(spell)
            .expect("the spell exists")
            .riders
            .contains(&crate::object::Rider::Flashback),
        "the flashback cost was not paid (CR 702.34a)"
    );
}

/// The unlock itself: "As a sorcery, you may pay the mana cost of a locked
/// door to unlock it" (CR 709.5e), a special action (CR 116.2m).
///
/// Walk-In Closet is cast as its left half and Forgotten Cellar is unlocked
/// later, which is how the card is mostly played. The unlock is offered only
/// with the Cellar's {3}{G}{G} floating and the stack empty in the seat's own
/// main phase, and refused otherwise: with nothing floating, and with a
/// Giant Growth on the stack. Taken, it uses no stack, and the Cellar's
/// trigger hears it; the Room is then both halves, a mana value of eight,
/// and keeps the Closet's static beside the trigger.
#[allow(clippy::too_many_lines)] // One unlock, refused twice and then taken.
#[test]
fn a_locked_door_unlocks_as_a_sorcery_for_its_mana_cost_and_its_trigger_hears_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[walk_in_closet(), giant_growth()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    float_green(&mut engine, p0, 3);
    cast_with_floating(&mut engine, p0, walk_in_closet());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let room = on_battlefield(&engine, p0, walk_in_closet()).expect("the Closet resolved");
    assert_eq!(doors_of(&engine, room), crate::object::Doors::room(0b01));
    seed_graveyard(&mut engine, p0, 1);
    let buried = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];

    assert_eq!(
        unlocks_offered(&engine, room),
        [] as [u8; 0],
        "nothing floating, nothing offered"
    );
    assert!(
        unlock(&mut engine, p0, room, 1).is_err(),
        "and the unlock is refused"
    );

    // With a spell on the stack, the Cellar's cost floating is not enough.
    float_green(&mut engine, p0, 1);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    cast_with_floating(&mut engine, p0, giant_growth());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    float_green(&mut engine, p0, 5);
    assert!(
        on_stack(&engine, giant_growth()).is_some(),
        "Giant Growth waits on the stack"
    );
    assert_eq!(
        unlocks_offered(&engine, room),
        [] as [u8; 0],
        "not while the stack holds a spell: \"as a sorcery\""
    );
    assert!(
        unlock(&mut engine, p0, room, 1).is_err(),
        "and the unlock is refused"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        unlocks_offered(&engine, room),
        [1],
        "the stack empty, the Cellar's {{3}}{{G}}{{G}} floating: the locked door, and only it"
    );
    unlock(&mut engine, p0, room, 1).expect("Forgotten Cellar unlocks");
    assert_eq!(doors_of(&engine, room), crate::object::Doors::room(0b11));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "a special action uses no stack (CR 116.1); what is there is the trigger"
    );
    assert_eq!(
        room_abilities_on_stack(&engine, room),
        [1],
        "\"When you unlock this door\", second in the Room's list with both doors open"
    );
    assert_eq!(
        name_value_colors(&engine, room),
        (
            "Walk-In Closet".to_string(),
            8,
            baylee_core::color::ColorSet::of(baylee_core::color::Color::Green)
        ),
        "both halves' mana costs make its mana value"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.lands.contains(&buried),
        "the Closet's static is still there with both doors open"
    );
}

/// A Room put onto the battlefield without being cast has neither door
/// unlocked (CR 709.5d): no name, no mana cost and no rules text (CR 709.5),
/// so a mana value of zero and no graveyard land drop. Each door is then
/// unlocked in turn, the left one firing nothing and the right one its
/// trigger; and once Naturalize destroys the Room, the card it leaves is the
/// card again, left half first, with no doors (CR 400.7). This turn that is
/// an exiled card: the Cellar's replacement caught the Room itself.
#[allow(clippy::too_many_lines)] // One Room, from locked to gone.
#[test]
fn a_room_put_onto_the_battlefield_uncast_has_both_doors_locked() {
    let p0 = PlayerId::new(0);
    let mut board = vec![walk_in_closet()];
    board.extend(std::iter::repeat_n(forest(), 10));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[naturalize()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let room = on_battlefield(&engine, p0, walk_in_closet()).expect("the Room is out");
    assert_eq!(
        doors_of(&engine, room),
        crate::object::Doors::room(0),
        "not cast, so neither door is unlocked (CR 709.5d)"
    );
    assert_eq!(
        name_value_colors(&engine, room),
        (String::new(), 0, baylee_core::color::ColorSet::EMPTY),
        "no name and no mana cost, so no colour either (CR 709.5)"
    );
    seed_graveyard(&mut engine, p0, 1);
    let buried = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.lands.contains(&buried),
        "no rules text, so no land drop from the graveyard"
    );

    float_green(&mut engine, p0, 3);
    assert_eq!(
        unlocks_offered(&engine, room),
        [0],
        "{{2}}{{G}} floating opens the Closet and not the Cellar"
    );
    unlock(&mut engine, p0, room, 0).expect("the left door unlocks");
    assert!(stack_is_empty(&engine), "the Closet's door fires nothing");
    assert_eq!(
        name_value_colors(&engine, room),
        (
            "Walk-In Closet".to_string(),
            3,
            baylee_core::color::ColorSet::of(baylee_core::color::Color::Green)
        )
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(legal.lands.contains(&buried), "the Closet's static is on");

    float_green(&mut engine, p0, 5);
    assert_eq!(unlocks_offered(&engine, room), [1]);
    unlock(&mut engine, p0, room, 1).expect("the right door unlocks");
    assert_eq!(room_abilities_on_stack(&engine, room), [1]);
    pass_until(&mut engine, |e| at_rest(e, p0));

    float_green(&mut engine, p0, 2);
    cast_with_floating(&mut engine, p0, naturalize());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![room],
            },
        )
        .expect("the Room is an enchantment to destroy");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        on_battlefield(&engine, p0, walk_in_closet()).is_none(),
        "destroyed"
    );
    assert_eq!(
        in_graveyard(&engine, p0, walk_in_closet()),
        None,
        "the Cellar's replacement is on this turn, and the Room is a card"
    );
    let card = exiled_card(&engine, p0, walk_in_closet()).expect("exiled instead");
    let gone = engine.state().object(card).expect("the exiled card");
    assert_eq!(
        (gone.doors, gone.face_index),
        (crate::object::Doors::NONE, 0),
        "the designations were the permanent's (CR 400.7)"
    );
    assert_eq!(
        (
            engine
                .state()
                .names
                .get(gone.characteristics().name)
                .to_string(),
            gone.characteristics().mana_cost
        ),
        (
            "Walk-In Closet".to_string(),
            "{2}{G}"
                .parse::<baylee_core::mana::ManaCost>()
                .expect("a mana cost")
        ),
        "off the battlefield the card is its printed front again"
    );
}
