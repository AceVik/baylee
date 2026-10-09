use super::*;
use crate::i18n::Lang;

fn chars(text: &str) -> usize {
    text.chars().count()
}

/// TOURS.md §1.6: every step is one paragraph under the phone bubble's
/// budget, every title one line, in both languages.
#[test]
fn every_step_fits_the_phone_bubble() {
    let mut over = Vec::new();
    for (tour, step) in every_step() {
        for lang in Lang::ALL {
            let body = step.body.text(lang);
            let title = step.title.text(lang);
            let budget = if lang == Lang::De { BODY_DE } else { BODY_EN };
            if chars(body) > budget {
                over.push(format!(
                    "{tour:?} {} body {lang:?}: {}",
                    step.id,
                    chars(body)
                ));
            }
            if chars(title) > TITLE {
                over.push(format!(
                    "{tour:?} {} title {lang:?}: {}",
                    step.id,
                    chars(title)
                ));
            }
            assert!(
                !body.contains('\n') && !title.contains('\n'),
                "{} is one paragraph",
                step.id
            );
            assert!(
                !body.contains("{0}") && !body.contains("{1}"),
                "{} leaves a placeholder unfilled",
                step.id
            );
        }
    }
    assert!(over.is_empty(), "over the bubble's budget: {over:#?}");
}

/// The tripwire of TOURS.md §3.5: a tour names this client's things, it
/// does not explain the rules. Cheap, not a proof; the review is.
#[test]
fn no_tour_sentence_explains_the_rules() {
    let deny = [
        "CR ",
        "Regel ",
        "rule ",
        "Priorität bedeutet",
        "priority means",
        "In Magic",
    ];
    for phrase in every_phrase() {
        for lang in Lang::ALL {
            let text = phrase.text(lang);
            for word in deny {
                assert!(!text.contains(word), "{phrase:?} says {word:?} in {lang:?}");
            }
        }
    }
    assert!(
        every_phrase().len() > 60,
        "the tripwire reads the tour's phrases"
    );
}

/// The tripwire fires on what it denies (so a passing run means something).
#[test]
fn the_tripwire_would_fire() {
    let planted = "In Magic, priority means the right to act.";
    assert!(
        ["priority means", "In Magic"]
            .iter()
            .any(|w| planted.contains(w))
    );
}

#[test]
fn the_scripts_count_what_the_spec_counts() {
    let count = |tour: Tour| -> usize { tour.chapters().iter().map(|c| c.steps.len()).sum() };
    assert_eq!(count(Tour::Lobby), 14);
    assert_eq!(LOBBY.len(), 5);
    assert_eq!(
        count(Tour::Builder),
        14,
        "D1–D14; the phone's D11b waits for tours on phones"
    );
    assert_eq!(BUILDER.len(), 4);
    assert_eq!(count(Tour::Table), 34);
    assert_eq!(TABLE.len(), 12, "11 chapters and the closing");
    let mut ids: Vec<&str> = every_step().iter().map(|(_, s)| s.id).collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "a step id names one step");
}

#[test]
fn a_device_without_the_field_reads_every_tour_on() {
    let tours: Tours = serde_json::from_str("{}").unwrap();
    assert_eq!(tours, Tours::default());
    assert!(tours.lobby && tours.builder && tours.table && tours.tips);
    let mut seen = Tours::default();
    seen.seen.insert(mark(Tour::Lobby, "door"));
    seen.tips = false;
    let back: Tours = serde_json::from_str(&serde_json::to_string(&seen).unwrap()).unwrap();
    assert_eq!(back, seen);
}

#[test]
fn a_lobby_chapter_runs_through_and_is_seen() {
    let mut tours = Tours::default();
    let at = tours.due(Tour::Lobby, Place::Door).unwrap();
    let mut run = Run::chapter(Tour::Lobby, at, false, 0).unwrap();
    assert_eq!(run.current().id, "L1");
    assert!(!run.has_back());
    assert_eq!(run.next(&mut tours), Moved::Step);
    assert!(run.has_back());
    run.back();
    assert_eq!(run.current().id, "L1");
    run.next(&mut tours);
    assert_eq!(run.next(&mut tours), Moved::Step);
    assert!(run.last());
    assert_eq!(run.next(&mut tours), Moved::Over);
    assert!(tours.seen.contains("lobby/door"));
    assert_eq!(tours.due(Tour::Lobby, Place::Door), None);
    // Play opens before Create, and the closing only after the room.
    assert_eq!(tours.due(Tour::Lobby, Place::Play), Some(1));
    tours.seen.insert(mark(Tour::Lobby, "play"));
    assert_eq!(tours.due(Tour::Lobby, Place::Play), Some(2));
    tours.seen.insert(mark(Tour::Lobby, "create"));
    assert_eq!(tours.due(Tour::Lobby, Place::Play), None);
    tours.seen.insert(mark(Tour::Lobby, "room"));
    assert_eq!(tours.due(Tour::Lobby, Place::Play), Some(4));
}

#[test]
fn a_try_it_step_lets_go_and_lights_its_primary_once_done() {
    let mut tours = Tours::default();
    tours.seen.insert(mark(Tour::Lobby, "play"));
    let at = tours.due(Tour::Lobby, Place::Play).unwrap();
    let mut run = Run::chapter(Tour::Lobby, at, false, 0).unwrap();
    assert_eq!(run.current().id, "L8");
    assert_eq!(run.mode, Mode::Try);
    assert!(!run.holds_keyboard());
    assert!(!run.primary_live());
    run.hold();
    assert!(run.primary_live());
    run.next(&mut tours);
    assert_eq!(run.current().id, "L9");
    assert!(!run.primary_live(), "the latch is the step's");
}

#[test]
fn the_switches_and_the_box() {
    let mut tours = Tours::default();
    tours.set_all(false);
    assert_eq!(tours.due(Tour::Lobby, Place::Door), None);
    assert!(!tours.tips);
    tours.set(Tour::Lobby, true);
    assert!(tours.due(Tour::Lobby, Place::Door).is_some());
    tours.seen.insert("lobby/door".into());
    tours.restart();
    assert!(tours.seen.is_empty());
}

/// TOURS.md §1.5's invariant, by construction: a narrated step folds itself
/// on the frame a question is pending for this seat, and comes back when it
/// is answered — unless the player folded it by hand meanwhile.
#[test]
fn no_narrated_step_stands_over_a_pending_question() {
    let mut run = Run::chapter(Tour::Table, 0, false, 4).unwrap();
    for _ in 0..40 {
        run.follow_question(true);
        assert_ne!(
            run.mode,
            Mode::Narrated,
            "{} stands over a question",
            run.current().id
        );
        run.follow_question(false);
        if run.current().kind.holds() {
            assert_eq!(run.mode, Mode::Narrated, "{} comes back", run.current().id);
        }
        if run.next(&mut Tours::default()) == Moved::Over {
            break;
        }
    }
    // Folded by hand while it was folded for the question: it stays.
    let mut run = Run::chapter(Tour::Table, 0, false, 4).unwrap();
    run.follow_question(true);
    run.fold();
    run.follow_question(false);
    assert_eq!(run.mode, Mode::Folded);
}

#[test]
fn the_table_runs_on_through_its_chapters_and_skips_what_is_not_here() {
    let mut tours = Tours::default();
    let mut run = Run::chapter(Tour::Table, 0, false, 2).unwrap();
    let mut walked = Vec::new();
    loop {
        walked.push(run.current().id);
        if run.next(&mut tours) == Moved::Over {
            break;
        }
    }
    // JIT steps wait for their anchors; ring steps skip at two; the phone's
    // step skips on a desk; Reporting comes before Keys.
    for absent in ["T9", "T10", "T11", "T13", "T14", "T25", "T21", "T22", "T24"] {
        assert!(!walked.contains(&absent), "{absent} walked");
    }
    let at = |id| walked.iter().position(|w| *w == id).unwrap();
    assert!(at("T33") < at("T28"));
    assert_eq!(walked.last(), Some(&"T34"));
    assert!(TABLE.iter().all(|c| tours.chapter_seen(Tour::Table, c)));
    // Skip on a continuous tour goes on with the next chapter.
    let mut run = Run::chapter(Tour::Table, 0, false, 4).unwrap();
    assert_eq!(run.skip(&mut Tours::default()), Moved::Step);
    assert_eq!(run.current().id, "T2");
}

/// A try-it step whose check comes true moves on by itself: no press
/// between doing the thing and the next step, for every check the lobby
/// tour waits on.
#[test]
fn a_satisfied_try_it_step_moves_on_without_a_press() {
    let mut tours = Tours::default();
    tours.seen.insert(mark(Tour::Lobby, "play"));
    let at = tours.due(Tour::Lobby, Place::Play).unwrap();
    let mut run = Run::chapter(Tour::Lobby, at, false, 0).unwrap();
    assert_eq!(run.current().kind, Kind::Try(Check::CreateSheetOpen));
    assert_eq!(run.satisfied(&mut tours), Moved::Step);
    assert_eq!(run.current().id, "L9", "the sheet open is L9 at once");
    assert_eq!(run.current().kind, Kind::Try(Check::InRoom));
    assert_eq!(run.satisfied(&mut tours), Moved::Over);
    assert!(
        tours.seen.contains("lobby/create"),
        "the room is the chapter's end"
    );
    // Every try-it step of every tour behaves alike.
    for (tour, step) in every_step() {
        if !matches!(step.kind, Kind::Try(_)) {
            continue;
        }
        let (c, s) = tour
            .chapters()
            .iter()
            .enumerate()
            .find_map(|(c, ch)| {
                ch.steps
                    .iter()
                    .position(|x| x.id == step.id)
                    .map(|s| (c, s))
            })
            .unwrap();
        let mut run = Run::jit(tour, c, s, false);
        run.single = false;
        run.mode = Mode::Try;
        let before = (run.chapter, run.step);
        let moved = run.satisfied(&mut Tours::default());
        assert!(
            moved == Moved::Over || (run.chapter, run.step) != before,
            "{} stays after its check came true",
            step.id
        );
    }
}

/// Desktop only (owner, 09.10.): under touch or on a phone nothing starts.
#[test]
fn no_tour_step_starts_on_a_phone_or_under_touch() {
    assert!(offered_here(false, false));
    for (phone, touch) in [(true, false), (false, true), (true, true)] {
        assert!(!offered_here(phone, touch), "{phone} {touch}");
    }
}
