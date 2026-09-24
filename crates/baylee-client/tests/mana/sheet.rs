//! The ability sheet, read over the **whole pool**.
//!
//! [`super::chooser`] and [`super::arming`] pin the sheet on the handful of
//! cards whose shapes were argued about — a Forest, a painland, a
//! planeswalker, a Lantern. This is the other half: every card in the pool
//! that offers something on the battlefield, seated in a real engine, read
//! through the same `abilities::options` the renderer calls, and held against
//! the rules the sheet is built out of.
//!
//! A rule wrote the sheet, so a rule is what a sweep can hold. One card
//! disagreeing is a card; a hundred disagreeing the same way is the rule.
//!
//! **The population is bounded on both sides and printed**, for the reason
//! `xtask cross-read` carries a floor: a reader that walks a pool and reaches
//! nothing reports "no disagreements", which is the most convincing wrong
//! answer there is. [`FLOOR`] is what makes an empty sweep fail instead.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

use baylee_cards_dsl::{AbilityDef, ActivationZone, CardDef, Coverage};
use baylee_client::abilities::{self, Split};
use baylee_client_core::Lang;
use baylee_client_core::abilitysheet;
use baylee_client_core::i18n::Phrase;
use baylee_client_core::interaction::Interaction;
use baylee_client_core::manaplan::Tap;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::{LegalActions, default_arrangement};

/// How many of the pool's permanents share one game.
///
/// One engine per card would be honest and slow: the sweep's cost is almost
/// entirely the opening of a game, and a battlefield of ten permanents asks
/// the same questions ten times for one opening. Cards do see each other —
/// an anthem over there changes a creature here — and that is a feature
/// rather than a hazard: the sheet is supposed to read the board it is on.
const BATCH: usize = 10;

/// The fewest permanents a sweep may reach before it is not a sweep.
///
/// Measured rather than chosen: the count is printed on every run, and this
/// stands well under it so that ordinary additions to the pool never trip it
/// while a reader gone blind — a filter that matches nothing, a walk that
/// finds no main phase — does. Measured on 17.09.2026: 631 candidates, 628
/// of them seated and read, 602 with something to offer.
const FLOOR: usize = 550;

/// The fewest permanents that must open a **sheet** — two rows or more.
///
/// A second floor and not the same one: most of the pool offers one thing and
/// is answered by a click, so a sweep of the sheet's own keys, cursor and
/// pager is about the 430 permanents that have a list at all. Both numbers
/// are printed on every run.
const SHEET_FLOOR: usize = 350;

/// The five basics, three apiece, so an ability with a mana cost is offered
/// at all.
///
/// The engine lists an activated ability only where its cost is payable out
/// of the **floating** pool (`Engine::can_afford`), which is the correct
/// rules answer and means a sweep with no mana in it would see only the free
/// and tap-only half of the pool. Fifteen mana, three of every colour, pays
/// for all but a handful of the pool's activation costs.
const BASICS: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

/// How many of each.
const BASICS_EACH: usize = 3;

fn deck_entry(index: baylee_core::ids::CardIndex) -> DeckEntry {
    DeckEntry {
        card: index,
        print: PrintRef::new(0),
    }
}

fn by_name(name: &str) -> baylee_core::ids::CardIndex {
    baylee_cards::decks::by_name(name).expect("a card of that name in the pool")
}

/// Whether the engine could ever list one of this card's abilities while it
/// stands on the battlefield.
///
/// Read off the DSL rather than guessed from the type line: a battlefield
/// zone on an activated ability, or a loyalty ability, which has no zone
/// because a planeswalker has nowhere else to be.
fn offers_on_the_battlefield(def: &CardDef) -> bool {
    def.abilities_for_face(0)
        .iter()
        .any(|ability| match ability {
            AbilityDef::Activated { zone, .. } | AbilityDef::ActivatedConditional { zone, .. } => {
                *zone == ActivationZone::Battlefield
            }
            AbilityDef::Loyalty { .. } => true,
            _ => false,
        })
}

/// Every card worth seating, in pool order.
///
/// `Unimplemented` is left out because a stub has no abilities to offer and
/// would only buy openings; `Partial` is kept, because a partial card is one
/// a player can put in a deck and its sheet is drawn from whatever it does
/// carry.
fn candidates() -> Vec<&'static CardDef> {
    baylee_cards::all()
        .filter(|def| !matches!(def.coverage, Coverage::Unimplemented))
        .filter(|def| offers_on_the_battlefield(def))
        .collect()
}

/// A duel whose seat 0 opens with fifteen basics and `cards` beside them.
fn sweep_preset(cards: &[&'static CardDef]) -> GamePreset {
    let forest = deck_entry(by_name("Forest"));
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![forest; 60],
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    let mut preset = GamePreset {
        format: FormatId::Freeform,
        seed: 7,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    };
    let mut board: Vec<DeckEntry> = Vec::new();
    for name in BASICS {
        for _ in 0..BASICS_EACH {
            board.push(deck_entry(by_name(name)));
        }
    }
    board.extend(cards.iter().map(|def| deck_entry(def.index)));
    preset.seats[0].starting_battlefield = board;
    preset
}

/// A batch, seated and asked: the table, the seat's own view, and an
/// `Interaction` holding the priority the engine is offering it.
///
/// `None` where the walk did not reach our own main phase, which a batch is
/// allowed to do — a card whose enter trigger asks a question this harness
/// answers differently from the engine's expectation leaves the table
/// somewhere else. Every sweep counts what it skipped and every sweep carries
/// a floor, so a preamble that started failing would fail the floor rather
/// than pass quietly.
///
/// One function because it is one preamble, and four copies of it are four
/// places for the seat number, the mana and the `Pending` to drift apart.
fn seated(batch: &[&'static CardDef]) -> Option<Seated> {
    let mut table = Table::open_with(&sweep_preset(batch));
    if !walk_to_our_main(&mut table) {
        return None;
    }
    float_the_basics(&mut table);
    let Some(Pending::Priority { legal, player }) = table.pending.clone() else {
        return None;
    };
    if player != PlayerId::new(0) {
        return None;
    }
    let pending = Pending::Priority {
        player,
        legal: legal.clone(),
    };
    Some(Seated {
        view: table.view().clone(),
        interaction: Interaction::new(pending.clone(), PlayerId::new(0)),
        pending,
        legal: *legal,
    })
}

/// What [`seated`] hands back.
///
/// A struct and not a tuple because two of the four sweeps want the
/// `Pending` as well — they build a fresh [`baylee_client::Duel`] per row —
/// and a four-tuple with two ignored slots reads as though something had been
/// forgotten.
struct Seated {
    /// Seat 0's whole view.
    view: PlayerView,
    /// The priority it is holding, for rebuilding a `Duel` per row.
    pending: Pending,
    /// That same priority, as the client's own state machine.
    interaction: Interaction,
    /// What the engine is offering, for a sweep that counts offers rather
    /// than reading options.
    legal: LegalActions,
}

/// Walks a freshly opened table to this seat's own first main phase,
/// answering **everything** on the way.
///
/// [`Table::walk_to_main`] answers the four questions a Forest-and-a-creature
/// table asks and returns on anything else, which is right for the tests it
/// was written for and loses a fifth of this pool: a board dealt fifteen
/// basics and ten strangers asks about a shockland's life, an untap step's
/// storage lands, a Cavern's creature type and a Birds of Paradise's colour
/// before it ever reaches a priority. Those batches were skipped whole — 132
/// of 631 candidates, and not a random 132, because the lands that ask
/// questions are exactly the lands with abilities to draw.
///
/// The answers are deliberately the *dullest* legal one: no life paid, no
/// card chosen beyond the minimum, the first colour offered. A sweep is about
/// the sheet and not about the board, and a board that took an interesting
/// branch would be a board nobody could reproduce.
fn walk_to_our_main(table: &mut Table) -> bool {
    for _ in 0..400 {
        let Some(pending) = table.pending.clone() else {
            return false;
        };
        let us = PlayerId::new(0);
        let action = match pending {
            Pending::Priority { player, .. } if player == us => {
                if table.view().phase == baylee_view::Phase::FirstMain && table.view().active == us
                {
                    return true;
                }
                PlayerAction::PassPriority
            }
            Pending::Mulligan { player, .. } if player == us => PlayerAction::MulliganKeep,
            Pending::MulliganBottom { player, count, .. } if player == us => {
                let hand: Vec<ObjectId> = table
                    .view()
                    .hand
                    .iter()
                    .take(count as usize)
                    .map(|c| c.id)
                    .collect();
                PlayerAction::ChooseObjects { objects: hand }
            }
            Pending::ChooseAttackers { player, .. } if player == us => {
                PlayerAction::DeclareAttackers { attackers: vec![] }
            }
            Pending::ChooseBlockers { player, .. } if player == us => {
                PlayerAction::DeclareBlockers { blockers: vec![] }
            }
            Pending::DiscardChoice { player, count } if player == us => {
                let hand: Vec<ObjectId> = table
                    .view()
                    .hand
                    .iter()
                    .take(count as usize)
                    .map(|c| c.id)
                    .collect();
                PlayerAction::ChooseObjects { objects: hand }
            }
            Pending::LegendChoice { player, options } if player == us => {
                PlayerAction::ChooseObjects {
                    objects: options.into_iter().take(1).collect(),
                }
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                ..
            } if player == us => PlayerAction::ChooseObjects {
                objects: options.into_iter().take(min as usize).collect(),
            },
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                ..
            } if player == us => {
                let want = min as usize;
                let objects: Vec<ObjectId> = options.iter().copied().take(want).collect();
                let players: Vec<PlayerId> = player_options
                    .iter()
                    .copied()
                    .take(want.saturating_sub(objects.len()))
                    .collect();
                PlayerAction::ChooseTargets { objects, players }
            }
            Pending::ChooseSubtype { player, options } if player == us => match options.first() {
                Some(first) => PlayerAction::ChooseSubtype(*first),
                None => return false,
            },
            Pending::ChooseColor { player, options } if player == us => match options.first() {
                Some(first) => PlayerAction::ChooseColor(*first),
                None => return false,
            },
            Pending::YesNo { player, .. } if player == us => PlayerAction::YesNo(false),
            Pending::ChooseCastMode { player, .. } if player == us => PlayerAction::ChooseMode(0),
            Pending::ChooseNumber { player, min, .. } if player == us => {
                PlayerAction::ChooseNumber(min)
            }
            Pending::ChoosePlayer { player, options } if player == us => match options.first() {
                Some(first) => PlayerAction::ChoosePlayer(*first),
                None => return false,
            },
            Pending::Arrange {
                player,
                cards,
                piles,
                ..
            } if player == us => match default_arrangement(&cards, &piles) {
                Some(piles) => PlayerAction::Arrange { piles },
                None => return false,
            },
            _ => return false,
        };
        table.submit(action);
    }
    false
}

/// Taps every basic on the table, so the pool is full when the sheet is read.
fn float_the_basics(table: &mut Table) {
    for _ in 0..(BASICS.len() * BASICS_EACH) {
        let Some(Pending::Priority { legal, .. }) = table.pending.clone() else {
            return;
        };
        let next = table
            .view()
            .battlefield
            .iter()
            .find(|o| BASICS.contains(&o.name.as_str()) && legal.mana_abilities.contains(&o.id))
            .map(|o| o.id);
        let Some(id) = next else { return };
        table.submit(PlayerAction::ActivateManaAbility { source: id });
    }
}

/// One permanent's sheet, held against every rule the sheet is built out of.
///
/// Findings are collected rather than asserted one at a time: a sweep that
/// stops at the first disagreement reports one card and hides the shape, and
/// the shape is the whole reason to sweep.
fn audit(
    view: &PlayerView,
    interaction: &Interaction,
    legal: &LegalActions,
    id: ObjectId,
    name: &str,
    found: &mut Vec<String>,
) {
    let options = abilities::options(Lang::En, view, interaction, id);
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == id)
        .map(|(_, index)| *index)
        .collect();
    let any_offer = !offered.is_empty() || legal.mana_abilities.contains(&id);

    // 1. A permanent the engine is offering something must show something.
    if any_offer && options.is_empty() {
        found.push(format!(
            "{name}: the engine offers {offered:?} and the sheet lists nothing"
        ));
        return;
    }
    if !any_offer {
        return;
    }

    let split = Split::of(&options);

    // 2. The pips are a prefix, which is the whole of `Split`'s arithmetic:
    // a pour behind a written row would put a keycap on a pip.
    for (at, option) in options.iter().enumerate() {
        if (at < split.pips) != option.pour.is_some() {
            found.push(format!(
                "{name}: row {at} of {} is on the wrong side of the split ({split:?})",
                options.len()
            ));
        }
    }

    // 3. One pip per colour. A colour offered by two taps is one press.
    let mut seen: Vec<baylee_core::mana::ManaColor> = Vec::new();
    for option in options.iter().filter_map(|o| o.pour) {
        if seen.contains(&option.color) {
            found.push(format!("{name}: two pips pour {:?}", option.color));
        }
        seen.push(option.color);
    }

    // 4. No two rows do the same thing. A pip is exempt on its action alone —
    // five colours off one tap are five presses of one activation — so the
    // pair compared is what the press actually decides.
    for (i, a) in options.iter().enumerate() {
        for b in options.iter().skip(i + 1) {
            if a.action == b.action && a.pour.map(|p| p.color) == b.pour.map(|p| p.color) {
                found.push(format!("{name}: two rows send {:?}", a.action));
            }
        }
    }

    audit_what_the_rows_say(view, &options, split, id, name, offered.as_slice(), found);
}

/// The second half of [`audit`]: what each row of the list *is*, rather than
/// how the list is shaped.
///
/// Split at the seam the checks already had — one to four are about the list
/// and five to seven about its rows — because the two halves share only the
/// list they walk, and one function reading a hundred lines of numbered
/// paragraphs is a function nobody reads to the end.
fn audit_what_the_rows_say(
    view: &PlayerView,
    options: &[abilities::AbilityOption],
    split: Split,
    id: ObjectId,
    name: &str,
    offered: &[u32],
    found: &mut Vec<String>,
) {
    // 5. Every offered ability that is not mana has a row of its own. A mana
    // ability may be folded — into a pip, or into the one tap `manasources`
    // reduced the permanent to — and the fold is checked as the weaker claim
    // below, because which tap won is `manasources`'s judgement and not the
    // sheet's.
    for index in offered {
        let named = options.iter().any(|option| {
            matches!(
                &option.action,
                PlayerAction::ActivateAbility { source, ability_index }
                    if *source == id && ability_index == index
            )
        });
        let pipped = options
            .iter()
            .any(|o| o.pour.is_some_and(|p| p.step.tap == Tap::Ability(*index)));
        if named || pipped {
            continue;
        }
        if makes_mana_in_the_registry(view, id, *index) {
            if !options.iter().any(|o| o.mana) {
                found.push(format!(
                    "{name}: mana ability {index} is folded away and no mana row is left"
                ));
            }
            continue;
        }
        found.push(format!(
            "{name}: the engine offers ability {index} and no row of the sheet sends it"
        ));
    }

    // 6. Every written row carries a digit, and the digits reach every row.
    let (_head, counted) = split.numbered();
    let mut reached = vec![0usize; counted];
    for page in 0..abilitysheet::pages(counted) {
        for digit in "123456789".chars() {
            if let Some(at) = abilitysheet::option_of(counted, page, digit) {
                if at < counted {
                    reached[at] += 1;
                } else {
                    found.push(format!(
                        "{name}: page {page} digit {digit} names row {at} of {counted}"
                    ));
                }
            }
        }
    }
    for (at, times) in reached.iter().enumerate() {
        if *times != 1 {
            found.push(format!(
                "{name}: row {at} of {counted} is named by {times} digits"
            ));
        }
    }

    // 7. A row a player cannot read. Two halves, and they are the two ways a
    // row can end up wordless.
    //
    // A row the card *does* print a sentence for draws that sentence and
    // nothing else, so `Ability 4` — the label this whole sheet was built to
    // stop drawing — must never be what it settles for.
    //
    // A row the card prints **nothing** for has only this client's own name
    // for it, which `AbilityOption::printed_index` is the one door to, and
    // that name had better not be empty: a granted ability with no words is a
    // keycap beside a cost, and a player has no way at all to learn what it
    // does.
    for (at, option) in options.iter().enumerate() {
        if option.pour.is_some() {
            continue;
        }
        match option.printed_index() {
            Some(index)
                if option.printed.is_none()
                    && option.label
                        == Phrase::AbilityNumbered.fill(Lang::En, &[&(index + 1).to_string()]) =>
            {
                found.push(format!(
                    "{name}: row {at} is \"{}\" and carries no printed sentence",
                    option.label
                ));
            }
            None if option.label.trim().is_empty() => {
                found.push(format!(
                    "{name}: row {at} is printed on no card and this client has no name for it"
                ));
            }
            _ => {}
        }
    }
}

/// Whether the registry says this ability of this object makes mana.
fn makes_mana_in_the_registry(view: &PlayerView, id: ObjectId, index: u32) -> bool {
    matches!(
        baylee_client::manasources::ability_at(view, id, index),
        Some(
            AbilityDef::Activated {
                mana_ability: true,
                ..
            } | AbilityDef::ActivatedConditional {
                mana_ability: true,
                ..
            }
        )
    )
}

/// The sweep.
#[test]
fn every_permanent_in_the_pool_draws_a_sheet_that_obeys_its_own_rules() {
    let cards = candidates();
    let mut found: Vec<String> = Vec::new();
    let mut reached = 0usize;
    let mut offering = 0usize;
    let mut skipped: Vec<&str> = Vec::new();

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view,
            interaction,
            legal,
            ..
        }) = seated(batch)
        else {
            skipped.extend(batch.iter().map(|d| d.name()));
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                skipped.push(def.name());
                continue;
            };
            reached += 1;
            let id = object.id;
            if legal.abilities.iter().any(|(s, _)| *s == id) || legal.mana_abilities.contains(&id) {
                offering += 1;
            }
            audit(&view, &interaction, &legal, id, def.name(), &mut found);
        }
    }

    println!(
        "sheet sweep: {} candidates, {reached} seated, {offering} offering, {} skipped, {} findings",
        cards.len(),
        skipped.len(),
        found.len()
    );
    assert!(
        reached >= FLOOR,
        "the sweep reached {reached} permanents, under its floor of {FLOOR} — \
         a reader that finds nothing reports no disagreements"
    );
    assert!(
        found.is_empty(),
        "{} sheets disagree with the rules they are built from:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// Whether a press reached something the engine or the table can hear.
///
/// Four outcomes, because the sheet has four kinds of row and the owner's
/// rules give each of them a different one: a send puts an action in the
/// outbox, an arm-then-act row holds a deed, a pip opens a one-step mana run,
/// and a written mana row a pip could not stand for *steps* into that tap and
/// turns the sheet into its bubble.
fn the_press_landed(duel: &baylee_client::Duel, before_tap: Option<u32>) -> bool {
    !duel.outbox().is_empty()
        || duel.armed.is_some()
        || duel.mana_run.is_some()
        || (duel.ability_tap != before_tap && duel.ability_tap.is_some())
}

/// A fresh client holding this view and this question, with nothing armed.
///
/// The question is rebuilt from its `Pending` rather than cloned, because an
/// `Interaction` is deliberately not `Clone`: it is a seat's answer in
/// progress, and a copy of one is two seats answering.
fn duel_at(view: &PlayerView, pending: &Pending) -> baylee_client::Duel {
    let mut duel = baylee_client::Duel::default();
    duel.view = Some(view.clone());
    duel.interaction = Some(Interaction::new(pending.clone(), PlayerId::new(0)));
    duel
}

/// Every row the sheet draws answers the digit drawn on it.
///
/// The sweep above reads the *list*; this one presses it. They are different
/// questions and the second is the one a player asks: a row that is built
/// correctly, numbered correctly and drawn correctly is still a defect if the
/// key beside it does nothing — and a hand-built `PlayerAction` would never
/// have shown that, which is the whole of
/// `client-tests-must-answer-like-a-player`.
///
/// Every press starts from a fresh client, so a row is never answered by
/// state the row before it left behind.
#[test]
fn every_row_a_sheet_draws_answers_the_digit_drawn_on_it() {
    let cards = candidates();
    let mut found: Vec<String> = Vec::new();
    let mut sheets = 0usize;
    let mut presses = 0usize;

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view,
            pending,
            interaction,
            ..
        }) = seated(batch)
        else {
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                continue;
            };
            let id = object.id;
            let name = def.name();
            let options = abilities::options(Lang::En, &view, &interaction, id);
            if options.len() < 2 {
                continue;
            }
            sheets += 1;

            // The click that opens it. A permanent with a list opens a sheet
            // and sends nothing — the list is the question, not the answer.
            let mut opened = duel_at(&view, &pending);
            baylee_client::input::activate_card(&mut opened, id);
            if opened.ability_menu != Some(id) {
                found.push(format!(
                    "{name}: {} options and the click opened no sheet",
                    options.len()
                ));
                continue;
            }
            if !opened.outbox().is_empty() {
                found.push(format!("{name}: opening the sheet already sent something"));
            }

            let (_, counted) = Split::of(&options).numbered();
            for at in 0..counted {
                let page = at / abilitysheet::PAGE;
                let digit =
                    char::from_digit(u32::try_from(at % abilitysheet::PAGE).unwrap_or(0) + 1, 10)
                        .expect("a digit for a row of a page");
                let mut duel = duel_at(&view, &pending);
                baylee_client::input::activate_card(&mut duel, id);
                duel.ability_page = page;
                let before_tap = duel.ability_tap;
                presses += 1;
                if !baylee_client::input::sheet_digit(&mut duel, digit) {
                    found.push(format!(
                        "{name}: row {at} is drawn on page {page} key {digit} and the key is refused"
                    ));
                    continue;
                }
                if the_press_landed(&duel, before_tap) {
                    continue;
                }
                // Not sent and not armed: the only honest remaining state is
                // a second press, which is what an arm-then-act row asks for.
                let before_tap = duel.ability_tap;
                baylee_client::input::sheet_digit(&mut duel, digit);
                if !the_press_landed(&duel, before_tap) {
                    found.push(format!(
                        "{name}: row {at} (page {page}, key {digit}) does nothing, twice"
                    ));
                }
            }

            // The pager, where there is one, turns the page and says so.
            if abilitysheet::paged(counted) {
                let mut duel = duel_at(&view, &pending);
                baylee_client::input::activate_card(&mut duel, id);
                let before = duel.ability_page;
                if !baylee_client::input::sheet_digit(&mut duel, abilitysheet::PAGER) {
                    found.push(format!(
                        "{name}: {counted} rows over {} pages and the pager is refused",
                        abilitysheet::pages(counted)
                    ));
                } else if duel.ability_page == before {
                    found.push(format!("{name}: the pager turned no page"));
                }
            }
        }
    }

    println!(
        "sheet presses: {sheets} sheets, {presses} rows pressed, {} findings",
        found.len()
    );
    assert!(
        sheets >= SHEET_FLOOR,
        "only {sheets} permanents in the pool opened a sheet at all, \
         under the floor of {SHEET_FLOOR}"
    );
    assert!(
        found.is_empty(),
        "{} rows do not answer their own key:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The highlighted row is always a row that is drawn.
///
/// The cursor walks the whole list — pips and written rows are one column to
/// `W`/`S` — while the sheet draws nine rows of it at a time, so the page has
/// to follow the cursor. `ability_menu_keys` says it does; this is the pool
/// asked whether it does, over every permanent that opens a sheet at all.
///
/// Also the round trip: `len` steps down is where it started, and every index
/// on the way exactly once. A cursor that skips a row is a row that can be
/// reached by its digit and by nothing else.
#[test]
fn the_cursor_never_stands_on_a_row_the_page_does_not_draw() {
    use baylee_client::keys::Fired;
    use baylee_client_core::prefs::Action;

    let cards = candidates();
    let mut found: Vec<String> = Vec::new();
    let mut walked = 0usize;

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view,
            pending,
            interaction,
            ..
        }) = seated(batch)
        else {
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                continue;
            };
            let id = object.id;
            let name = def.name();
            let options = abilities::options(Lang::En, &view, &interaction, id);
            if options.len() < 2 {
                continue;
            }
            walked += 1;
            let split = Split::of(&options);
            let (head, counted) = split.numbered();

            let mut duel = duel_at(&view, &pending);
            baylee_client::input::activate_card(&mut duel, id);
            let mut seen = vec![0usize; options.len()];
            for _ in 0..options.len() {
                seen[duel.ability_pick.min(options.len() - 1)] += 1;
                // The page has to be showing whatever the cursor is on. A pip
                // is on every page and needs no check; a written row is on
                // exactly one.
                if duel.ability_pick >= split.pips || split.rows == 0 {
                    let at = duel.ability_pick - head;
                    let drawn = abilitysheet::rows(counted, duel.ability_page);
                    if !drawn.contains(&at) {
                        found.push(format!(
                            "{name}: the cursor is on row {at} and page {} draws {drawn:?}",
                            duel.ability_page
                        ));
                    }
                }
                baylee_client::input::ability_menu_keys(
                    Fired::of_actions(&[Action::CursorDown]),
                    &mut duel,
                );
            }
            if duel.ability_pick != 0 {
                found.push(format!(
                    "{name}: {} steps down landed on {} rather than back at the top",
                    options.len(),
                    duel.ability_pick
                ));
            }
            for (at, times) in seen.iter().enumerate() {
                if *times != 1 {
                    found.push(format!(
                        "{name}: walking the list passed row {at} {times} times"
                    ));
                }
            }

            // And the way out. One press of cancel puts the sheet away; on a
            // bubble stepped into from a row it is one step back instead,
            // which is a state this loop never enters.
            let mut duel = duel_at(&view, &pending);
            baylee_client::input::activate_card(&mut duel, id);
            baylee_client::input::ability_menu_keys(
                Fired::of_actions(&[Action::Cancel]),
                &mut duel,
            );
            if duel.ability_menu.is_some() {
                found.push(format!("{name}: cancel left the sheet standing"));
            }
            if !duel.outbox().is_empty() {
                found.push(format!("{name}: cancel sent something"));
            }
        }
    }

    println!(
        "sheet cursor: {walked} sheets walked, {} findings",
        found.len()
    );
    assert!(
        walked >= SHEET_FLOOR,
        "only {walked} sheets were walked, under the floor of {SHEET_FLOOR}"
    );
    assert!(
        found.is_empty(),
        "{} sheets move a cursor onto something they do not draw:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// **Every row of a sheet says what the card says.**
///
/// The owner's rule, in his own words: *"es gibt keine sondercases, überall
/// steht der original skryfall text in der clientsprache - fallback auf
/// englisch"*. There is exactly one source of words on this sheet — the
/// printed text of the printing, fetched from the gateway's catalog and,
/// behind it, from Scryfall — and the client composes none of its own. It
/// used to: `abilities::effect_label` wrote German prose out of an
/// `Effect` list, which meant a row read one way with a catalog, another way
/// without one, and a third way for an effect nobody had named yet.
///
/// What a sweep can hold is the half that decides *offline*: whether the
/// client knows **which** printed sentence a row is. That is
/// `AbilityOption::printed`, and a row that has none can draw no words
/// however good the catalog is.
///
/// Four kinds of row are exempt, and each of them for the same reason rather
/// than four: **it is printed on no card.** Three of them are exactly what
/// `AbilityOption::printed_index` answers `None` for — the CR 305.6 mana of a
/// basic land type, a granted ability, a prepared cast — and the sheet asks
/// the same method before it draws this client's own name for a row, so the
/// exemption here and the wording there cannot come apart. The fourth is a
/// **pip** of a poured bubble, whose whole sentence is the colour it pours
/// and which is drawn as that mana symbol.
///
/// Anything else is a position in the card's own ability list, and the card
/// prints a sentence for it.
#[test]
fn every_written_row_knows_which_printed_sentence_it_is() {
    let cards = candidates();
    let mut found: Vec<String> = Vec::new();
    let mut rows = 0usize;

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view, interaction, ..
        }) = seated(batch)
        else {
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                continue;
            };
            for option in abilities::options(Lang::En, &view, &interaction, object.id) {
                // A pip is its colour and nothing else.
                if option.pour.is_some() {
                    continue;
                }
                let Some(index) = option.printed_index() else {
                    continue;
                };
                rows += 1;
                if option.printed.is_none() {
                    found.push(format!("{}: ability {index}", def.name()));
                }
            }
        }
    }

    println!(
        "sheet words: {rows} rows of a card's own abilities, {} with no printed sentence",
        found.len()
    );
    assert!(
        rows >= SHEET_FLOOR,
        "only {rows} rows were read, under the floor of {SHEET_FLOOR}"
    );
    assert!(
        found.is_empty(),
        "{} rows draw an ability the card prints and cannot say which sentence it is:\n{}",
        found.len(),
        found.join("\n")
    );
}

/// What the sweep above leaves open: that a row which knows its sentence also
/// has **words** with nothing filed at all — the client offline, which is also
/// every client before its gateway has answered.
///
/// Each row goes through `cardtext::sentence`, the one door the sheet, the
/// stack and the cast chooser draw through, with no text filed, so what is
/// asked is exactly whether the compiled English Oracle carries the sentence
/// the line table points at. Before it was compiled in, every one of these
/// rows was blank offline.
#[test]
fn every_written_row_has_words_with_no_text_filed() {
    let cards = candidates();
    let mut blank: Vec<String> = Vec::new();
    let mut rows = 0usize;

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view, interaction, ..
        }) = seated(batch)
        else {
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                continue;
            };
            let card = object
                .rules
                .expect("a card on the battlefield names what it prints")
                .card;
            for option in abilities::options(Lang::En, &view, &interaction, object.id) {
                let Some(printed) = option.printed else {
                    continue;
                };
                rows += 1;
                if !has_words(card, printed) {
                    blank.push(format!(
                        "{}: line {} of {}",
                        def.name(),
                        printed.line,
                        printed.of
                    ));
                }
            }
        }
    }

    println!("offline words: {rows} rows, {} blank", blank.len());
    assert!(
        rows >= SHEET_FLOOR,
        "only {rows} rows were read, under the floor of {SHEET_FLOOR}"
    );
    assert!(
        blank.is_empty(),
        "{} rows know their sentence and draw no words offline:\n{}",
        blank.len(),
        blank.join("\n")
    );
}

/// The sweep's question, taken out so that its failing branch can be seen to
/// fail.
fn has_words(card: baylee_core::ids::CardIndex, at: baylee_view::StackText) -> bool {
    baylee_client::cardtext::sentence(None, card, None, at)
        .is_some_and(|blocks| blocks.iter().any(|b| !b.text().is_empty()))
}

/// The injection for the sweep above: a coordinate past the card's last
/// sentence has no words, so a sweep that found none blank was asking a
/// question that can be answered no.
#[test]
fn a_row_pointing_past_its_card_s_text_has_no_words() {
    let card = by_name("Mind Stone");
    let draw = baylee_view::StackText {
        face: 0,
        line: 1,
        of: 2,
    };
    assert!(has_words(card, draw));
    assert!(!has_words(card, baylee_view::StackText { line: 2, ..draw }));
}

/// How a row's printed sentence came apart in the sheet: see
/// [`every_written_row_draws_its_printed_cost_or_its_whole_sentence`].
#[derive(Debug, PartialEq, Eq)]
enum Column {
    /// The head is the column, licensed by what the ability costs.
    Head,
    /// No cost colon ahead of the reminder or quotation: drawn whole.
    Whole,
    /// A cost colon whose head the ability's cost does not license.
    Refused,
}

/// The sweep's question, taken out so that its failing branch can be seen.
fn column(sentence: &[baylee_client_core::card_face::TextBlock], key: &str) -> Column {
    use baylee_client_core::card_face::TextBlock;
    let colon = match sentence.first() {
        Some(TextBlock::Rules(first)) => {
            baylee_cardtext::split_cost(first).is_some_and(|split| !split.body.is_empty())
        }
        _ => false,
    };
    match (abilitysheet::cut(sentence.to_vec(), key).head, colon) {
        (Some(_), _) => Column::Head,
        (None, false) => Column::Whole,
        (None, true) => Column::Refused,
    }
}

/// Every written row draws the printed head of its own sentence as its cost,
/// or its whole sentence and no column — and **never** a sentence whose cost
/// colon the ability's own cost does not license.
///
/// The column used to be the ability's `Cost` in seventeen composed wordings,
/// which the owner struck: *„nicht custom texte für abilities verwenden,
/// sondern die echten texte von skryfall"*. It is now the head of the printed
/// sentence, cut where the ability's symbols license it
/// (`abilitysheet::cut`). A refused row still draws — whole, with an empty
/// column — so a refusal is not a blank; it is either the line table pointing
/// at the wrong sentence, or the key spelling a symbol differently from
/// Scryfall (`ManaCost`'s `Display` against `cardtext::symbols`), and either
/// would drop the column from every row of its kind in silence. Asked offline,
/// of the compiled English Oracle.
///
/// The rows drawn whole are the ones with no cost colon at all, which is what
/// a keyword line is (`Equip {1}`, whose only colon is in its reminder):
/// listed by name in [`DRAWN_WHOLE`], equality both ways, so a row that lost
/// its column shows up as a new name and a row that gained one as a stale
/// entry.
#[test]
fn every_written_row_draws_its_printed_cost_or_its_whole_sentence() {
    let cards = candidates();
    let mut heads = 0usize;
    let mut whole: Vec<String> = Vec::new();
    let mut refused: Vec<String> = Vec::new();

    for batch in cards.chunks(BATCH) {
        let Some(Seated {
            view, interaction, ..
        }) = seated(batch)
        else {
            continue;
        };
        for def in batch {
            let Some(object) = view
                .battlefield
                .iter()
                .find(|o| o.card.as_ref().is_some_and(|c| c.index == def.index))
            else {
                continue;
            };
            let card = object
                .rules
                .expect("a card on the battlefield names what it prints")
                .card;
            for option in abilities::options(Lang::En, &view, &interaction, object.id) {
                if option.pour.is_some() {
                    continue;
                }
                let Some(printed) = option.printed else {
                    continue;
                };
                let sentence = baylee_client::cardtext::sentence(None, card, None, printed)
                    .expect("every written row has words offline");
                let key = option.cost.as_deref().unwrap_or_default();
                let first = sentence.first().map(|b| b.text().to_string());
                match column(&sentence, key) {
                    Column::Head => heads += 1,
                    Column::Whole => whole.push(def.name().to_string()),
                    Column::Refused => refused.push(format!(
                        "{}: key `{key}` against `{}`",
                        def.name(),
                        first.unwrap_or_default()
                    )),
                }
            }
        }
    }
    whole.sort();
    whole.dedup();

    println!(
        "cost column: {heads} printed heads, {} cards drawn whole, {} refused",
        whole.len(),
        refused.len()
    );
    println!("drawn whole: {whole:?}");
    assert!(
        heads >= SHEET_FLOOR,
        "only {heads} rows drew a printed head, under the floor of {SHEET_FLOOR}"
    );
    assert!(
        refused.is_empty(),
        "{} rows print a cost their ability does not cost:\n{}",
        refused.len(),
        refused.join("\n")
    );
    let named: Vec<&str> = DRAWN_WHOLE.to_vec();
    assert_eq!(
        whole, named,
        "the rows drawn whole, with no cost column, are the keyword lines named here"
    );
}

/// Cards whose written row has no cost colon in its sentence, and is drawn
/// whole with no column. Held equal by the sweep above.
///
/// Every one a keyword line whose cost stands after the keyword and whose
/// only colon is in its reminder: Equip on sixteen of them, Level up
/// (Hexdrinker), Reconfigure (Rabbit Battery), Station (U.S.S. Enterprise-D).
/// Measured 24.09.2026 over the compiled English Oracle: 1302 rows drew a
/// printed head, these 19 cards drew whole, none was refused.
const DRAWN_WHOLE: &[&str] = &[
    "Basilisk Collar",
    "Bonesplitter",
    "Fireshrieker",
    "Hexdrinker",
    "Leonin Scimitar",
    "Lightning Greaves",
    "Loxodon Warhammer",
    "Nettlecyst",
    "Neurok Hoversail",
    "No-Dachi",
    "Rabbit Battery",
    "Shuko",
    "Skullclamp",
    "Slagwurm Armor",
    "Swiftfoot Boots",
    "Sword of Hearth and Home",
    "U.S.S. Enterprise-D, Galaxy-Class",
    "Vulshok Battlegear",
    "Vulshok Morningstar",
];

/// The injection for the sweep above: a key that does not license the head
/// is refused, a sentence with no cost colon is drawn whole, and a head that
/// is licensed comes off — so a sweep that found nothing refused was asking a
/// question that can be answered each way.
#[test]
fn a_row_whose_key_does_not_license_its_head_is_refused() {
    let card = by_name("Mind Stone");
    let at = baylee_view::StackText {
        face: 0,
        line: 1,
        of: 2,
    };
    let sentence =
        baylee_client::cardtext::sentence(None, card, None, at).expect("Mind Stone's draw line");
    assert_eq!(column(&sentence, "{1}, {T}"), Column::Head);
    assert_eq!(column(&sentence, "{W}, {T}"), Column::Refused);
    let equip = vec![baylee_client_core::card_face::TextBlock::Rules(
        "Equip {1}".into(),
    )];
    assert_eq!(column(&equip, "{1}"), Column::Whole);
}
