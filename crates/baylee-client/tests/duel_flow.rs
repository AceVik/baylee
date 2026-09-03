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

fn card(oracle: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle)
        .expect("the acceptance registry contains the card")
        .index
}

/// A duel of basic lands plus a cheap creature, so turns actually progress.
fn duel_preset(seed: u64) -> GamePreset {
    let forest = card("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6");
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
    let squad = card("79e69a91-d580-47fb-be76-1e32c50d2fa0"); // Great Divide Guide, 1/2
    preset.seats[0].starting_battlefield = (0..10)
        .map(|_| DeckEntry {
            card: squad,
            print: PrintRef::new(0),
        })
        .collect();
    preset
}

/// How the headless player answers a combat declaration.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Fight {
    /// Declare nothing. What a player holding the pass key would do, and what
    /// every test that is not about combat wants.
    #[default]
    Never,
    /// Attack with everything that may attack.
    Always,
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
                HostMessage::Failed(e) => self.errors.push(e),
            }
        }
    }

    /// Answers the pending choice the way a player holding down the pass key
    /// would: keep the opening hand, decline everything optional, attack with
    /// nothing, and pass priority.
    fn answer(&self, seat: PlayerId) -> Option<PlayerAction> {
        let pending = self.pending.clone()?;
        let mut interaction = Interaction::new(pending.clone(), seat);
        if !interaction.is_mine() {
            return None;
        }
        match &pending {
            Pending::Mulligan { .. } => interaction.answer_mulligan(true),
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
                let wanted = smallest_legal_pick(&pending);
                for id in interaction.selectable().to_vec() {
                    if interaction.selected().len() >= wanted {
                        break;
                    }
                    assert!(
                        can_reach(view, &interaction, id),
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
                        can_reach(view, &interaction, id),
                        "an ordering offers {id:?} and nothing on screen draws it"
                    );
                    interaction.toggle(id);
                }
                interaction.confirm()
            }
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
    let mut host =
        LocalHost::new(preset, PlayerId::new(0), &["You", "House AI"]).expect("the duel starts");
    let mut client = Client {
        fight,
        ..Client::default()
    };
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
