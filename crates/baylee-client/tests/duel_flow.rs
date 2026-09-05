//! End-to-end: play a real game through the client's own code path.
//!
//! Every layer except the renderer is exercised here — the host, the wire
//! encoding, the board model, the token grouping, and the interaction state
//! machine — driven by a loop that answers whatever it is asked the way a
//! player would. No window, no GPU, so it runs in CI on every commit.
//!
//! This is the test that would have caught the setup bug where a seat marked
//! `Open` (which is what every hosted game marks its human seat) started with
//! no library at all: the existing suites all asserted that *choices arrived*,
//! and none of them ever looked at what the human could actually see.

use baylee_client::host::{DuelHost, HostMessage, LocalHost};
use baylee_client_core::board::{BoardModel, Openings, SeatPod};
use baylee_client_core::browser::Browser;
use baylee_client_core::interaction::{CombatFocus, Interaction};
use baylee_core::ids::{CardIndex, PlayerId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{GameStatic, PlayerView};
use std::collections::BTreeSet;

/// The engine's whole question vocabulary, written once and read twice.
///
/// The macro is the point: `variant_name` matches on every arm with no
/// wildcard, so a new `Pending` variant stops this file compiling instead of
/// slipping past as an untested question — and the roster it is checked
/// against is built from the same list, so the two cannot drift. A
/// hand-written array beside a hand-written match would go stale on the first
/// variant somebody added.
macro_rules! question_vocabulary {
    ($($variant:ident),* $(,)?) => {
        /// Every question the engine can ask, by name.
        const EVERY_QUESTION: &[&str] = &[$(stringify!($variant)),*];

        fn question_name(pending: &Pending) -> &'static str {
            match pending {
                $(Pending::$variant { .. } => stringify!($variant),)*
            }
        }
    };
}

question_vocabulary!(
    Mulligan,
    MulliganBottom,
    Priority,
    ChooseAttackers,
    ChooseBlockers,
    DiscardChoice,
    LegendChoice,
    ChooseCards,
    ChooseTargets,
    ChooseSubtype,
    ChooseColor,
    YesNo,
    ChooseCastMode,
    ChooseNumber,
    ChoosePlayer,
    OrderObjects,
    GameOver,
);

fn card(oracle: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle)
        .expect("the acceptance registry contains the card")
        .index
}

/// A duel of basic lands plus a cheap creature, so turns actually progress.
fn duel_preset(seed: u64) -> GamePreset {
    let forest = card(FOREST);
    let deck: Vec<DeckEntry> = (0..60)
        .map(|_| DeckEntry {
            card: forest,
            print: PrintRef::new(0),
        })
        .collect();
    let seat = |ai: bool| SeatSpec {
        controller: if ai {
            SeatController::Ai(AIProfile::default())
        } else {
            SeatController::Open
        },
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat(false), seat(true)],
    }
}

/// The same duel with a squad already on the table for seat 0.
///
/// Creatures have to come from somewhere for combat to be reachable at all,
/// and casting them would test the engine's mana, not the client's combat.
/// `starting_battlefield` is the preset field that exists for exactly this.
fn combat_preset(seed: u64) -> GamePreset {
    let mut preset = duel_preset(seed);
    let squad = card(GREAT_DIVIDE_GUIDE);
    preset.seats[0].starting_battlefield = (0..10)
        .map(|_| DeckEntry {
            card: squad,
            print: PrintRef::new(0),
        })
        .collect();
    preset
}

/// The mirror image of `combat_preset`: the squad is the *opponent's*, so the
/// house AI attacks and this seat is the one asked to block.
///
/// Nothing else in this file ever reaches `Pending::ChooseBlockers` with
/// anything to say — a seat with an empty battlefield is asked nothing — so
/// the client's blocking path was untested end to end.
fn blocking_preset(seed: u64) -> GamePreset {
    let mut preset = duel_preset(seed);
    let squad = card(GREAT_DIVIDE_GUIDE);
    let squad_of = |n: usize| -> Vec<DeckEntry> {
        (0..n)
            .map(|_| DeckEntry {
                card: squad,
                print: PrintRef::new(0),
            })
            .collect()
    };
    preset.seats[1].starting_battlefield = squad_of(6);
    // Something to block *with*, but fewer of them: a house AI looking at an
    // equal wall of 1/2s has no profitable attack and simply does not come.
    preset.seats[0].starting_battlefield = squad_of(2);
    preset
}

// The cards the scenarios below are built out of, by oracle id — the same
// handle `card` already takes. Named because a bare uuid in a preset says
// nothing about why that card is there, and every one of these is chosen for
// one sentence of its oracle text.
/// Basic lands, for a seat that has to pay for what it is holding.
const PLAINS: &str = "bc71ebf6-2056-41f7-be35-b2e5c34afa99";
/// See [`PLAINS`].
const ISLAND: &str = "b2c6aa39-2d2a-459c-a555-fb48ba993373";
/// See [`PLAINS`].
const FOREST: &str = "b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6";
/// `{1}{W}`, and an enters trigger that says "choose one" — a modal *ability*,
/// which is where `ChooseCastMode` comes from off the stack rather than at
/// cast time. Its first mode scries, which asks two more questions again.
const CHARMING_PRINCE: &str = "c48d844c-3976-4fa5-8e0d-3f0e535e7619";
/// `{2}{U}`, "you **may** search your library" — the optional half is `YesNo`
/// and the search itself is `ChooseCards` out of a zone the table cannot draw.
const SPELLSEEKER: &str = "47a785ed-8095-4685-8daa-02c4e2b0ffcd";
/// `{2}{G}`, a search for *up to two* cards with two destinations: the same
/// `ChooseCards` with a real choice of how many, rather than a forced one.
const CULTIVATE: &str = "8b755881-a72d-4e21-a369-d2924eb4585a";
/// `{1}{U}` with overload — a modal choice made while *casting*, which is a
/// different code path in the engine from Charming Prince's.
const CYCLONIC_RIFT: &str = "d75b9c82-1b49-4c3e-a1b5-aeef57d6644b";
/// `{X}{U}{U}{U}` targeting a player: `ChooseNumber` for the X and a target
/// that is a player rather than an object.
const COMMANDERS_INSIGHT: &str = "54d7d7f8-22cd-4859-b203-924d248b422b";
/// `{2}{W}` legendary. Two of them on the table is the whole legend rule.
const LORAN: &str = "b3d81980-76f2-44e2-b1c9-01e30c726312";
/// `{2}{U}{U}`, kicker `{5}`, "a copy of target creature" — two questions in
/// one card: the kicker is a `YesNo` asked while casting, and the copy needs a
/// creature pointed at, which is the only `ChooseTargets` in the pool this
/// suite can set up on demand.
const RITE_OF_REPLICATION: &str = "fb60739e-1dc3-481d-a056-ad72e665c680";
/// `{1}` artifact whose first ability looks at three cards and puts them back
/// **in any order** — `OrderObjects`, which nothing else here can raise: every
/// other ordering in the engine comes off a library nobody in this file digs
/// into.
const SENSEIS_DIVINING_TOP: &str = "13575cf9-65c1-4861-b21e-eb2155e07766";
/// The 1/2 the combat scenarios are built out of, and the creature Rite of
/// Replication points at.
const GREAT_DIVIDE_GUIDE: &str = "79e69a91-d580-47fb-be76-1e32c50d2fa0";

/// One card, at the default printing.
fn entry(oracle: &str) -> DeckEntry {
    DeckEntry {
        card: card(oracle),
        print: PrintRef::new(0),
    }
}

/// A seat with mana already on the table and a hand somebody chose.
///
/// Every question this file could not reach needs a *card* to ask it, and the
/// deck both seats play is sixty Forests. So a scenario becomes a list of
/// cards rather than a game played into position: lands on the battlefield,
/// the spells in hand, and the greedy loop spends them.
///
/// Six lands of each colour, which is well past what any spell here costs —
/// and deliberately so. A kicker is a question only while it is *payable*, and
/// an `{X}` is only a question when X has more than one legal value, so both
/// depend on there being mana left over after the spell itself.
///
/// `board` is what else seat 0 starts with in play: the permanent a spell has
/// to point at, or the one whose ability asks the question.
fn spellbook_preset(seed: u64, hand: &[&str], board: &[&str]) -> GamePreset {
    let mut preset = duel_preset(seed);
    let mut table = Vec::new();
    for oracle in [PLAINS, ISLAND, FOREST] {
        table.extend((0..6).map(|_| entry(oracle)));
    }
    table.extend(board.iter().copied().map(entry));
    preset.seats[0].starting_battlefield = table;
    preset.seats[0].starting_hand = Some(hand.iter().copied().map(entry).collect());
    preset
}

/// Two of one legend on the table.
///
/// The only way to reach the legend rule at all: CR 704.5j is a state-based
/// action, so it is asked before the first priority of the game rather than
/// as the result of anything a player does.
fn legend_preset(seed: u64) -> GamePreset {
    let mut preset = duel_preset(seed);
    preset.seats[0].starting_battlefield = vec![entry(LORAN), entry(LORAN)];
    preset
}

/// How the headless player answers a combat declaration.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Fight {
    /// Declare nothing. What a player holding the pass key would do, and what
    /// every test that is not about combat wants.
    #[default]
    Never,
    /// Attack with everything that may attack, and block nothing.
    Always,
    /// Block every attacker, and attack with nothing.
    ///
    /// Two knobs and not one, because they are mutually exclusive on a board
    /// this size: a squad that attacked on its own turn is tapped through the
    /// opponent's, so a seat set to `Always` is never in a position to block.
    Defend,
}

/// The client state a headless run needs: exactly what the Bevy resource holds,
/// minus anything that touches a device.
#[derive(Default)]
struct Client {
    statics: Option<GameStatic>,
    view: Option<PlayerView>,
    board: Option<BoardModel>,
    pending: Option<Pending>,
    errors: Vec<String>,
    fight: Fight,
    /// Whether priority is spent rather than passed. A player who only passes
    /// never plays the land that asks for a creature type and never taps the
    /// dual that asks for a colour, so the questions those cards ask are
    /// unreachable from the pass loop — which is not the same as unhandled.
    greedy: bool,
    /// How many opening hands to throw away before keeping one.
    ///
    /// A player who always keeps is never asked which cards to put back, so
    /// `MulliganBottom` is unreachable from every other knob in this file. One
    /// mulligan is enough: London bottoms one card for the first, which is the
    /// smallest version of the question that is still the question.
    mulligans: u8,
    /// Every question this seat was actually asked, and every one it produced
    /// an answer to. Without them a `Pending` variant no scenario reaches is
    /// indistinguishable from one the client handles — the loop below would
    /// pass either way, because it only ever sees the questions it is given.
    met: BTreeSet<&'static str>,
    answered: BTreeSet<&'static str>,
    /// The question the last submitted answer was answering.
    ///
    /// A refusal arrives as a bare string, and "illegal action for your
    /// seat" five hundred times over says nothing about which answer the
    /// engine would not take. Together with `sent` this is what turns it
    /// into a report.
    asked: Option<&'static str>,
    /// The answer itself, as the engine saw it.
    sent: Option<String>,
}

impl Client {
    fn absorb(&mut self, messages: Vec<HostMessage>) {
        for message in messages {
            match message {
                HostMessage::Static(s) => self.statics = Some(*s),
                HostMessage::View(v) => {
                    self.board = Some(BoardModel::from_view(&v, Openings::none(), 12.0));
                    self.view = Some(*v);
                }
                HostMessage::Choice(p) => self.pending = Some(*p),
                HostMessage::Failed(e) => {
                    let asked = self.asked.unwrap_or("nothing this loop asked about");
                    let sent = self.sent.as_deref().unwrap_or("nothing");
                    self.errors
                        .push(format!("{e}, answering {asked} with {sent}"));
                }
            }
        }
    }

    /// Answers the pending choice the way a player holding down the pass key
    /// would: keep the opening hand, decline everything optional, attack with
    /// nothing, and pass priority.
    fn answer(&mut self, seat: PlayerId) -> Option<PlayerAction> {
        let pending = self.pending.clone()?;
        let mut interaction = Interaction::new(pending.clone(), seat);
        if !interaction.is_mine() {
            return None;
        }
        let question = question_name(&pending);
        self.asked = Some(question);
        if !matches!(pending, Pending::GameOver(_)) {
            self.met.insert(question);
        }
        let action = self.decide(&mut interaction, &pending);
        if let Some(action) = &action {
            self.answered.insert(question);
            self.sent = Some(format!("{action:?}"));
        }
        action
    }

    /// Whether this is the seat's own main phase — the only window in which
    /// floating more mana than the next spell costs is worth anything.
    fn spending_now(&self) -> bool {
        self.view.as_ref().is_some_and(|view| {
            view.active == view.seat
                && matches!(
                    view.phase,
                    baylee_view::Phase::FirstMain | baylee_view::Phase::SecondMain
                )
        })
    }

    /// The answer itself, split out so `answer` can keep the books around it
    /// without every arm below having to remember to.
    fn decide(&mut self, interaction: &mut Interaction, pending: &Pending) -> Option<PlayerAction> {
        match pending {
            // A player who actually does something with a turn: take the land
            // drop, then whatever a permanent offers. Both of the questions
            // this suite could otherwise only reach by hand-built actions --
            // a creature type as a land enters, a colour as one is tapped --
            // are on the far side of exactly this.
            Pending::Priority { legal, .. } if self.greedy => {
                let mana = if self.spending_now() {
                    legal.mana_abilities.first().copied()
                } else {
                    None
                };
                if let Some(land) = legal.lands.first().copied() {
                    interaction.play_card(land)
                } else if let Some(source) = mana {
                    // Every source first, and only then a spell. A loop that
                    // cast the moment it could afford *something* never floats
                    // more than the cheapest card in hand costs — which is why
                    // no scenario here had ever been offered a range for `{X}`
                    // or a kicker it was in a position to pay. Both questions
                    // exist only when there is mana left over.
                    //
                    // In the seat's own main phase, though, and not the first
                    // priority it is given. Mana empties at the end of every
                    // step (CR 500.4), so a loop that tapped out in upkeep
                    // arrived at the main phase with eighteen tapped lands and
                    // an uncastable hand — which is exactly what it did.
                    interaction.activate(source, 0)
                } else if let Some(spell) = legal.castable.first().copied() {
                    interaction.play_card(spell)
                } else if let Some(&(source, index)) = legal.abilities.first() {
                    interaction.activate(source, index)
                } else {
                    interaction.confirm()
                }
            }
            Pending::Mulligan { .. } => {
                let keep = self.mulligans == 0;
                self.mulligans = self.mulligans.saturating_sub(1);
                interaction.answer_mulligan(keep)
            }
            Pending::YesNo { .. } => interaction.answer_yes_no(false),
            // Bottoming and discarding name no options: the engine means
            // "your hand", so the answer comes off the hand bar's own model.
            Pending::MulliganBottom { count, .. } | Pending::DiscardChoice { count, .. } => {
                let board = self.board.as_ref()?;
                for card in board.hand.iter().take(*count as usize) {
                    interaction.toggle(card.id);
                }
                interaction.confirm()
            }
            // Every choice among *objects*, answered by clicking them where
            // a player would find them: on the table, or in the zone browser
            // for the zones the table cannot draw. Building `ChooseObjects`
            // by hand passed just as green while the client had no way to
            // show a library search at all — the same shortcut that hid the
            // colour bug, one prompt along.
            Pending::LegendChoice { .. }
            | Pending::ChooseCards { .. }
            | Pending::ChooseTargets { .. } => {
                let view = self.view.as_ref()?;
                let wanted = smallest_legal_pick(pending);
                for id in interaction.selectable().to_vec() {
                    if interaction.selected().len() >= wanted {
                        break;
                    }
                    assert!(
                        can_reach(view, interaction, id),
                        "{pending:?} offers {id:?} and nothing on screen draws it"
                    );
                    interaction.toggle(id);
                }
                interaction.confirm()
            }
            // An ordering is every offered card, clicked in turn — which is
            // exactly what the tray asks a player to do.
            Pending::OrderObjects { .. } => {
                let view = self.view.as_ref()?;
                for id in interaction.selectable().to_vec() {
                    assert!(
                        can_reach(view, interaction, id),
                        "an ordering offers {id:?} and nothing on screen draws it"
                    );
                    interaction.toggle(id);
                }
                interaction.confirm()
            }
            _ => self.decide_indexed(interaction, pending),
        }
    }

    /// The rest of the same answer: the choices taken by *position* and the
    /// two taken by aim. Split off from `decide` for its length alone — the
    /// match simply continues here, and the catch-all lives at the bottom.
    fn decide_indexed(
        &self,
        interaction: &mut Interaction,
        pending: &Pending,
    ) -> Option<PlayerAction> {
        match pending {
            // The choices answered by *position*, taken the way the prompt
            // bar takes them: the rows out of `choices::options`, then
            // `choose_index` and `confirm`. This used to build the action
            // by hand, which is exactly how the client came to have no way
            // of answering a colour at all -- a tapped dual land stopped
            // the game dead while this test happily played on, because the
            // test was proving that the *engine* accepts an answer, not
            // that anybody could give one.
            Pending::ChooseColor { .. } | Pending::ChoosePlayer { .. } => {
                let rows = baylee_client::choices::options(
                    &interaction.prompt(),
                    baylee_client_core::Lang::En,
                    self.statics.as_ref(),
                    "",
                )
                .expect("an indexed choice offers rows");
                assert!(!rows.is_empty(), "a choice with no rows cannot be answered");
                let index = rows[0].index;
                interaction
                    .choose_index(index)
                    .then(|| interaction.confirm())?
            }
            // A cast option has rows too, but only when the engine offered
            // any; an empty list would be a chooser with nothing in it.
            Pending::ChooseCastMode { .. } => {
                interaction.choose_index(0).then(|| interaction.confirm())?
            }
            // A creature type goes the whole way a player does: type a few
            // letters, take what is still on screen, send *that row's* index.
            // Answering it with `choose_index(0)` would pass just as green
            // while proving nothing about the filter -- which is the same
            // shortcut that hid the colour bug, one prompt along.
            Pending::ChooseSubtype { options, .. } => {
                let typed = options
                    .first()
                    .and_then(|id| baylee_core::generated::subtypes::name(*id))
                    .map_or(String::new(), |name| name[..2].to_lowercase());
                let rows = baylee_client::choices::options(
                    &interaction.prompt(),
                    baylee_client_core::Lang::En,
                    self.statics.as_ref(),
                    &typed,
                )
                .expect("a creature type is a chooser");
                assert!(
                    !rows.is_empty(),
                    "typing the start of an offered type must leave it on screen"
                );
                let index = rows[0].index;
                interaction
                    .choose_index(index)
                    .then(|| interaction.confirm())?
            }
            // Combat, driven the way `input.rs` drives it: aim, declare each
            // candidate against the aim, confirm. Nothing here reaches past
            // the interaction layer, so a client that cannot express an attack
            // fails this test rather than quietly passing the turn.
            Pending::ChooseAttackers { .. } if self.fight == Fight::Always => {
                let CombatFocus::Defender(defender) = interaction.combat_focus() else {
                    return interaction.confirm();
                };
                for attacker in interaction.selectable().to_vec() {
                    assert!(
                        interaction.declare_attacker(attacker, defender),
                        "the engine offered a candidate the client then refused"
                    );
                }
                interaction.confirm()
            }
            // And the other half of it. `declare_blocker` is reached from the
            // shell through the generic click path, never by name, so nothing
            // in this file used to drive it at all: `ChooseBlockers` fell into
            // the catch-all below and declared no blocks, which is a legal
            // answer and proves nothing.
            Pending::ChooseBlockers { .. } if self.fight == Fight::Defend => {
                let CombatFocus::Attacker(attacker) = interaction.combat_focus() else {
                    return interaction.confirm();
                };
                for blocker in interaction.selectable().to_vec() {
                    assert!(
                        interaction.declare_blocker(blocker, attacker),
                        "the engine offered a blocker the client then refused"
                    );
                }
                interaction.confirm()
            }
            Pending::GameOver(_) => None,
            // Priority, attackers, blockers, and X all confirm to a safe
            // default through the interaction itself.
            _ => interaction.confirm(),
        }
    }
}

/// Plays up to `steps` decisions of the standard duel and returns the state.
fn play(seed: u64, steps: usize) -> (Client, LocalHost) {
    run(&duel_preset(seed), steps, Fight::Never)
}

/// Plays up to `steps` decisions and returns the final client state.
fn run(preset: &GamePreset, steps: usize, fight: Fight) -> (Client, LocalHost) {
    drive(
        preset,
        steps,
        Client {
            fight,
            ..Client::default()
        },
    )
}

/// The same loop, played by somebody who spends their turn instead of passing
/// it. Only for the coverage ledger: the tests that assert on a specific card
/// drive it by hand, because they are about *that* card's prompt.
fn run_greedily(preset: &GamePreset, steps: usize) -> Client {
    drive(
        preset,
        steps,
        Client {
            greedy: true,
            ..Client::default()
        },
    )
    .0
}

/// The loop itself, with the client handed in already set up.
///
/// One loop and not three. Every scenario differs only in what it *answers*,
/// and a copy of the loop per knob is how a scenario comes to stop one step
/// early without anybody noticing.
fn drive(preset: &GamePreset, steps: usize, mut client: Client) -> (Client, LocalHost) {
    let mut host =
        LocalHost::new(preset, PlayerId::new(0), &["You", "House AI"]).expect("the duel starts");
    client.absorb(host.poll());

    for _ in 0..steps {
        if matches!(client.pending, Some(Pending::GameOver(_))) {
            break;
        }
        let Some(action) = client.answer(PlayerId::new(0)) else {
            break;
        };
        host.submit(action);
        client.absorb(host.poll());
        // A refused action leaves the question standing, so the next pass
        // builds the same answer and it is refused again — five hundred
        // identical lines that say nothing about the first one. Stop at the
        // first: whatever asserts on `errors` then has one report to show.
        if !client.errors.is_empty() {
            break;
        }
    }
    (client, host)
}

/// The same duel with a dual land on the table, so a mana ability that asks
/// which colour is reachable at all.
/// Cavern of Souls in the opening hand.
///
/// It asks its question as it *enters*, not when it is tapped, so this is the
/// worse half of the same deadlock: a land drop -- the most ordinary thing a
/// turn contains -- and the game stops. The card is `Coverage::Implemented`,
/// so the deckbuilder offers it.
fn cavern_preset(seed: u64) -> GamePreset {
    let mut preset = duel_preset(seed);
    let cavern = card("89ca686a-7c72-4d8f-9290-e89635624a83");
    preset.seats[0].starting_hand = Some(vec![DeckEntry {
        card: cavern,
        print: PrintRef::new(0),
    }]);
    preset
}

/// Playing Cavern of Souls asks for a creature type, and the client answers
/// it the way a player does: type a few letters, take a row that is still on
/// screen, send *that row's* index.
///
/// The index is the point. A filtered list shows twelve of some three hundred
/// and fifty, so a row's position is not its answer, and a chooser that sent
/// the position would name the wrong creature type -- silently, and only for
/// players who typed.
#[test]
fn a_cavern_of_souls_can_be_played_and_its_type_answered() {
    let preset = cavern_preset(7);
    let mut host =
        LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"]).expect("the duel starts");
    let mut client = Client::default();
    client.absorb(host.poll());

    // Walk to a priority that offers the land, playing it rather than passing.
    let mut played = false;
    for _ in 0..200 {
        let pending = client.pending.clone().expect("a choice");
        if let Pending::Priority { legal, .. } = &pending
            && let Some(land) = legal.lands.first().copied()
        {
            let interaction = Interaction::new(pending.clone(), PlayerId::new(0));
            let action = interaction
                .play_card(land)
                .expect("the engine offered this land drop");
            host.submit(action);
            client.absorb(host.poll());
            played = true;
            break;
        }
        let Some(action) = client.answer(PlayerId::new(0)) else {
            break;
        };
        host.submit(action);
        client.absorb(host.poll());
    }
    assert!(played, "the land never came up as playable");

    let pending = client.pending.clone().expect("a choice");
    let Pending::ChooseSubtype { options, .. } = &pending else {
        panic!("a Cavern entering the battlefield asks for a creature type, got {pending:?}");
    };
    assert!(
        options.len() > baylee_client::choices::SUBTYPE_ROWS,
        "the engine offers every creature type, which is why there is a filter"
    );

    // Type "elf", and check the row that comes back answers the engine's own
    // option rather than the position it happens to sit at.
    let mut interaction = Interaction::new(pending.clone(), PlayerId::new(0));
    let rows = baylee_client::choices::options(
        &interaction.prompt(),
        baylee_client_core::Lang::En,
        client.statics.as_ref(),
        "elf",
    )
    .expect("a creature type is a chooser");
    assert_eq!(rows[0].label, "Elf");
    assert!(
        rows[0].index > 0,
        "an elf is not the engine's first creature type, so position != answer"
    );
    let elf = options[rows[0].index];

    assert!(interaction.choose_index(rows[0].index));
    let action = interaction.confirm().expect("a picked row is submittable");
    assert_eq!(
        action,
        PlayerAction::ChooseSubtype(elf),
        "the chooser answered the type the row named"
    );
    host.submit(action);
    client.absorb(host.poll());
    assert!(
        !matches!(client.pending, Some(Pending::ChooseSubtype { .. })),
        "the engine moved on, so the answer was accepted"
    );
}

fn dual_land_preset(seed: u64) -> GamePreset {
    let mut preset = duel_preset(seed);
    // Underground Sea: `{T}: Add {U} or {B}` -- a mana ability the engine
    // cannot resolve without asking, which is the whole point of it here.
    let sea = card("4b22be3a-8ce1-47d1-b82e-6c3ccfb0548b");
    preset.seats[0].starting_battlefield = vec![DeckEntry {
        card: sea,
        print: PrintRef::new(0),
    }];
    preset
}

/// Tapping a dual land is a question, and the client has to be able to answer
/// it. This is the deadlock this test exists for: the prompt bar drew "Choose
/// a colour" with no buttons under it, `Interaction::choose_index` was never
/// called by anything, and the game stopped there for good -- in any deck with
/// a dual land, the first time one was tapped for its mana.
#[test]
fn a_dual_land_can_be_tapped_and_the_colour_answered() {
    let preset = dual_land_preset(11);
    let mut host =
        LocalHost::new(&preset, PlayerId::new(0), &["You", "House AI"]).expect("the duel starts");
    let mut client = Client::default();
    client.absorb(host.poll());

    // Walk to a priority that offers the land's ability, answering whatever
    // is asked on the way.
    let mut asked = false;
    for _ in 0..200 {
        let pending = client.pending.clone().expect("a choice");
        if let Pending::Priority { legal, .. } = &pending
            && let Some((source, index)) = legal.abilities.first().copied()
        {
            let interaction = Interaction::new(pending.clone(), PlayerId::new(0));
            let action = interaction
                .activate(source, index)
                .expect("the engine offered this ability");
            host.submit(action);
            client.absorb(host.poll());
            asked = true;
            break;
        }
        let Some(action) = client.answer(PlayerId::new(0)) else {
            break;
        };
        host.submit(action);
        client.absorb(host.poll());
    }
    assert!(asked, "the land never offered its ability");

    let pending = client.pending.clone().expect("a choice");
    let Pending::ChooseColor { options, .. } = &pending else {
        panic!("tapping a two-colour land asks which colour, got {pending:?}");
    };
    assert_eq!(options.len(), 2, "blue or black");

    // And now the part that did not exist: the rows the prompt bar draws, and
    // the answer a click on one of them sends.
    let mut interaction = Interaction::new(pending.clone(), PlayerId::new(0));
    let rows = baylee_client::choices::options(
        &interaction.prompt(),
        baylee_client_core::Lang::En,
        client.statics.as_ref(),
        "",
    )
    .expect("a colour choice is a chooser");
    assert_eq!(rows.len(), 2, "one row per colour the engine offered");
    assert!(
        rows.iter().all(|r| r.pip.is_some()),
        "a colour row is drawn as its mana symbol"
    );

    assert!(interaction.choose_index(1));
    let action = interaction.confirm().expect("a picked row is submittable");
    assert_eq!(action, PlayerAction::ChooseColor(options[1]));
    host.submit(action);
    client.absorb(host.poll());

    assert!(
        !matches!(client.pending, Some(Pending::ChooseColor { .. })),
        "the game is still asking for a colour after one was given"
    );
    assert!(
        client.errors.is_empty(),
        "engine refused: {:?}",
        client.errors
    );
}

#[test]
fn a_human_seat_is_dealt_a_real_opening_hand_and_library() {
    let (client, _) = play(4, 0);
    let view = client.view.expect("the client can draw immediately");

    assert_eq!(view.hand.len(), 7, "the human seat must have a hand");
    let me = view.seat(PlayerId::new(0)).expect("own seat line");
    assert_eq!(me.library_count, 53);
    assert!(!me.is_decking_out());

    // And the opponent's hand is a count, never contents.
    let them = view.seat(PlayerId::new(1)).expect("opponent seat line");
    assert_eq!(them.hand_count, 7);
    assert_eq!(them.library_count, 53);
}

#[test]
fn the_game_advances_through_the_clients_own_path() {
    let (client, _) = play(4, 400);
    let view = client.view.expect("a view");

    assert!(
        client.errors.is_empty(),
        "the client sent something the engine rejected: {:?}",
        client.errors
    );
    assert!(
        view.turn > 1,
        "several turns should have passed, was on turn {}",
        view.turn
    );
}

#[test]
fn lands_reach_the_battlefield_and_show_up_grouped_in_the_board_model() {
    let (client, _) = play(4, 400);
    let board = client.board.expect("a board model");
    let pods: usize = board.pods.iter().map(SeatPod::permanent_count).sum();
    assert!(pods > 0, "somebody should have played a land by now");

    // Sixty identical Forests are the grouping case: however many are on the
    // battlefield, they draw as very few cards.
    for pod in &board.pods {
        let lands = pod
            .lane(baylee_client_core::layout::LaneKind::Lands)
            .expect("a lands lane");
        if lands.permanent_count() > 1 {
            assert!(
                lands.groups.len() < lands.permanent_count(),
                "identical lands must collapse: {} groups for {} permanents",
                lands.groups.len(),
                lands.permanent_count()
            );
        }
    }
}

#[test]
fn the_static_payload_lets_every_board_card_resolve_to_an_image() {
    let (client, _) = play(4, 400);
    let statics = client.statics.expect("the static payload");
    let board = client.board.expect("a board model");

    for key in board.required_images() {
        assert!(
            statics.print(key.print).is_some(),
            "the print table must cover every card the board wants to draw"
        );
    }
}

#[test]
fn the_client_never_builds_an_action_the_engine_refuses() {
    // The strongest property the interaction layer claims: whatever the game
    // asks, the answer the client constructs is accepted. Any rejection would
    // show up as an error message from the host.
    for seed in [1u64, 7, 23, 99] {
        let (client, _) = play(seed, 250);
        assert!(
            client.errors.is_empty(),
            "seed {seed} produced rejected actions: {:?}",
            client.errors
        );
    }
}

#[test]
fn the_same_seed_plays_out_identically() {
    // Determinism is the platform's central claim; a client that replays a
    // seed must land in the same place, or replays and reconnects are worthless.
    let (a, _) = play(12, 200);
    let (b, _) = play(12, 200);

    let (va, vb) = (a.view.expect("view"), b.view.expect("view"));
    assert_eq!(va.turn, vb.turn);
    assert_eq!(va.seats[0].life, vb.seats[0].life);
    assert_eq!(va.seats[0].library_count, vb.seats[0].library_count);
    assert_eq!(a.board, b.board, "the same seed must render the same board");
}

#[test]
fn hidden_information_never_reaches_the_client() {
    let (client, _) = play(4, 300);
    let view = client.view.expect("a view");

    // The opponent's hand is a number and nothing else: there is no field on
    // the view that could carry their cards, which is the point of the wire
    // type's shape.
    let them = view.seat(PlayerId::new(1)).expect("opponent");
    assert!(them.hand_count > 0);

    // Every object the client can see is in a public zone or its own hand.
    for object in &view.battlefield {
        assert!(
            object.card.is_some() || object.name == "Face-down" || !object.name.is_empty(),
            "a visible permanent must be identifiable or explicitly hidden"
        );
    }
}

#[test]
fn a_whole_game_can_be_won_through_the_clients_combat_path() {
    // The claim this test exists to make: a player can finish a game with
    // nothing but the client. Every decision below is built by `Interaction`
    // from what the engine offered, combat included, and the game ends because
    // ten 1/2s walked across the table — not because a deck ran out.
    let (client, _) = run(&combat_preset(9), 4_000, Fight::Always);

    let Some(Pending::GameOver(result)) = client.pending else {
        panic!("the game never ended: {:?}", client.pending);
    };
    assert_eq!(
        result.winner,
        Some(baylee_engine::win::Victor::Player(PlayerId::new(0)))
    );
    assert!(client.errors.is_empty(), "{:?}", client.errors);

    let view = client.view.expect("a final view");
    let them = view.seat(PlayerId::new(1)).expect("opponent seat line");
    assert!(
        them.life <= 0,
        "the opponent should be dead, at {}",
        them.life
    );
}

#[test]
fn declaring_no_attackers_is_the_same_answer_as_declaring_none() {
    // `input.rs` answers "None" by cancelling and confirming, which has to
    // produce the empty declaration the engine accepts — not a refusal to
    // answer, which would hang the turn.
    let (client, _) = run(&combat_preset(9), 4_000, Fight::Never);
    assert!(client.errors.is_empty(), "{:?}", client.errors);

    // Nobody attacked, so nobody is dead and the client is still being asked.
    let view = client.view.expect("a view");
    for seat in [PlayerId::new(0), PlayerId::new(1)] {
        assert!(view.seat(seat).expect("seat line").life > 0);
    }
}

/// How few objects a choice will accept — the answer a pass-key player gives.
fn smallest_legal_pick(pending: &Pending) -> usize {
    match pending {
        Pending::ChooseCards { min, .. } | Pending::ChooseTargets { min, .. } => *min as usize,
        // The legend rule keeps exactly one.
        Pending::LegendChoice { .. } => 1,
        _ => 0,
    }
}

/// Whether `id` is drawn somewhere the player could click it: on the table
/// (the board model) or in the zone browser (every zone the table cannot
/// show). The client's one real claim about answering a choice about objects.
fn can_reach(view: &PlayerView, interaction: &Interaction, id: baylee_core::ids::ObjectId) -> bool {
    let board = BoardModel::from_view(view, Openings::none(), 12.0);
    let on_table = board
        .pods
        .iter()
        .flat_map(|p| p.lanes.iter())
        .flat_map(|l| l.groups.iter())
        .any(|g| g.members.contains(&id))
        || board.hand.iter().any(|c| c.id == id);
    let mut browser = Browser::new();
    browser.follow(view, Some(interaction));
    on_table
        || browser
            .rows(view, Some(interaction))
            .iter()
            .any(|r| r.id == id && r.selectable)
}

/// Questions no scenario in this file has ever put to the client.
///
/// Empty, and the assertion is what keeps it that way: every question the
/// engine can ask is now put to the client by some scenario here and answered
/// through `Interaction`. A name would belong here only as a *finding* — a
/// prompt a real game can raise that nothing here asks, whose handling is
/// therefore unproven, because the pass loop would look exactly this green
/// whether that arm were correct or missing entirely. It is asserted exactly
/// rather than merely printed, so a scenario that stops reaching one fails the
/// build instead of quietly shrinking the ledger.
const UNREACHED: &[&str] = &[];

/// The one member of the vocabulary that is not a question.
///
/// `Pending::GameOver` is a state, not something to answer: there is no action
/// a seat could send. It is in [`EVERY_QUESTION`] because the engine has the
/// variant and the macro may not skip arms, and out of the ledger because a
/// seat that "answered" it would be answering nothing.
const NOT_A_QUESTION: &[&str] = &["GameOver"];

/// The ledger: what the seat was asked, what it answered, and what nothing
/// asked it.
///
/// Two claims, and only the first is an assertion about the client as it
/// stands: every question this suite reaches, it answers. The second is the
/// map of where the suite itself does not go.
#[test]
fn every_question_this_suite_reaches_gets_an_answer() {
    let mut met: BTreeSet<&'static str> = BTreeSet::new();
    let mut answered: BTreeSet<&'static str> = BTreeSet::new();
    // A macro rather than a closure, for the one thing a closure cannot do:
    // `stringify!` names the scenario in the failure. Twelve runs feed this
    // ledger, and "the engine refused an answer" without saying which run
    // produced it is a finding somebody then has to go looking for.
    macro_rules! record {
        ($scenario:expr) => {{
            let client: &Client = &$scenario;
            // An answer the engine refused is not an answer. Without this the
            // ledger would count a question as answered because the client
            // *produced* an action, not because anything accepted it.
            assert!(
                client.errors.is_empty(),
                "{}: the engine refused an answer the client built: {:?}",
                stringify!($scenario),
                client.errors
            );
            met.extend(client.met.iter().copied());
            answered.extend(client.answered.iter().copied());
        }};
    }

    record!(play(4, 400).0);
    record!(run(&combat_preset(9), 4_000, Fight::Always).0);
    record!(run(&combat_preset(9), 4_000, Fight::Never).0);
    record!(run_greedily(&cavern_preset(7), 400));
    record!(run_greedily(&dual_land_preset(11), 400));
    record!(run_greedily(&combat_preset(9), 400));
    record!(run(&blocking_preset(3), 4_000, Fight::Defend).0);
    // A hand somebody chose, so the questions a Forest never asks get asked.
    record!(run_greedily(
        &spellbook_preset(5, &[CHARMING_PRINCE, SPELLSEEKER, CULTIVATE], &[]),
        600,
    ));
    record!(run_greedily(
        &spellbook_preset(6, &[CYCLONIC_RIFT, COMMANDERS_INSIGHT], &[]),
        600,
    ));
    record!(run_greedily(
        &spellbook_preset(7, &[RITE_OF_REPLICATION], &[GREAT_DIVIDE_GUIDE]),
        600,
    ));
    record!(run_greedily(
        &spellbook_preset(13, &[], &[SENSEIS_DIVINING_TOP]),
        600,
    ));
    record!(run_greedily(&legend_preset(8), 200));
    // And the one question that is only asked of a player who says no twice:
    // the house rules give the first mulligan free, so one puts nothing back.
    record!(
        drive(
            &duel_preset(12),
            200,
            Client {
                greedy: true,
                mulligans: 2,
                ..Client::default()
            },
        )
        .0
    );

    let unanswered: Vec<&str> = met.difference(&answered).copied().collect();
    assert!(
        unanswered.is_empty(),
        "the engine asked and the client had no answer: {unanswered:?}"
    );

    let unreached: Vec<&str> = EVERY_QUESTION
        .iter()
        .copied()
        .filter(|question| !NOT_A_QUESTION.contains(question) && !met.contains(question))
        .collect();
    assert_eq!(
        unreached, UNREACHED,
        "the set of questions this suite never asks has changed"
    );
}

/// A player can block, and the block is what the engine acts on.
///
/// The companion to `a_whole_game_can_be_won_through_the_clients_combat_path`,
/// and the half that was missing: that test proves a seat can *attack*, and a
/// seat with an empty battlefield is never asked to block, so nothing here had
/// ever put `Pending::ChooseBlockers` to the interaction layer with a real
/// answer available. The claim is the life total — six 1/2s walk in every turn
/// and none of their damage reaches the player, which is only true if the
/// pairings the client built came back as blocks.
#[test]
fn an_attack_can_be_blocked_through_the_clients_own_path() {
    let (defended, _) = run(&blocking_preset(3), 600, Fight::Defend);
    let (exposed, _) = run(&blocking_preset(3), 600, Fight::Never);

    assert!(
        defended.met.contains("ChooseBlockers"),
        "the house AI never attacked, so this preset proves nothing about blocking"
    );
    assert!(
        defended.answered.contains("ChooseBlockers"),
        "the client was asked to block and had nothing to say"
    );
    assert!(
        defended.errors.is_empty(),
        "the engine refused something the client sent: {:?}",
        defended.errors
    );

    // The same duel, same seed, same number of decisions -- the one difference
    // being whether the pairings the client built were sent. If blocking did
    // nothing, the two life totals would be identical.
    let mine = defended.view.as_ref().expect("a view").seats[0].life;
    let theirs = exposed.view.as_ref().expect("a view").seats[0].life;
    assert!(
        mine > theirs,
        "blocking left this seat at {mine} life and not blocking at {theirs}, \
         so the blocks the client declared never reached combat"
    );
}
