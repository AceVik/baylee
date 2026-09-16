//! The ways a card can be cast, when there is more than one.
//!
//! Suspend, evoke, a free alternative cost — the engine offers them all and
//! the client has to ask which, unless only one is reachable. What closes the
//! chooser and what forgets the answer belong here too.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Suspending, end to end, the way the owner asked for it.
///
/// Reported as: *"Suspend works different. The text says: Rather then cast
/// this spell from your hand, PAY x and exile …, so first tap and pay mana,
/// then suspend."* Which is what the card says, and neither half of this
/// client did it: the engine offered `suspendable` off an empty pool (fixed
/// on its side), and nothing here ever built a `PlayerAction::Suspend` at
/// all — `legal.suspendable` was read by the automation rules and by nothing
/// else, so a Suspend 4—{U} in hand answered no click.
///
/// Driven through `activate_card` and `advance_mana_run` against a real
/// `LocalHost`, because what is claimed is about a *click*: a test that built
/// the action by hand would pass just as loudly with no button behind it.
#[test]
fn a_suspend_card_taps_its_island_and_then_suspends() {
    use baylee_client::input::activate_card;
    use baylee_client::{Deed, Duel, RunEnd, advance_mana_run};
    use baylee_client_core::interaction::Interaction;

    let mut table = Table::open_with(&suspend_preset());
    table.walk_to_main();

    let vision = table
        .view()
        .hand
        .iter()
        .find(|c| c.name == "Ancestral Vision")
        .expect("the card is in the opening hand")
        .id;

    // The engine offers neither half of it yet, which is the whole problem:
    // the {U} is still in the Island. And it is not castable at any point —
    // a card with no mana cost cannot be cast (CR 202.1a).
    assert!(
        !table.legal().suspendable.contains(&vision),
        "the cost is not floating, so the engine offers no suspend"
    );
    assert!(
        !table.legal().castable.contains(&vision),
        "and a blank mana cost is never a free spell"
    );

    let mut duel = Duel::default();
    let refresh = |duel: &mut Duel, table: &Table| {
        duel.view = Some(table.view().clone());
        duel.interaction = Some(Interaction::new(
            table.pending.clone().expect("priority"),
            PlayerId::new(0),
        ));
        baylee_client::rebuild_board(duel);
    };
    refresh(&mut duel, &table);
    assert!(
        duel.suspend_reach.contains(&vision),
        "one untapped Island pays {{U}}, so the client offers to tap it"
    );

    // And the drawn hand says so, which is the half a player can actually
    // see. A suspend-only card used to sit there with no light on it at all:
    // `Openings` was built from `lands` and `castable` alone, so neither the
    // engine's own `suspendable` nor this client's `suspend_reach` reached
    // the board model. Clicking it worked — finding it did not.
    let drawn = |duel: &Duel| {
        duel.board
            .as_ref()
            .expect("a view builds a board")
            .hand
            .iter()
            .find(|c| c.id == vision)
            .map(|c| (c.playable, c.reachable))
            .expect("the card is in the drawn hand")
    };
    assert_eq!(
        drawn(&duel),
        (false, true),
        "with the {{U}} still in the Island: indigo, and not gold"
    );

    // First click arms, second click sends. Nothing reaches the wire in
    // between — suspending exiles the card and there is no undo.
    activate_card(&mut duel, vision);
    let Some(Deed::Run { then, .. }) = duel.armed.as_ref().map(|a| a.deed.clone()) else {
        panic!("the click arms a run: {:?}", duel.armed);
    };
    assert_eq!(then, RunEnd::Suspend, "and the run ends in a suspend");
    assert!(duel.outbox().is_empty(), "arming puts nothing on the wire");

    activate_card(&mut duel, vision);
    for action in duel.take_outbox() {
        table.submit(action);
    }
    assert!(duel.mana_run.is_some(), "the second click starts the run");

    // Now spend it, one engine round trip per step, exactly as the frame
    // loop does.
    let mut gold_once_the_mana_was_up = false;
    for _ in 0..8 {
        refresh(&mut duel, &table);
        // The other half of the light: the moment the engine itself offers
        // the suspend, the card is gold rather than indigo — the same
        // promotion a castable spell gets when its mana floats.
        if table.legal().suspendable.contains(&vision) {
            gold_once_the_mana_was_up = drawn(&duel).0;
        }
        advance_mana_run(&mut duel);
        let sent = duel.take_outbox();
        if sent.is_empty() {
            break;
        }
        for action in sent {
            table.submit(action);
        }
        if duel.mana_run.is_none() {
            break;
        }
    }
    assert_eq!(duel.last_error, None, "the run finished without aborting");
    assert!(
        gold_once_the_mana_was_up,
        "the engine's own offer lights the card gold"
    );

    // The card is in exile with four time counters on it, and the Island is
    // tapped. That is what "pay {U} and exile it" means.
    let exiled = table
        .view()
        .exile
        .iter()
        .flatten()
        .find(|o| o.name == "Ancestral Vision")
        .expect("the suspended card is in exile");
    assert_eq!(
        exiled
            .counters
            .iter()
            .find(|c| c.kind == baylee_view::CounterKind::Time)
            .map(|c| c.count),
        Some(4),
        "Suspend 4 exiles it with four time counters: {:?}",
        exiled.counters
    );
}

/// The click path resolves a cast before a suspend, and nothing in the pool
/// is both.
///
/// `activate_card` reads `play_card` first and `suspend` after it, which is
/// an *order* and would be a silent choice on a card that offered both — and
/// there is no undo for either. The pool makes the question moot: a suspend
/// card prints no mana cost, so CR 202.1a keeps it out of `castable`
/// altogether. This is that claim as a build failure rather than a comment,
/// because the first card that breaks it needs a chooser and not an order.
#[test]
fn no_suspend_card_in_the_pool_is_also_castable() {
    let mut both: Vec<&str> = Vec::new();
    for def in baylee_cards::all() {
        let suspends = def
            .abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Suspend { .. }));
        if !suspends {
            continue;
        }
        for face in def.faces {
            if face.mana_cost.symbols().next().is_some() {
                both.push(face.name);
            }
        }
    }
    assert!(
        both.is_empty(),
        "these cards can be cast *and* suspended, so one click has two \
         answers and `activate_card` picks the first silently: {both:?}"
    );
}

/// The measurement in the report, answered.
///
/// Eight Plains, both of Reveillark's ways payable, and the player asked. It
/// used to go straight onto the stack for the printed `{4}{W}` with three
/// lands still untapped and no question asked at all.
#[test]
fn reveillark_asks_which_way_and_evokes_when_it_is_told_to() {
    use baylee_client::Duel;
    use baylee_client::input::{activate_card, pick_choice};
    use baylee_engine::choice::CastModeKind;

    let mut table = Table::open_with(&white_preset(&[REVEILLARK], 8));
    table.walk_to_main();
    let lark = id_in_hand(&table, "Reveillark");

    let mut duel = Duel::default();
    refresh(&mut duel, &table);
    assert!(
        !table.legal().castable.contains(&lark),
        "the mana is in the lands, so the engine offers nothing yet"
    );
    assert!(
        duel.reachable.contains(&lark),
        "and this client offers to go and get it"
    );

    // The click opens the chooser. Nothing is armed and nothing is on the
    // wire: the question comes before the first tap, which is the whole
    // repair.
    activate_card(&mut duel, lark);
    let ways = duel
        .cast_menu
        .as_ref()
        .map(|m| m.modes.iter().map(|w| w.kind).collect::<Vec<_>>())
        .expect("two ways, so a chooser");
    assert_eq!(
        ways,
        vec![CastModeKind::Normal, CastModeKind::Alternative(0)],
        "printed {{4}}{{W}} and evoke {{5}}{{W}}"
    );
    assert!(duel.armed.is_none(), "a chooser is not an armed deed");
    assert!(
        duel.outbox().is_empty(),
        "and it says nothing to the engine"
    );

    // The evoke row. It arms, the way the ability sheet's rows do, because a
    // spell on the stack is the least undoable thing in the game.
    pick_choice(&mut duel, 1);
    assert!(
        duel.cast_menu.is_none(),
        "the chooser is answered and closes"
    );
    assert_eq!(
        duel.cast_answer,
        Some((lark, CastModeKind::Alternative(0))),
        "and the way is remembered as a kind, not as an index"
    );
    assert!(
        duel.armed.is_some(),
        "with the deed waiting for its second tap"
    );
    assert!(duel.outbox().is_empty());

    // Second tap sends it: six lands tapped for {5}{W}, then the cast, then
    // the engine's own question — answered with the evoke.
    activate_card(&mut duel, lark);
    for action in duel.take_outbox() {
        table.submit(action);
    }
    assert!(duel.mana_run.is_some(), "the second tap starts the run");
    assert_eq!(
        play_it_out(&mut duel, &mut table),
        1,
        "the engine asked which way exactly once"
    );

    assert_eq!(
        untapped_lands(&table),
        2,
        "evoke is {{5}}{{W}}: six of the eight Plains are spent"
    );
    assert!(
        table.view().stack.iter().any(|o| o.name == "Reveillark"),
        "and the spell is on the stack: {:?}",
        table
            .view()
            .stack
            .iter()
            .map(|o| &o.name)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        duel.cast_answer, None,
        "the answer was spent, not left over"
    );
}

/// The other direction, and the one the click could not reach at all.
///
/// Solitude's evoke is free, so the engine has it in `castable` from the
/// first frame and the click cast it that way in silence. Five Plains make
/// the printed `{3}{W}{W}` reachable too — and picking it has to beat the
/// engine's own offer, which is exactly what `fire_armed` had to learn.
#[test]
fn solitude_can_be_hard_cast_even_though_the_engine_offers_the_free_way() {
    use baylee_client::Duel;
    use baylee_client::input::{activate_card, pick_choice};
    use baylee_engine::choice::CastModeKind;

    // Reveillark is in the hand as the white card the evoke would exile —
    // without one the pitch is unpayable and there is only one way.
    let mut table = Table::open_with(&white_preset(&[SOLITUDE, REVEILLARK], 5));
    table.walk_to_main();
    let solitude = id_in_hand(&table, "Solitude");

    let mut duel = Duel::default();
    refresh(&mut duel, &table);
    assert!(
        table.legal().castable.contains(&solitude),
        "the free evoke is payable with an empty pool, so the engine offers it"
    );

    activate_card(&mut duel, solitude);
    let ways = duel
        .cast_menu
        .as_ref()
        .map(|m| m.modes.iter().map(|w| w.kind).collect::<Vec<_>>())
        .expect("free or printed, so a chooser");
    assert_eq!(
        ways,
        vec![CastModeKind::Normal, CastModeKind::Alternative(0)]
    );

    // The printed cost, which is the row that was unreachable.
    pick_choice(&mut duel, 0);
    assert_eq!(duel.cast_answer, Some((solitude, CastModeKind::Normal)));

    activate_card(&mut duel, solitude);
    assert!(
        duel.outbox().is_empty(),
        "the engine's own offer must not short-circuit a chosen way"
    );
    assert!(duel.mana_run.is_some(), "five lands to tap first");
    assert_eq!(play_it_out(&mut duel, &mut table), 1);

    assert_eq!(untapped_lands(&table), 0, "{{3}}{{W}}{{W}} spends all five");
    assert!(
        table.view().stack.iter().any(|o| o.name == "Solitude"),
        "and Solitude is on the stack, paid for the printed way"
    );
    assert!(
        table.view().hand.iter().any(|c| c.name == "Reveillark"),
        "with the white card still in hand, which is the whole reason to hard cast it"
    );
}

/// One way is no question. The click keeps the behaviour it always had — two
/// presses, not three — and that matters as much as the chooser does.
///
/// It is the pitch filter read live as well: Solitude alone in a hand has
/// nothing white to exile but itself, and the card being cast is never a
/// candidate for its own pitch. So the free way is not on offer at all, the
/// printed one is, and one way is not a question.
#[test]
fn a_card_with_one_reachable_way_opens_no_chooser() {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;

    let mut table = Table::open_with(&white_preset(&[SOLITUDE], 5));
    table.walk_to_main();
    let solitude = id_in_hand(&table, "Solitude");

    let mut duel = Duel::default();
    refresh(&mut duel, &table);
    activate_card(&mut duel, solitude);
    assert!(
        duel.cast_menu.is_none(),
        "one way is not a question: {:?}",
        duel.cast_menu.as_ref().map(|m| m.modes.len())
    );
    assert!(
        duel.armed.is_some(),
        "the click arms the deed it always armed"
    );
}

/// A tap on **another** card puts the chooser away.
///
/// The trace it was written for: the chooser stands for Solitude and the next
/// click is Reveillark, which with five Plains has exactly one way and so
/// opens no chooser of its own. Every branch under that one used to leave the
/// old question standing — a headline asking about Solitude over an armed row
/// offering Reveillark, and a row press that would then arm the card the
/// player had stopped looking at.
#[test]
fn a_tap_on_another_card_puts_the_cast_chooser_away() {
    use baylee_client::Duel;
    use baylee_client::input::activate_card;

    let mut table = Table::open_with(&white_preset(&[SOLITUDE, REVEILLARK], 5));
    table.walk_to_main();
    let solitude = id_in_hand(&table, "Solitude");
    let lark = id_in_hand(&table, "Reveillark");

    let mut duel = Duel::default();
    refresh(&mut duel, &table);

    activate_card(&mut duel, solitude);
    assert_eq!(
        duel.cast_menu.as_ref().map(|m| m.card),
        Some(solitude),
        "five Plains and a white card in hand is two ways to cast Solitude"
    );

    activate_card(&mut duel, lark);
    assert!(
        duel.cast_menu.is_none(),
        "the question belonged to the card the player has left"
    );
    assert_eq!(
        duel.armed.as_ref().map(|a| a.object),
        Some(lark),
        "and the click still arms what it clicked"
    );
}

/// Taking the deed back forgets the way that was chosen for it.
///
/// The two are one press (`take_cast_row` answers and arms together), so an
/// `Esc` that kept the answer would take back the taps and keep the choice —
/// and the answer is spent by the *engine's* `ChooseCastMode`, which is still
/// reachable by another route: Solitude's free evoke needs no run at all.
#[test]
fn taking_the_deed_back_forgets_the_way_that_was_chosen() {
    use baylee_client::Duel;
    use baylee_client::input::{activate_card, disarm, pick_choice};

    let mut table = Table::open_with(&white_preset(&[SOLITUDE, REVEILLARK], 5));
    table.walk_to_main();
    let solitude = id_in_hand(&table, "Solitude");

    let mut duel = Duel::default();
    refresh(&mut duel, &table);

    activate_card(&mut duel, solitude);
    pick_choice(&mut duel, 0);
    assert!(duel.armed.is_some(), "a row arms rather than sending");
    assert!(
        duel.cast_answer.is_some_and(|(card, _)| card == solitude),
        "and the way it picked is remembered"
    );

    disarm(&mut duel);
    assert!(duel.armed.is_none());
    assert!(
        duel.cast_answer.is_none(),
        "Esc takes back the whole of what was said, which way included"
    );
}
