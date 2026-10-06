use super::*;
use crate::llmseat::ledger::{Ask, Budget};

/// A key as a player might paste it.
const KEY: &str = "sk-ant-api03-AAAABBBBCCCCDDDDEEEE";

/// A file with three profiles, the middle one the default, and caps.
fn three() -> SeatSettings {
    SeatSettings::parse(
        r#"{
          "default": "b",
          "caps": { "day_usd": 10, "month_tokens": 9000000 },
          "profiles": {
            "a": { "provider": "anthropic", "model": "claude-opus-5-5", "max_tokens": 24000 },
            "b": { "provider": "anthropic", "model": "claude-sonnet-5-5", "game_usd": 2.5 },
            "c": {
              "provider": "openai", "model": "deepseek-chat", "answer": "json",
              "price": { "input": 0.3, "output": 1.2 },
              "key_env": "DEEPSEEK_API_KEY", "base_url": "https://api.deepseek.com/v1"
            }
          }
        }"#,
    )
    .expect("a file that reads")
}

fn panel() -> SeatPanel {
    SeatPanel::new(Disk::Read(three()))
}

/// Puts the caret in `spot` and makes its box say `text`, as a player
/// selecting all and typing over it would.
fn type_into(panel: &mut SeatPanel, spot: Spot, text: &str) {
    panel.act(Act::Focus(spot));
    assert_eq!(panel.focus(), Some(spot), "{spot:?} takes the caret");
    panel.edit(|buffer| {
        buffer.select_all();
        buffer.insert(text);
    });
}

/// The first fault shown at `spot`.
fn fault_at(panel: &SeatPanel, spot: Spot) -> Option<Problem> {
    panel
        .faults()
        .into_iter()
        .find(|fault| fault.spot == spot)
        .map(|fault| fault.problem)
}

#[test]
fn a_file_reads_into_the_boxes_and_back_unchanged() {
    let panel = panel();
    assert_eq!(panel.len(), 3);
    assert_eq!(panel.name(1), Some("b"));
    assert_eq!(panel.default(), Some(1));
    assert_eq!(panel.selected(), Some(0));
    assert_eq!(
        panel
            .buffer(Spot::Profile(1, Slot::GameUsd))
            .map(TextBuffer::text),
        Some("2.5")
    );
    assert_eq!(
        panel
            .buffer(Spot::Cap(CapField::DayUsd))
            .map(TextBuffer::text),
        Some("10"),
        "no trailing .0"
    );
    assert_eq!(panel.answer(2), Some(AnswerMode::Json));
    assert!(!panel.changed());
    assert_eq!(panel.faults(), []);
    assert_eq!(panel.to_save(), None, "nothing edited, nothing to write");
    // What the boxes say is the file, to the last field.
    assert_eq!(panel.read().0, three());
}

#[test]
fn a_missing_file_is_an_empty_panel_that_adding_a_profile_fills() {
    let mut panel = SeatPanel::new(Disk::Missing);
    assert!(panel.is_empty());
    assert!(panel.editable());
    assert_eq!(panel.selected(), None);
    assert_eq!(panel.default(), None);
    assert_eq!(panel.to_save(), None, "no empty file is written");
    assert_eq!(panel.spent(), None, "no book read yet");

    panel.act(Act::Add);
    assert_eq!(panel.len(), 1);
    assert_eq!(panel.selected(), Some(0));
    assert_eq!(
        panel.focus(),
        Some(Spot::Profile(0, Slot::Name)),
        "the caret waits in the new name"
    );
    let settings = panel.to_save().expect("a profile to write");
    assert_eq!(settings.check(), Ok(()));
    let profile = &settings.profiles["sonnet"];
    assert_eq!(profile.model, DEFAULT_ANTHROPIC_MODEL);
    assert_eq!(settings.default, None, "nobody chose a default");
    // Typing replaces the selected name.
    panel.edit(|buffer| buffer.insert("mine"));
    assert!(panel.to_save().unwrap().profiles.contains_key("mine"));

    panel.act(Act::Remove(0));
    assert!(panel.is_empty());
    assert_eq!(panel.focus(), None);
    assert_eq!(panel.to_save(), None, "back to no file, nothing to write");
}

#[test]
fn a_refused_file_is_shown_and_never_written_over() {
    let mut panel = SeatPanel::new(Disk::Refused("the settings file x: not JSON".into()));
    assert!(!panel.editable());
    panel.act(Act::Add);
    assert!(
        panel.is_empty(),
        "nothing is added over a file that did not read"
    );
    assert_eq!(panel.to_save(), None);
    assert!(panel.found(Disk::Read(three())), "mended by hand");
    assert!(panel.editable());
    assert_eq!(panel.len(), 3);
}

/// Every refusal the settings file has appears beside the field it is
/// about, and Save waits while any stands.
#[test]
fn every_refusal_shows_beside_its_field() {
    let at = |slot| Spot::Profile(0, slot);
    let typed: Vec<(Spot, &str, Problem)> = vec![
        (at(Slot::Name), "two words", Problem::Refused(Why::NotAName)),
        (at(Slot::Name), "api_key", Problem::Refused(Why::KeyNamed)),
        (at(Slot::Name), "b", Problem::NameTaken),
        (
            at(Slot::Model),
            "no spaces",
            Problem::Refused(Why::NotAModelId),
        ),
        (at(Slot::Model), "", Problem::Refused(Why::NotAModelId)),
        (
            at(Slot::Effort),
            "Very High",
            Problem::Refused(Why::NotAWord),
        ),
        (at(Slot::MaxTokens), "0", Problem::Refused(Why::Zero)),
        (at(Slot::MaxTokens), "lots", Problem::NotAWholeNumber),
        (at(Slot::MaxTokens), "5000000000", Problem::NotAWholeNumber),
        // The half left empty is the one that says so.
        (at(Slot::PriceIn), "3", Problem::HalfAPrice),
        (at(Slot::PriceOut), "-1", Problem::NotDollars),
        (at(Slot::GameTokens), "1e6", Problem::NotAWholeNumber),
        (at(Slot::ThinkSecs), "0", Problem::Refused(Why::Zero)),
        (
            at(Slot::KeyEnv),
            "my key",
            Problem::Refused(Why::NotAVariable),
        ),
        (
            at(Slot::BaseUrl),
            "http://api.example.com",
            Problem::Refused(Why::NotSecure),
        ),
        (Spot::Cap(CapField::MonthUsd), "lots", Problem::NotDollars),
        (
            Spot::Cap(CapField::DayTokens),
            "2.5",
            Problem::NotAWholeNumber,
        ),
    ];
    for (spot, text, problem) in typed {
        let mut panel = panel();
        type_into(&mut panel, spot, text);
        let shown = if problem == Problem::HalfAPrice {
            at(Slot::PriceOut)
        } else {
            spot
        };
        assert_eq!(
            fault_at(&panel, shown),
            Some(problem.clone()),
            "{spot:?} «{text}»"
        );
        assert_eq!(panel.to_save(), None, "{spot:?} «{text}» blocks Save");
        let fault = PanelFault {
            spot: shown,
            problem,
        };
        let (en, de) = (panel.say(&fault, Lang::En), panel.say(&fault, Lang::De));
        assert!(!en.is_empty() && en != de, "{spot:?}: «{en}» «{de}»");
    }

    // A key in the model's box is one fault, beside it: the model it
    // blanks has no price, and that is not said a second time.
    let mut panel = self::panel();
    type_into(&mut panel, Spot::Profile(1, Slot::Model), KEY);
    assert_eq!(
        panel.faults(),
        [PanelFault {
            spot: Spot::Profile(1, Slot::Model),
            problem: Problem::Refused(Why::KeyShaped),
        }]
    );
    // A dollar budget for a model with no price, and the price that mends it.
    let mut panel = self::panel();
    type_into(&mut panel, Spot::Profile(1, Slot::Model), "mystery-1");
    assert_eq!(
        fault_at(&panel, Spot::Profile(1, Slot::GameUsd)),
        Some(Problem::Refused(Why::Unpriced))
    );
    type_into(&mut panel, Spot::Profile(1, Slot::PriceIn), "1");
    type_into(&mut panel, Spot::Profile(1, Slot::PriceOut), "4,5");
    assert_eq!(panel.faults(), []);
    let saved = panel.to_save().expect("mended");
    assert_eq!(
        saved.profiles["b"].price,
        Some(GivenPrice {
            input: 1.0,
            output: 4.5
        }),
        "a decimal comma reads"
    );
}

/// The answer's choice is refused beside it where the provider cannot
/// answer that way: JSON, plain or by its schema, is an OpenAI-compatible
/// endpoint's.
#[test]
fn a_json_answer_is_an_openai_compatible_endpoint_s() {
    for json in [AnswerMode::Json, AnswerMode::JsonSchema] {
        let mut panel = panel();
        panel.act(Act::Answer(0, Some(json)));
        assert_eq!(
            fault_at(&panel, Spot::Profile(0, Slot::Answer)),
            Some(Problem::Refused(Why::JsonNeedsOpenAi)),
            "{json:?}"
        );
        panel.act(Act::Provider(0, Provider::OpenAi));
        assert_eq!(
            panel.faults(),
            [],
            "{json:?} is an OpenAI-compatible endpoint's"
        );
        let saved = panel.to_save().expect("an edit with nothing wrong");
        let name = panel.name(0).expect("a profile").to_string();
        assert_eq!(saved.profiles[&name].answer, Some(json));
    }
}

/// Each way the file can be refused is placed on a field, and said in
/// both languages: a new refusal fails to compile here until it is.
#[test]
fn every_refusal_has_a_place_and_words() {
    let all = [
        Why::KeyNamed,
        Why::KeyShaped,
        Why::NoSuchProfile,
        Why::NotAName,
        Why::NotAModelId,
        Why::NoSuchTool,
        Why::NotAWord,
        Why::JsonNeedsOpenAi,
        Why::NotForCli,
        Why::CliOnly,
        Why::NotAbsolute,
        Why::Zero,
        Why::NotAnAmount,
        Why::Unpriced,
        Why::NotAVariable,
        Why::NotSecure,
        Why::Unwritable,
    ];
    for why in all {
        // Exhaustive, so that the list above cannot fall behind.
        let spot = match why {
            Why::KeyNamed | Why::KeyShaped | Why::NotAName => Spot::Profile(0, Slot::Name),
            Why::NoSuchProfile => Spot::Default,
            Why::NotAModelId | Why::NoSuchTool => Spot::Profile(0, Slot::Model),
            Why::NotAWord => Spot::Profile(0, Slot::Effort),
            Why::JsonNeedsOpenAi => Spot::Profile(0, Slot::Answer),
            Why::NotForCli => Spot::Profile(0, Slot::PriceIn),
            Why::CliOnly | Why::NotAbsolute => Spot::Profile(0, Slot::Command),
            Why::Zero => Spot::Profile(0, Slot::MaxTokens),
            Why::NotAnAmount => Spot::Cap(CapField::DayUsd),
            Why::Unpriced => Spot::Profile(0, Slot::GameUsd),
            Why::NotAVariable => Spot::Profile(0, Slot::KeyEnv),
            Why::NotSecure => Spot::Profile(0, Slot::BaseUrl),
            Why::Unwritable => Spot::File,
        };
        let fault = PanelFault {
            spot,
            problem: Problem::Refused(why),
        };
        let panel = panel();
        let (en, de) = (panel.say(&fault, Lang::En), panel.say(&fault, Lang::De));
        assert!(!en.is_empty() && !de.is_empty() && en != de, "{why:?}");
        assert!(!en.contains('{'), "{why:?}: {en}");
    }
}

/// A key typed or pasted into any box is refused where it was typed, and
/// the panel never offers it for saving.
#[test]
fn a_key_typed_anywhere_is_never_saved() {
    let mut spots: Vec<Spot> = Slot::TYPED
        .iter()
        .map(|slot| Spot::Profile(2, *slot))
        .collect();
    spots.extend(CapField::ALL.iter().map(|cap| Spot::Cap(*cap)));
    assert_eq!(spots.len(), 17, "every box on the panel");
    for pasted in [KEY, "Bearer abcdef", "x-api-key: 12345"] {
        for spot in &spots {
            let mut panel = panel();
            type_into(&mut panel, *spot, pasted);
            let problem = fault_at(&panel, *spot);
            // A bearer header's space makes a name no name at all; every
            // other box says it is a key.
            let key = Some(Problem::Refused(Why::KeyShaped));
            assert!(
                problem == key
                    || (pasted != KEY && problem == Some(Problem::Refused(Why::NotAName))),
                "{spot:?} «{pasted}»: {problem:?}"
            );
            assert_eq!(panel.to_save(), None, "{spot:?} «{pasted}»");
            // And what the panel would write says so too.
            let (settings, _) = panel.read();
            assert!(!settings.to_json().contains("AAAABBBB") || settings.check().is_err());
        }
    }
}

#[test]
fn the_default_follows_renames_and_removals() {
    let mut panel = panel();
    type_into(&mut panel, Spot::Profile(1, Slot::Name), "bee");
    assert_eq!(panel.default(), Some(1));
    assert_eq!(panel.to_save().unwrap().default.as_deref(), Some("bee"));

    // Removing a profile above it moves it up a place, and keeps it.
    panel.act(Act::Remove(0));
    assert_eq!(panel.default(), Some(0));
    assert_eq!(panel.name(0), Some("bee"));
    assert_eq!(panel.to_save().unwrap().default.as_deref(), Some("bee"));

    // A copy lands below and takes a name of its own; the default stays.
    panel.act(Act::Duplicate(0));
    assert_eq!(panel.name(1), Some("bee-2"));
    assert_eq!(panel.default(), Some(0));
    assert_eq!(panel.selected(), Some(1));
    let saved = panel.to_save().unwrap();
    assert_eq!(saved.profiles["bee-2"], saved.profiles["bee"]);

    // Removing a profile below leaves it be.
    panel.act(Act::Remove(2));
    assert_eq!(panel.default(), Some(0));

    // Removing the default leaves none, rather than a model nobody chose.
    panel.act(Act::Remove(0));
    assert_eq!(panel.default(), None);
    assert_eq!(panel.name(0), Some("bee-2"));
    assert_eq!(panel.to_save().unwrap().default, None);

    // Pressing Default makes it, and pressing it again unmakes it.
    panel.act(Act::Default(0));
    assert_eq!(panel.to_save().unwrap().default.as_deref(), Some("bee-2"));
    panel.act(Act::Default(0));
    assert_eq!(panel.default(), None);
}

#[test]
fn removing_the_shown_profile_shows_its_neighbour() {
    let mut panel = panel();
    panel.act(Act::Select(2));
    panel.act(Act::Focus(Spot::Profile(2, Slot::Model)));
    panel.act(Act::Remove(2));
    assert_eq!(panel.selected(), Some(1), "the last removed, the one above");
    assert_eq!(panel.focus(), None, "the caret went with its box");
    panel.act(Act::Focus(Spot::Profile(1, Slot::Model)));
    panel.act(Act::Remove(0));
    assert_eq!(panel.selected(), Some(0));
    assert_eq!(
        panel.focus(),
        Some(Spot::Profile(0, Slot::Model)),
        "the caret stays in its box as the box moves up"
    );
}

/// The book's sums for the day and the month the moment falls in, on the
/// player's clock.
#[test]
fn spent_today_and_this_month_are_read_at_a_given_moment() {
    // 2026-09-30 12:00:00 UTC, and two hours east of it.
    const NOON: i64 = 1_790_769_600;
    let cest = Some(2 * 3600);
    let moment = |unix| Moment { unix, offset: cest };
    let ask = |usd: f64| Ask {
        profile: Some("b"),
        model: "claude-sonnet-5-5",
        game: Budget::Usd(usd),
        floor: Budget::Usd(0.25),
    };
    let unpriced = Ask {
        profile: Some("c"),
        model: "deepseek-chat",
        game: Budget::Tokens(40_000),
        floor: Budget::Tokens(1_000),
    };
    let caps = Caps::default();
    let mut ledger = Ledger::default();
    // Yesterday: a dollar and a quarter, settled.
    let first = ledger
        .reserve(&caps, &ask(3.0), moment(NOON - 86_400))
        .unwrap();
    ledger
        .settle(first.id, Some(1.25), 9_000, moment(NOON - 86_000))
        .unwrap();
    // Today: two dollars still open, and a game of a model with no price.
    ledger.reserve(&caps, &ask(2.0), moment(NOON)).unwrap();
    let tokens = ledger.reserve(&caps, &unpriced, moment(NOON + 60)).unwrap();
    ledger
        .settle(tokens.id, None, 12_345, moment(NOON + 600))
        .unwrap();
    // Just after local midnight: tomorrow already, in the month after.
    ledger
        .reserve(&caps, &ask(0.5), moment(NOON + 12 * 3600))
        .unwrap();

    let mut panel = panel();
    panel.read_book(Ok(ledger), moment(NOON));
    let view = panel.spent().unwrap().unwrap();
    assert_eq!(
        (view.day.as_str(), view.month.as_str()),
        ("2026-09-30", "2026-09")
    );
    assert!((view.today.usd - 2.0).abs() < 1e-9, "{view:?}");
    assert_eq!(view.today.tokens, 12_345);
    assert_eq!((view.today.priced.played, view.today.priced.open), (1, 1));
    assert!((view.this_month.usd - 3.25).abs() < 1e-9, "{view:?}");
    assert_eq!(view.offset.as_deref(), Some("UTC+02:00"));

    let lines = panel.spent_lines(Lang::En);
    assert_eq!(lines[0], "Counted in local time, UTC+02:00.");
    assert_eq!(
        lines[1],
        "Today, 2026-09-30: $2.00 of $10.00 · 12,345 tokens · games: 2, still open: 1"
    );
    assert_eq!(
        lines[2],
        "This month, 2026-09: $3.25 · 12,345 of 9,000,000 tokens · games: 3, still open: 1"
    );
    let german = panel.spent_lines(Lang::De);
    assert_eq!(german[0], "Gezählt in Ortszeit, UTC+02:00.");
    assert!(german[1].contains("2,00 $ von 10,00 $"), "{}", german[1]);

    // Past local midnight the day and the month move on, and nothing else
    // had to be read.
    assert!(panel.tick(moment(NOON + 12 * 3600 + 60)));
    let view = panel.spent().unwrap().unwrap();
    assert_eq!(
        (view.day.as_str(), view.month.as_str()),
        ("2026-10-01", "2026-10")
    );
    assert!((view.today.usd - 0.5).abs() < 1e-9, "{view:?}");
    assert!(!panel.tick(moment(NOON + 12 * 3600 + 120)), "the same day");

    // Where the platform will not say, the days are UTC's, and it is said.
    panel.tick(Moment {
        unix: NOON,
        offset: None,
    });
    assert_eq!(panel.spent_lines(Lang::En)[0], "Counted in UTC.");
    // A book that cannot be read says so, and nothing else.
    panel.read_book(Err("broken".into()), moment(NOON));
    assert_eq!(
        panel.spent_lines(Lang::En),
        ["The spend book cannot be read: broken"]
    );
}

#[test]
fn a_file_changed_on_disk_is_shown_unless_there_are_edits() {
    let mut panel = panel();
    let mut changed = three();
    changed.profiles.remove("a");
    assert!(panel.found(Disk::Read(changed.clone())));
    assert_eq!(panel.len(), 2, "no edits: the file as it now is");
    assert!(!panel.newer_on_disk());
    assert!(!panel.found(Disk::Read(changed.clone())), "the same again");

    type_into(&mut panel, Spot::Profile(0, Slot::Effort), "high");
    let mut again = changed;
    again.default = None;
    assert!(panel.found(Disk::Read(again)));
    assert!(
        panel.newer_on_disk(),
        "the edit is kept, and the change said"
    );
    assert_eq!(
        panel
            .buffer(Spot::Profile(0, Slot::Effort))
            .map(TextBuffer::text),
        Some("high")
    );
    assert!(panel.to_save().is_some(), "saving writes over it");
    panel.act(Act::Revert);
    assert!(!panel.newer_on_disk());
    assert_eq!(
        panel.default(),
        None,
        "discarding shows the file as it is now"
    );
    assert_eq!(
        panel
            .buffer(Spot::Profile(0, Slot::Effort))
            .map(TextBuffer::text),
        Some("")
    );

    // The caret goes with its profile when the file adds one above it.
    let mut panel = self::panel();
    panel.act(Act::Focus(Spot::Profile(1, Slot::Model)));
    let mut more = three();
    more.profiles
        .insert("aa".into(), more.profiles["a"].clone());
    assert!(panel.found(Disk::Read(more)));
    assert_eq!(panel.selected(), Some(2));
    assert_eq!(panel.focus(), Some(Spot::Profile(2, Slot::Model)));
    assert_eq!(panel.name(2), Some("b"));

    // A file that goes away, with nothing edited, is the empty panel.
    assert!(panel.found(Disk::Missing));
    assert!(panel.is_empty());
}

#[test]
fn a_save_is_remembered_until_the_next_edit() {
    let mut panel = panel();
    type_into(&mut panel, Spot::Profile(0, Slot::Effort), "high");
    let settings = panel.to_save().unwrap();
    panel.saved(settings.clone());
    assert_eq!(panel.disk(), &Disk::Read(settings));
    assert!(!panel.changed());
    assert_eq!(panel.last_save(), Some(&Saved::Written));
    assert_eq!(panel.to_save(), None);
    panel.edit(|buffer| buffer.insert("er"));
    assert_eq!(panel.last_save(), None);
    panel.not_saved("disk full".into());
    assert_eq!(panel.last_save(), Some(&Saved::Failed("disk full".into())));
}

#[test]
fn tab_walks_the_shown_profile_then_the_caps() {
    let mut panel = panel();
    panel.act(Act::Select(1));
    panel.tab(false);
    assert_eq!(panel.focus(), Some(Spot::Profile(1, Slot::Name)));
    panel.tab(false);
    assert_eq!(panel.focus(), Some(Spot::Profile(1, Slot::Model)));
    panel.tab(true);
    panel.tab(true);
    assert_eq!(
        panel.focus(),
        Some(Spot::Cap(CapField::MonthTokens)),
        "back from the first box is the last"
    );
    panel.blur();
    assert!(!panel.typing());

    // An Anthropic profile draws no address box, so the caret never goes
    // into one; an OpenAI-compatible one does.
    panel.act(Act::Focus(Spot::Profile(1, Slot::KeyEnv)));
    panel.tab(false);
    assert_eq!(panel.focus(), Some(Spot::Cap(CapField::DayUsd)));
    assert!(!panel.shows(Spot::Profile(1, Slot::BaseUrl)));
    panel.act(Act::Focus(Spot::Profile(2, Slot::KeyEnv)));
    panel.tab(false);
    assert_eq!(panel.focus(), Some(Spot::Profile(2, Slot::BaseUrl)));
    // Switched to Anthropic, its address stays in view while it holds one.
    panel.act(Act::Provider(2, Provider::Anthropic));
    panel.blur();
    assert!(panel.shows(Spot::Profile(2, Slot::BaseUrl)));
}

/// A CLI profile shows the boxes a CLI takes and hides an API's, says
/// how its model and program are named and what it plays with when they
/// are left empty, and refuses beside each box what a CLI does not take.
#[test]
fn a_cli_profile_shows_a_cli_s_boxes_and_refuses_an_api_s() {
    let mut panel = panel();
    panel.act(Act::Provider(0, Provider::Cli));
    let spot = |slot| Spot::Profile(0, slot);
    assert_eq!(
        fault_at(&panel, spot(Slot::Model)),
        Some(Problem::Refused(Why::NoSuchTool)),
        "claude-opus-5-5 names no tool"
    );
    type_into(&mut panel, spot(Slot::Model), "claude:opus");
    panel.blur();
    assert_eq!(panel.faults(), []);
    for (slot, shown) in [
        (Slot::Model, true),
        (Slot::Effort, true),
        (Slot::GameTokens, true),
        (Slot::GameCalls, true),
        (Slot::Command, true),
        (Slot::KeyEnv, false),
        (Slot::BaseUrl, false),
        (Slot::PriceIn, false),
        (Slot::PriceOut, false),
        (Slot::GameUsd, false),
    ] {
        assert_eq!(panel.shows(spot(slot)), shown, "{slot:?}");
    }
    assert!(panel.shows(Spot::Profile(1, Slot::KeyEnv)), "an API's");
    assert!(!panel.shows(Spot::Profile(1, Slot::Command)), "an API's");
    let hint = |panel: &SeatPanel, slot| panel.hint(spot(slot), Lang::En);
    assert_eq!(
        hint(&panel, Slot::Command).as_deref(),
        Some("claude, found on PATH")
    );
    assert_eq!(
        hint(&panel, Slot::GameTokens).as_deref(),
        Some("20000000 by default")
    );
    assert_eq!(
        hint(&panel, Slot::GameCalls).as_deref(),
        Some("500 by default")
    );
    assert_eq!(hint(&panel, Slot::Effort).as_deref(), Some("the CLI's own"));
    assert_eq!(hint(&panel, Slot::KeyEnv), None);
    assert_eq!(
        panel
            .hint(Spot::Profile(1, Slot::GameCalls), Lang::En)
            .as_deref(),
        Some("no limit")
    );
    assert!(
        panel
            .model_note(0, Lang::En)
            .unwrap()
            .contains("claude:opus")
    );
    assert_eq!(panel.model_note(1, Lang::En), None);
    assert!(panel.price_note(0, Lang::En).contains("no price"));
    assert_eq!(panel.key_variable(0), None, "a CLI reads no key");
    assert_eq!(panel.key_line(0, &|_| true, Lang::En), None);
    assert!(panel.suggestions(0).is_empty());

    // A program is named by its absolute path, and only a CLI runs one.
    type_into(&mut panel, spot(Slot::Command), "claude");
    assert_eq!(
        fault_at(&panel, spot(Slot::Command)),
        Some(Problem::Refused(Why::NotAbsolute))
    );
    type_into(&mut panel, spot(Slot::Command), "/opt/homebrew/bin/claude");
    type_into(&mut panel, spot(Slot::GameCalls), "0");
    assert_eq!(
        fault_at(&panel, spot(Slot::GameCalls)),
        Some(Problem::Refused(Why::Zero))
    );
    let fault = PanelFault {
        spot: spot(Slot::GameCalls),
        problem: Problem::Refused(Why::Zero),
    };
    assert!(panel.say(&fault, Lang::En).contains("calls"));
    type_into(&mut panel, spot(Slot::GameCalls), "300");
    panel.act(Act::Answer(0, Some(AnswerMode::Tools)));
    assert_eq!(
        fault_at(&panel, spot(Slot::Answer)),
        Some(Problem::Refused(Why::NotForCli))
    );
    panel.act(Act::Answer(0, None));
    type_into(&mut panel, Spot::Profile(1, Slot::Command), "/usr/bin/x");
    assert_eq!(
        fault_at(&panel, Spot::Profile(1, Slot::Command)),
        Some(Problem::Refused(Why::CliOnly))
    );
    type_into(&mut panel, Spot::Profile(1, Slot::Command), "");
    assert_eq!(panel.faults(), []);
    let saved = panel.to_save().expect("a CLI profile with nothing wrong");
    let cli = &saved.profiles["a"];
    assert_eq!(cli.provider, Provider::Cli);
    assert_eq!(cli.model, "claude:opus");
    assert_eq!(cli.command.as_deref(), Some("/opt/homebrew/bin/claude"));
    assert_eq!(cli.game_calls, Some(300));
    assert_eq!(SeatSettings::parse(&saved.to_json()), Ok(saved.clone()));
}

/// What an API's profile holds that a CLI does not take stays in view
/// when it becomes one, refused beside its box: here b's dollars a game.
#[test]
fn a_profile_made_a_cli_keeps_what_it_held_in_view_refused() {
    let mut panel = panel();
    panel.act(Act::Provider(1, Provider::Cli));
    type_into(&mut panel, Spot::Profile(1, Slot::Model), "claude");
    assert!(panel.shows(Spot::Profile(1, Slot::GameUsd)));
    assert_eq!(
        fault_at(&panel, Spot::Profile(1, Slot::GameUsd)),
        Some(Problem::Refused(Why::NotForCli))
    );
}

#[test]
fn a_suggestion_puts_a_priced_model_in_the_box() {
    let mut panel = panel();
    let offered = panel.suggestions(1);
    assert_eq!(offered.len(), 3, "{offered:?}");
    assert!(panel.suggestions(2).is_empty(), "none priced for openai");
    let (index, model, price) = offered[2];
    panel.act(Act::Suggest(1, index));
    assert_eq!(panel.to_save().unwrap().profiles["b"].model, model);
    assert_eq!(panel.build_price(1), Some(price));
    assert_eq!(
        SeatPanel::suggestion(model, price, Lang::En),
        "claude-opus-5-5: $4 in, $20 out"
    );
    assert_eq!(
        panel.price_note(1, Lang::En),
        "This build's price: $4 in, $20 out per million tokens. A price given here replaces it."
    );
    assert!(
        panel
            .price_note(2, Lang::De)
            .starts_with("Dieser Build kennt keinen")
    );
}

/// Whether a key's variable is set is asked by its name, and the line
/// names the variable, never a value.
#[test]
fn the_key_variable_is_named_and_its_presence_asked() {
    let panel = panel();
    let set = |name: &str| name == "DEEPSEEK_API_KEY";
    assert_eq!(panel.key_variable(0).as_deref(), Some("ANTHROPIC_API_KEY"));
    assert_eq!(
        panel.key_line(2, &set, Lang::En),
        Some((
            "DEEPSEEK_API_KEY is set in this program's environment.".to_string(),
            true
        ))
    );
    assert!(!panel.key_line(0, &set, Lang::En).unwrap().1);
    let mut panel = panel;
    type_into(&mut panel, Spot::Profile(0, Slot::KeyEnv), KEY);
    assert_eq!(
        panel.key_variable(0),
        None,
        "a key is no variable to ask for"
    );
    type_into(&mut panel, Spot::Profile(0, Slot::KeyEnv), "lower");
    assert_eq!(panel.key_line(0, &set, Lang::En), None);
}

#[test]
fn a_model_with_no_price_is_warned_of_under_dollar_caps() {
    let mut panel = panel();
    assert_eq!(
        panel.warning(2, Lang::En),
        None,
        "it has a price of its own"
    );
    type_into(&mut panel, Spot::Profile(2, Slot::PriceIn), "");
    type_into(&mut panel, Spot::Profile(2, Slot::PriceOut), "");
    assert_eq!(panel.unpriced_period(2), Some(Period::Day));
    assert!(
        panel
            .warning(2, Lang::En)
            .unwrap()
            .contains("daily token limit")
    );
    assert_eq!(panel.faults(), [], "a warning, not a fault");
    type_into(&mut panel, Spot::Cap(CapField::DayTokens), "1_000_000");
    assert_eq!(panel.unpriced_period(2), None);
    assert_eq!(panel.to_save().unwrap().caps.day_tokens, Some(1_000_000));
}

#[test]
fn an_empty_box_says_what_the_bridge_plays_instead() {
    let panel = panel();
    let hint = |slot| panel.hint(Spot::Profile(0, slot), Lang::En);
    assert_eq!(hint(Slot::MaxTokens).as_deref(), Some("16000 by default"));
    assert_eq!(hint(Slot::Effort).as_deref(), Some("medium by default"));
    assert_eq!(hint(Slot::PriceIn).as_deref(), Some("4 by default"));
    assert_eq!(
        panel
            .hint(Spot::Profile(2, Slot::Effort), Lang::En)
            .as_deref(),
        Some("the endpoint's own")
    );
    assert_eq!(
        panel
            .hint(Spot::Profile(2, Slot::BaseUrl), Lang::En)
            .as_deref(),
        Some("BAYLEE_LLM_BASE_URL, else https://api.openai.com/v1")
    );
    assert_eq!(hint(Slot::Model), None);
    assert_eq!(hint(Slot::GameUsd).as_deref(), Some("5 by default"));
    let mut panel = panel;
    type_into(&mut panel, Spot::Profile(2, Slot::PriceIn), "");
    type_into(&mut panel, Spot::Profile(2, Slot::PriceOut), "");
    assert_eq!(
        panel.hint(Spot::Profile(2, Slot::GameUsd), Lang::En),
        None,
        "a model with no price has no dollar limit to default to"
    );
    assert_eq!(panel.hint(Spot::Cap(CapField::DayUsd), Lang::En), None);
}

#[test]
fn money_and_counts_are_written_as_the_language_writes_them() {
    assert_eq!(usd(1234.5, Lang::En), "$1,234.50");
    assert_eq!(usd(1234.5, Lang::De), "1.234,50 $");
    assert_eq!(usd(0.004, Lang::En), "$0.00");
    assert_eq!(rate(2.0, Lang::En), "$2");
    assert_eq!(rate(0.2, Lang::De), "0,20 $");
    assert_eq!(grouped(20_000_000, Lang::En), "20,000,000");
    assert_eq!(grouped(999, Lang::De), "999");
    assert_eq!(grouped(1000, Lang::De), "1.000");
}

#[cfg(not(target_arch = "wasm32"))]
mod on_disk {
    use super::*;
    use crate::llmseat::desk::Desk;
    use crate::llmseat::{FILE, store};
    use std::path::PathBuf;

    /// A directory of this test's own, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("baylee-seatpanel-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const NOW: Moment = Moment {
        unix: 1_790_769_600,
        offset: None,
    };

    #[test]
    fn the_desk_writes_what_holds_and_nothing_with_a_key() {
        let dir = scratch("save");
        let path = dir.join(FILE);
        let mut desk = Desk::open(path.clone(), NOW);
        assert!(desk.panel().is_empty());
        assert_eq!(
            desk.panel().spent().unwrap().unwrap().today.priced.played,
            0
        );

        desk.act(Act::Add);
        desk.panel_mut().edit(|buffer| buffer.insert("sonnet"));
        desk.act(Act::Focus(Spot::Profile(0, Slot::Model)));
        desk.panel_mut().edit(|buffer| buffer.insert(KEY));
        desk.act(Act::Save);
        assert!(!path.exists(), "a key is never written");
        assert_eq!(desk.panel().last_save(), None, "nothing was tried");

        desk.panel_mut().edit(|buffer| {
            buffer.select_all();
            buffer.insert("claude-opus-5-5");
        });
        desk.act(Act::Save);
        assert_eq!(desk.panel().last_save(), Some(&Saved::Written));
        let written = store::load(&path).unwrap().unwrap();
        assert_eq!(written.profiles["sonnet"].model, "claude-opus-5-5");
        assert!(!std::fs::read_to_string(&path).unwrap().contains("AAAABBBB"));
        assert!(!desk.poll(NOW), "its own write is no change on disk");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_desk_reads_again_what_changed_on_disk() {
        let dir = scratch("poll");
        let path = dir.join(FILE);
        store::save(&path, &three()).unwrap();
        let mut desk = Desk::open(path.clone(), NOW);
        assert_eq!(desk.panel().len(), 3);
        assert!(!desk.poll(NOW), "nothing changed");

        // The player edits the file by hand.
        let mut fewer = three();
        fewer.profiles.remove("c");
        store::save(&path, &fewer).unwrap();
        assert!(desk.poll(NOW));
        assert_eq!(desk.panel().len(), 2);

        // A bridge's game writes the book beside it.
        let mut ledger = Ledger::default();
        ledger
            .reserve(
                &Caps::default(),
                &Ask {
                    profile: Some("a"),
                    model: "claude-opus-5-5",
                    game: Budget::Usd(4.0),
                    floor: Budget::Usd(0.25),
                },
                NOW,
            )
            .unwrap();
        std::fs::write(dir.join(crate::llmseat::ledger::FILE), ledger.to_json()).unwrap();
        assert!(desk.poll(NOW));
        let today = desk.panel().spent().unwrap().unwrap().today;
        assert_eq!((today.priced.played, today.priced.open), (1, 1));
        assert!((today.usd - 4.0).abs() < 1e-9);

        // A file broken by hand is shown as refused, and left alone.
        std::fs::write(&path, "{ not json").unwrap();
        assert!(desk.poll(NOW));
        assert!(!desk.panel().editable());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// A preset adds an adapter's profile, protocol and address and key
/// variable filled and every box still editable; the key's entry follows
/// the boxes as typed, and there is none for a CLI or a faulty address.
#[test]
fn a_preset_fills_an_adapter_and_the_key_follows_its_address() {
    let mut panel = SeatPanel::new(Disk::Missing);
    panel.act(Act::AddPreset(Preset::DeepSeekAnthropic));
    assert_eq!(panel.provider(0), Some(Provider::Anthropic));
    let none = |_: &str| None;
    let at = panel.key_entry(0, &none).expect("an entry");
    assert_eq!(at.account(), "DEEPSEEK_API_KEY@api.deepseek.com");
    let saved = panel.to_save().expect("a profile to write");
    assert_eq!(saved.check(), Ok(()));
    assert_eq!(
        saved.profiles["deepseek-anthropic"],
        Preset::DeepSeekAnthropic.profile()
    );
    type_into(
        &mut panel,
        Spot::Profile(0, Slot::BaseUrl),
        "https://llm.example.org",
    );
    assert_eq!(panel.key_entry(0, &none).unwrap().host(), "llm.example.org");
    type_into(
        &mut panel,
        Spot::Profile(0, Slot::BaseUrl),
        "http://llm.example.org",
    );
    assert_eq!(panel.key_entry(0, &none), None, "an address in the clear");
    panel.act(Act::AddPreset(Preset::ClaudeCode));
    assert_eq!(panel.key_entry(1, &none), None, "a CLI reads no key");
    panel.act(Act::AddPreset(Preset::DeepSeekAnthropic));
    assert_eq!(panel.name(2), Some("deepseek-anthropic-2"));
}
