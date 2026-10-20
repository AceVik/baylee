use super::*;
use baylee_client_core::interaction::Outcome;

/// Runs the frame that starts the cues' synthesis (unless it ran), waits
/// for every take, then the frame that collects them into [`Voices`].
///
/// It waits on the tasks themselves, as long as they take, and never on
/// the wall clock: the pool is the process's, every test here starts its
/// own synthesis on it, and under a loaded run (the whole client suite
/// beside a gate) a 60-s deadline ran out while the takes were still
/// being computed — two of these tests failed that way and passed alone.
fn voiced(app: &mut App) {
    if !app.world().contains_resource::<Voicing>() && !app.world().contains_resource::<Voices>() {
        app.update();
    }
    if let Some(mut voicing) = app.world_mut().get_resource_mut::<Voicing>() {
        for voiced in voicing.0.iter_mut().flat_map(|(_, takes)| takes.iter_mut()) {
            if let Voiced::Pending(task) = voiced {
                let done = bevy::tasks::block_on(task);
                *voiced = Voiced::Done(done);
            }
        }
    }
    app.update();
    assert!(
        app.world().contains_resource::<Voices>(),
        "the cues were never voiced"
    );
}

/// The frame the app opens on does not synthesise the cues: it only
/// starts them. Red on the old `voice_the_cues`, which put a finished
/// [`Voices`] in that frame — 3.7 s of a Windows desktop's first frame,
/// with the window standing white meanwhile. Without the collector the
/// buffers never arrive; with it they do, and they are every one.
#[test]
fn the_first_frame_does_not_wait_for_the_cues() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .add_systems(Startup, voice_the_cues);
    app.update();
    assert!(
        !app.world().contains_resource::<Voices>(),
        "the first frame synthesised the cues itself"
    );
    assert!(
        app.world().contains_resource::<Voicing>(),
        "nor started them"
    );
    app.add_systems(Update, collect_the_voices);
    voiced(&mut app);
    assert!(
        !app.world().contains_resource::<Voicing>(),
        "collected once"
    );
    let wanted: usize = takes().iter().map(|(_, takes)| takes.len()).sum();
    let got: usize = {
        let voices = app.world().resource::<Voices>();
        voices.voices.iter().map(|(_, handles)| handles.len()).sum()
    };
    assert_eq!(got, wanted, "a buffer went missing on the way");
}

/// What [`watch`] saw on the frame that just ran.
///
/// "Did this frame mark the duel dirty" can only be asked from *inside*
/// the frame. `App::update` ends with `World::clear_trackers`, which moves
/// `last_change_tick` past every write the update made, so a `Ref<Duel>`
/// taken afterwards answers `false` whatever the systems did — the first
/// draft of the counter-test below asked from outside and passed with the
/// early return deleted.
#[derive(Resource, Default)]
struct Dirtied(bool);

/// Runs after [`play_the_cues`] and records whether it moved the tick.
fn watch(duel: Res<Duel>, mut dirtied: ResMut<Dirtied>) {
    dirtied.0 = duel.is_changed();
}

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Duel>()
        .init_resource::<Dirtied>()
        .add_systems(Update, (play_the_cues, watch).chain());
    app
}

/// The drain is wired, which is the thing worth saying about a sink:
/// "decided but never drained" is a queue that grows for the length of a
/// game.
#[test]
fn a_decided_cue_is_drained_by_the_frame_after_it() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<Duel>()
        .cues
        .note_ending(Outcome::YouLost);
    assert_eq!(
        app.world().resource::<Duel>().cues.pending(),
        [Beat::once(Cue::GameLost)]
    );
    app.update();
    let duel = app.world().resource::<Duel>();
    assert!(duel.cues.pending().is_empty(), "the queue was drained");
    assert_eq!(
        duel.cues.last(),
        Some(Beat::once(Cue::GameLost)),
        "and remembered"
    );
}

/// The counter-test for the early return: a frame with nothing to say
/// must not report the duel as having changed, or every reader that
/// watches the resource rebuilds on every frame of a quiet game.
///
/// The second half is the counter-test's own counter-test. A watcher that
/// can never see dirt would pass the first assertion however wrong the
/// system was, so the cue goes in through `bypass_change_detection` —
/// leaving [`play_the_cues`] as the only thing that can have moved the
/// tick on the frame after it.
#[test]
fn a_silent_frame_does_not_touch_the_duel() {
    let mut app = app();
    // `init_resource` marked it; that is not a frame's doing.
    app.update();
    app.update();
    assert!(
        !app.world().resource::<Dirtied>().0,
        "a frame with no cues in it marked the whole duel dirty"
    );

    app.world_mut()
        .resource_mut::<Duel>()
        .bypass_change_detection()
        .cues
        .note_refusal();
    app.update();
    assert!(
        app.world().resource::<Dirtied>().0,
        "the watcher never sees dirt, so the assertion above proves nothing"
    );
}

/// A client with no device still decides, drains and remembers.
///
/// The headless case, and the reason both resources are `Option` in the
/// system's signature.
#[test]
fn a_client_with_no_device_still_drains() {
    let mut app = app();
    app.world_mut().resource_mut::<Duel>().cues.note_refusal();
    app.update();
    let duel = app.world().resource::<Duel>();
    assert!(duel.cues.pending().is_empty());
    assert_eq!(duel.cues.last(), Some(Beat::once(Cue::Refused)));
}

/// Every cue is synthesised, **at every count it can carry**.
///
/// Run against the real startup system in a real asset world, because
/// what can go wrong here is a missing *arm* rather than wrong
/// arithmetic: [`RECIPES`] is eleven entries and [`Cue::ALL`] is fifteen,
/// three of which are several buffers each. A count with no buffer is a
/// draw of four that plays nothing at all — silent at runtime and loud in
/// no test.
#[test]
fn every_cue_has_a_voice() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .add_systems(Startup, voice_the_cues)
        .add_systems(Update, collect_the_voices);
    voiced(&mut app);
    let voices = app.world().resource::<Voices>();
    let wanted: usize = Cue::ALL.iter().map(|cue| cue.counts().count()).sum();
    assert_eq!(voices.len(), wanted, "a cue with no sound");
    for cue in Cue::ALL {
        for count in cue.counts() {
            let beat = Beat::of(cue, count);
            assert!(
                voices.pick(beat).is_some(),
                "{} has no voice at {count}",
                cue.name()
            );
        }
    }
}

/// A count past a cue's ceiling is played as the ceiling, not as silence.
///
/// [`Beat::of`] clamps, so this is really a test that nothing between the
/// model and the sink goes round it — a `Beat` built by hand with a twelve
/// in it would find no buffer and drop the sound at exactly the moment
/// there was most to hear.
#[test]
fn drawing_a_whole_deck_sounds_like_a_handful() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .add_systems(Startup, voice_the_cues)
        .add_systems(Update, collect_the_voices);
    voiced(&mut app);
    let voices = app.world().resource::<Voices>();
    let most = Beat::of(Cue::CardDrawn, Cue::CardDrawn.most());
    assert_eq!(Beat::of(Cue::CardDrawn, 60), most, "a Windfall is a hand");
    assert!(voices.pick(most).is_some());
}

/// A burst is one blow per thing that happened, and the run accelerates.
///
/// The arithmetic nothing else can see: [`burst`] builds the strikes and
/// [`render`] turns them into samples, so a ladder indexed wrongly or a
/// gap that failed to accumulate would come out as a buffer that still
/// peaks correctly, still renders twice the same, and is simply the wrong
/// sound.
#[test]
fn a_burst_is_one_blow_per_thing_that_happened() {
    for cue in Cue::ALL.iter().filter(|cue| cue.most() > 1) {
        let per = usize::from(*cue != Cue::CardDrawn) + 1;
        for count in cue.counts() {
            for recipe in burst(*cue, count) {
                assert_eq!(
                    recipe.strikes.len(),
                    per * count as usize,
                    "{} at {count} is the wrong number of blows",
                    cue.name()
                );
            }
        }
    }
    let three = &burst(Cue::CardDrawn, 3)[0];
    let (first, second) = (
        three.strikes[1].at - three.strikes[0].at,
        three.strikes[2].at - three.strikes[1].at,
    );
    assert!(first > second, "the burst does not accelerate");
    assert!(second > 0.080, "the last gap fuses at {second}");
}

/// A counter gesture's two blows are one event, and the second says which
/// way.
///
/// Both halves matter and both are easy to get backwards. Inside the
/// ~50 ms grouping window the pair is heard as one thing, which is what
/// makes three creatures three and not six; and the *destination* is the
/// louder blow, because that is the note carrying the direction.
#[test]
fn a_counter_is_one_gesture_that_lands_somewhere() {
    for (cue, up) in [(Cue::CreatureGrew, true), (Cue::CreatureShrank, false)] {
        let solo = &burst(cue, 1)[0];
        let (first, second) = (&solo.strikes[0], &solo.strikes[1]);
        let apart = second.at - first.at;
        assert!(
            (0.020..=0.050).contains(&apart),
            "{} is {apart}s apart: one event needs under 50 ms and a \
                 direction needs over 20",
            cue.name()
        );
        assert!(second.gain > first.gain, "the grace note is the loud one");
        assert_eq!(second.hz > first.hz, up, "{} lands wrong", cue.name());
    }
}

/// The room does not invert a counter gesture.
///
/// This module has been bitten twice by a delay network having a *pitch* —
/// a comb notch that turned a falling third written 0.80 then 1.00 into
/// 0.92 then 0.70 — and both times at a pitch the tap set had not been
/// searched over. [`note::CS5`] and [`note::D5`] are new and were not in
/// that search, so the dynamic written in [`counters`] is checked *after*
/// the room rather than trusted before it: the destination has to still be
/// the louder blow once the reflections are on it.
#[test]
fn the_room_leaves_the_gesture_pointing_where_it_was_aimed() {
    for cue in [Cue::CreatureGrew, Cue::CreatureShrank] {
        let solo = &burst(cue, 1)[0];
        let samples = render(solo);
        let loudest = |from: f32, to: f32| {
            let a = ((from * RATE) as usize * 2).min(samples.len());
            let b = ((to * RATE) as usize * 2).min(samples.len());
            samples[a..b].iter().fold(0.0_f32, |m, s| m.max(s.abs()))
        };
        let grace = loudest(0.0, FLAM);
        let lands = loudest(FLAM, FLAM + 0.060);
        assert!(
            lands > grace,
            "{}: the room made the grace note ({grace:.3}) the loud one \
                 against the destination ({lands:.3})",
            cue.name()
        );
    }
}

/// Thirty-seven buffers, thirty-seven different sounds.
///
/// A recipe copied and not edited is the easy mistake in a table this
/// shape, and it is invisible: two cues that sound alike are a client
/// saying two different things the same way. Compared as bytes, because
/// the whole point of computing them is that they are reproducible.
#[test]
fn no_two_cues_sound_alike() {
    let mut rendered: Vec<Vec<u8>> = all_the_sounds()
        .iter()
        .map(|(_, recipe)| wav(&render(recipe)))
        .collect();
    let before = rendered.len();
    rendered.sort_unstable();
    rendered.dedup();
    assert_eq!(before, rendered.len(), "two cues render the same bytes");
}

/// And the six taps are six sounds, which is the whole of what keeps
/// [`Cue::YourMove`] from being a metronome.
#[test]
fn no_two_taps_sound_alike() {
    let mut rendered: Vec<Vec<u8>> = VARIANTS
        .iter()
        .map(|&(cents, spot)| wav(&render(&tap(detuned(cents), spot))))
        .collect();
    let before = rendered.len();
    rendered.sort_unstable();
    rendered.dedup();
    assert_eq!(before, rendered.len(), "two taps render the same bytes");
}

/// Rendering twice gives the same bytes.
///
/// The property the whole approach rests on: a generated sound that
/// depended on a clock or on an unseeded RNG would be different on every
/// run, and `no_two_cues_sound_alike` would pass while saying nothing.
/// The mallet's noise is the only thing here that could break it.
#[test]
fn a_cue_renders_the_same_bytes_twice() {
    for (cue, recipe) in all_the_sounds() {
        assert_eq!(
            wav(&render(&recipe)),
            wav(&render(&recipe)),
            "{} is not reproducible",
            cue.name()
        );
    }
}

/// Nothing clips, and nothing is silent either.
///
/// Bounded on **both** sides deliberately: a one-sided "quiet enough"
/// assertion is what let a felt authored four times too dark through, and
/// the same mistake in a sink is a cue that plays and cannot be heard.
#[test]
fn every_cue_is_loud_enough_to_hear_and_quiet_enough_not_to_clip() {
    for (cue, recipe) in all_the_sounds() {
        let samples = render(&recipe);
        let peak = samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
        let want = recipe.peak * MASTER;
        assert!(
            (peak - want).abs() < 0.01,
            "{} peaks at {peak}, asked for {want}",
            cue.name()
        );
        assert!(peak <= 1.0, "{} clips at {peak}", cue.name());
        assert!(peak > 0.1, "{} is inaudible at {peak}", cue.name());
    }
}

/// The two loudest things that can happen at once still fit.
///
/// The two loudest *life* cues on one frame are what [`MASTER`] was
/// chosen against: a change at this seat and one elsewhere, which is the
/// combat damage step of any multiplayer turn. Written here so that a
/// change to either peak fails rather than merely distorts.
///
/// It is **not** the loudest frame there is — see
/// [`an_ending_silences_the_life_cues_under_it`] for the one that is, and
/// for what stops it.
#[test]
fn two_life_cues_on_one_frame_do_not_clip() {
    let together = once(Cue::MyLifeLost) + once(Cue::TheirLifeLost);
    assert!(together < 1.0, "two cues sum to {together}");
}

/// One of a cue, at what it is normalised to after [`MASTER`].
fn once(cue: Cue) -> f32 {
    peak_of(Beat::once(cue))
}

/// A lethal hit is three cues on one frame, and three do not fit.
///
/// The frame a game ends on carries the damage that ended it: a life loss
/// here, a life loss there, and the ending. 0.70 + 0.28 + 0.63 is 1.61,
/// which clips — at the loudest moment of the game, where a listener is
/// least likely to forgive it. So an ending is played **alone**: it is
/// the only thing on that frame anybody is listening for, and the life
/// change that caused it is already drawn on two bars.
#[test]
fn an_ending_silences_the_life_cues_under_it() {
    let lethal = [Cue::MyLifeLost, Cue::TheirLifeLost, Cue::GameWon].map(Beat::once);
    let raw: f32 = lethal.iter().map(|&beat| peak_of(beat)).sum();
    assert!(raw > 1.0, "the three would not have clipped anyway");
    let kept = audible(&lethal);
    assert_eq!(
        kept,
        vec![Beat::once(Cue::GameWon)],
        "the ending is not alone"
    );
    let together: f32 = kept.iter().map(|&beat| peak_of(beat)).sum();
    assert!(together < 1.0, "the ending clips at {together}");
}

/// And an ordinary frame is left exactly as it was.
#[test]
fn a_frame_with_no_ending_keeps_every_cue() {
    let pair = [Cue::MyLifeLost, Cue::TheirLifeLost].map(Beat::once);
    assert_eq!(audible(&pair), pair.to_vec());
}

/// Every cue's peak is where the module says it is.
///
/// [`peak_of`] is read by [`audible`] and has three branches — the recipe
/// table, the tap, and the bursts — because the three are stored three
/// different ways. A branch that answered zero would make its cue free,
/// so the frame would admit it *and* everything under it, and the clip
/// would arrive at the loudest moment rather than the quietest.
#[test]
fn every_cue_can_say_how_loud_it_is() {
    for cue in Cue::ALL {
        for count in cue.counts() {
            let loud = peak_of(Beat::of(cue, count));
            assert!(
                loud > 0.1 && loud <= 1.0,
                "{} at {count} claims {loud}",
                cue.name()
            );
        }
    }
}

/// The frames a real game makes, and which of them still fit.
///
/// The bend [`audible`] documents, held as numbers. A *Sign in Blood* is
/// a life loss and a draw; an infect combat is life and a shrink; a
/// *Fathom Mage* is a counter and a draw — none of those existed when
/// [`MASTER`] was chosen against exactly two cues. Each is checked to
/// **fit**, because a budget that dropped them would be a quieter client
/// than the owner asked for; and the lethal combat frame is checked to be
/// **cut**, because that is what a budget is for.
#[test]
fn a_frame_admits_what_it_can_hold_and_drops_the_rest() {
    let fits = |cues: &[Cue]| {
        let frame: Vec<Beat> = cues.iter().map(|&cue| Beat::once(cue)).collect();
        audible(&frame).len() == frame.len()
    };
    assert!(fits(&[Cue::MyLifeLost, Cue::CardDrawn]), "Sign in Blood");
    assert!(fits(&[Cue::MyLifeLost, Cue::CreatureShrank]), "infect");
    assert!(fits(&[Cue::CreatureGrew, Cue::CardDrawn]), "Fathom Mage");
    assert!(fits(&[Cue::CardDrawn, Cue::YourMove]), "an ordinary turn");

    // A whole multiplayer combat: both life cues, a shrink, a draw and
    // the nudge. 0.70 + 0.28 + 0.25 + 0.15 + 0.15 is 1.53.
    let combat = [
        Cue::MyLifeLost,
        Cue::TheirLifeLost,
        Cue::CreatureShrank,
        Cue::CardDrawn,
        Cue::YourMove,
    ]
    .map(Beat::once);
    let raw: f32 = combat.iter().map(|&beat| peak_of(beat)).sum();
    assert!(raw > 1.0, "the five would not have clipped anyway");
    let kept = audible(&combat);
    let together: f32 = kept.iter().map(|&beat| peak_of(beat)).sum();
    assert!(together <= 1.0, "the frame still clips at {together}");
    assert!(!kept.is_empty(), "a busy frame went silent");
    // The ranking, not merely the sum: what survives a full frame is the
    // life a player must not miss, never the texture under it.
    assert!(
        kept.contains(&Beat::once(Cue::MyLifeLost)),
        "the loudest thing that happened was dropped"
    );
}

/// The header says what the bytes are.
///
/// Forty-four bytes written by hand, so what can be wrong is a *field*,
/// and a wrong field is a sound that plays at the wrong speed or does not
/// decode at all — which on a device is indistinguishable from silence.
#[test]
fn the_riff_header_describes_the_samples_that_follow() {
    let samples = render(&RECIPES[0].1);
    let bytes = wav(&samples);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[36..40], b"data");
    let declared = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
    assert_eq!(
        declared as usize,
        samples.len() * 2,
        "the data chunk lies about its length"
    );
    assert_eq!(bytes.len(), 44 + samples.len() * 2, "trailing bytes");
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    assert_eq!(rate, 44_100, "the wrong sample rate is the wrong pitch");
    assert_eq!(
        u16::from_le_bytes([bytes[22], bytes[23]]),
        2,
        "not stereo, so the two ears would be read as one at double speed"
    );
    // The three fields that have to agree with the channel count, and the
    // reason this test names all of them: a stereo file with a mono block
    // align decodes at half speed with the ears interleaved, which is a
    // failure that *plays* — the worst kind to leave to the ear.
    assert_eq!(
        u16::from_le_bytes([bytes[32], bytes[33]]),
        4,
        "block align is not two channels of sixteen bits"
    );
    assert_eq!(
        u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
        44_100 * 4,
        "byte rate does not match the block align"
    );
    assert_eq!(samples.len() % 2, 0, "an odd sample count is half a frame");
}

/// Both ears hear the same sound, and only the room differs.
///
/// The design's one rule about stereo, held as a number. A [`Cue`] names
/// a moment and never a seat — `Their…` is deduplicated across every seat
/// that lost life on one view — so a pan would be a position the game
/// cannot back up. What is decorrelated is the *room*: two different sets
/// of reflections behind one dry signal, which the ear reads as space
/// rather than as direction.
///
/// What is bounded is **mono compatibility**, in two numbers, and what is
/// deliberately *not* bounded is how much the two channels differ.
///
/// A side-to-mid ratio would be the obvious check and it is the wrong
/// one: a long reverberant tail carries far more energy than a stopped
/// note, so a genuinely distant cue is more than half side energy while
/// still being perfectly centred — `TheirLifeLost` sits at 0.51 and is
/// right to. What must hold is that the two ears **agree**: a positive
/// correlation, so nothing is inverted or hard-panned, and a mono sum
/// that keeps almost all of its peak, so a phone speaker loses reverb and
/// not the sound.
///
/// The first draft of the room failed both: two different early-reflection
/// sets measured −0.99 correlated at A3, which is a cue that all but
/// disappears the moment anyone sums it.
#[test]
fn the_two_ears_agree_about_where_the_sound_is() {
    for (cue, recipe) in all_the_sounds() {
        let samples = render(&recipe);
        let (mid, side) = samples
            .as_chunks::<2>()
            .0
            .iter()
            .fold((0.0, 0.0), |(m, s), f| {
                (m + (f[0] + f[1]).powi(2), s + (f[0] - f[1]).powi(2))
            });
        assert!(side > 0.0, "{} is the same in both ears", cue.name());
        let corr = (mid - side) / (mid + side);
        assert!(
            corr > 0.25,
            "{} has ears that disagree: correlation {corr:.2}",
            cue.name()
        );

        let loudest = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        let summed = samples
            .as_chunks::<2>()
            .0
            .iter()
            .fold(0.0_f32, |m, f| m.max((f32::midpoint(f[0], f[1])).abs()));
        assert!(
            summed > 0.85 * loudest,
            "{} loses {:.0}% of its peak in mono",
            cue.name(),
            100.0 * (1.0 - summed / loudest)
        );
    }
}

/// No mode above the fundamental outlives a tenth of a second.
///
/// The bell guard, and the reason it is a test rather than a comment: the
/// decay law is `τ₁ · r^-1.2`, so a *longer* ring lengthens every partial
/// with it, and the endings ring four times as long as a life cue. An
/// ending whose 9.56 partial rang for 90 ms would be a struck bell, which
/// is exactly what this module's header says it is not — and the clamp
/// that stops it is one `.min()` easy to drop while tuning.
#[test]
fn nothing_above_the_fundamental_rings_like_a_bell() {
    for (cue, recipe) in all_the_sounds() {
        for blow in recipe.strikes {
            let ring = free_ring(blow.hz) * blow.damp;
            for mode in modes(blow.hz, blow.contact, blow.spot, ring).iter().skip(1) {
                assert!(
                    mode.tau <= 0.090,
                    "{}: {:.0} Hz rings {:.0} ms",
                    cue.name(),
                    mode.hz,
                    mode.tau * 1000.0
                );
            }
        }
    }
}

/// A buffer ends at silence.
///
/// The fade is what makes this true; without it the tail is wherever the
/// exponential had reached, and the step to zero is a click — the one
/// artefact a listener always notices and never blames on the right
/// thing.
#[test]
fn no_cue_ends_on_a_step() {
    for (cue, recipe) in all_the_sounds() {
        let samples = render(&recipe);
        let last = samples.last().copied().unwrap_or_default();
        assert!(
            last.abs() < 0.001,
            "{} ends at {last}, which is a click",
            cue.name()
        );
    }
}

/// The taps are asked for in order and the order wraps.
///
/// The cycle is the anti-metronome measure, and "played the same variant
/// every time" is a bug that a listener would hear long before a test
/// would — unless the test is this one.
#[test]
fn consecutive_taps_are_different_buffers() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .add_systems(Startup, voice_the_cues)
        .add_systems(Update, collect_the_voices);
    voiced(&mut app);
    let mut voices = app.world_mut().resource_mut::<Voices>();
    let mut seen = Vec::new();
    for _ in 0..VARIANTS.len() * 2 {
        seen.push(
            voices
                .pick(Beat::once(Cue::YourMove))
                .cloned()
                .expect("a tap"),
        );
        voices.played += 1;
    }
    for pair in seen.windows(2) {
        assert_ne!(pair[0], pair[1], "two taps in a row from one buffer");
    }
    assert_eq!(seen[0], seen[VARIANTS.len()], "the cycle does not wrap");
}

/// Writes every cue to `BAYLEE_CUE_DUMP` as a `.wav`, and hears nothing.
///
/// `#[ignore]`d, because it writes files and asserts almost nothing. It
/// exists because a *generated* sound has no other audit surface: every
/// other decision in this client can be read off a screenshot or a
/// number, and this one can only be listened to. `measure, do not
/// theorise` applies to the ear as well.
///
/// ```text
/// BAYLEE_CUE_DUMP=/tmp/cues cargo test -p baylee-client --lib \
///     -- --ignored every_cue_written_out
/// ```
#[test]
#[ignore = "writes files; run it when you want to hear them"]
fn every_cue_written_out() {
    let Ok(dir) = std::env::var("BAYLEE_CUE_DUMP") else {
        panic!("set BAYLEE_CUE_DUMP to a directory");
    };
    std::fs::create_dir_all(&dir).expect("a directory");
    for (cue, recipe) in &RECIPES {
        let path = format!("{dir}/{}.wav", cue.name());
        std::fs::write(&path, wav(&render(recipe))).expect("written");
    }
    for (i, &(cents, spot)) in VARIANTS.iter().enumerate() {
        let path = format!("{dir}/YourMove-{i}.wav");
        std::fs::write(&path, wav(&render(&tap(detuned(cents), spot)))).expect("written");
    }
    // The counted three, every count and every variant — which is most of
    // what there is to listen *to* now, and all of what the design's own
    // perceptual claims rest on: that seven taps at these gaps can be
    // counted, and that a 45 ms flam reads as a direction rather than as
    // one thickened note. Neither is a thing a test can assert.
    for cue in Cue::ALL.into_iter().filter(|cue| cue.most() > 1) {
        for count in cue.counts() {
            for (i, recipe) in burst(cue, count).iter().enumerate() {
                let path = format!("{dir}/{}-{count}-{i}.wav", cue.name());
                std::fs::write(&path, wav(&render(recipe))).expect("written");
            }
        }
    }
}

/// Every buffer the client will ever synthesise, with the cue it belongs
/// to.
///
/// [`RECIPES`] used to be the whole set and is now eight of twenty-six:
/// six taps and the counted three's bursts are built by functions rather
/// than written into a table. Every test below that says "every cue" has
/// to mean this, or the tests that hold the *instrument* together — the
/// bell guard, the clip bound, the ends-at-silence rule, the two ears —
/// would all be silently ignoring the newest and most frequent sounds in
/// the set, which is exactly the shape of a test that passes and proves
/// nothing.
fn all_the_sounds() -> Vec<(Cue, Recipe)> {
    let mut out: Vec<(Cue, Recipe)> = Vec::new();
    for (cue, recipe) in RECIPES {
        out.push((cue, recipe));
    }
    for (cents, spot) in VARIANTS {
        out.push((Cue::YourMove, tap(detuned(cents), spot)));
    }
    for cue in Cue::ALL.into_iter().filter(|cue| cue.most() > 1) {
        for count in cue.counts() {
            out.extend(burst(cue, count).into_iter().map(|recipe| (cue, recipe)));
        }
    }
    out
}

/// WT4: the priority cue is the firmer strike, peaking at 0.26 after
/// [`MASTER`] (DESIGN-v7 §4.4), struck with the soft mallet.
#[test]
fn your_move_is_the_firmer_strike() {
    let peak = peak_of(Beat::once(Cue::YourMove));
    assert!((peak - 0.26).abs() < 0.01, "YourMove peaks at {peak}");
    for (cents, spot) in VARIANTS {
        let recipe = tap(detuned(cents), spot);
        let drawn = render(&recipe).iter().fold(0.0f32, |a, s| a.max(s.abs()));
        assert!((drawn - 0.26).abs() < 0.01, "a variant peaks at {drawn}");
        assert!(
            recipe
                .strikes
                .iter()
                .all(|s| (s.contact - mallet::SOFT).abs() < 1e-6)
        );
    }
}

/// The turn's click peaks where [`peak_of`] says it does, so the frame's
/// budget ([`audible`]) counts it honestly.
#[test]
fn the_turn_click_peaks_where_it_says() {
    let drawn = compass_samples().iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(
        (drawn - TURN_CLICK_PEAK).abs() < 0.01,
        "the click peaks at {drawn}, the budget says {TURN_CLICK_PEAK}"
    );
}

/// An app that voices and plays the cues under these device settings.
fn sink(settings: crate::settings::ClientSettings) -> App {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .init_resource::<Duel>()
        .init_resource::<Prefs>()
        .insert_resource(settings)
        .add_systems(Startup, voice_the_cues)
        .add_systems(Update, (collect_the_voices, play_the_cues).chain());
    voiced(&mut app);
    app
}

fn players(app: &mut App) -> usize {
    app.world_mut()
        .query::<&AudioPlayer>()
        .iter(app.world())
        .count()
}

/// WT4: the priority cue's own switch (D22). Off, the cue is decided and
/// reported and nothing plays; the cycle still moves.
#[test]
fn the_priority_cue_switched_off_is_decided_and_not_played() {
    let mut settings = crate::settings::ClientSettings::default();
    settings.audio.priority_cue = false;
    let mut app = sink(settings);
    app.world_mut()
        .resource_mut::<Duel>()
        .cues
        .note_question(true);
    app.update();
    let last = app.world().resource::<Duel>().cues.last().map(|b| b.cue);
    assert_eq!(last, Some(Cue::YourMove), "decided and reported");
    assert_eq!(players(&mut app), 0, "and nothing played");
    assert_eq!(app.world().resource::<Voices>().played, 1);
    // Every other cue still plays under the switch.
    app.world_mut().resource_mut::<Duel>().cues.note_refusal();
    app.update();
    assert_eq!(players(&mut app), 1, "the switch is one cue wide");
}

/// WT4: the music's switch never gates the priority cue.
#[test]
fn the_priority_cue_plays_with_the_music_muted() {
    let mut settings = crate::settings::ClientSettings::default();
    settings.music.set_muted(true);
    let mut app = sink(settings);
    app.world_mut()
        .resource_mut::<Duel>()
        .cues
        .note_question(true);
    app.update();
    assert_eq!(
        players(&mut app),
        1,
        "muted music silenced the priority cue"
    );
}

/// The turn's click goes through the sink now, under this device's game
/// volume: at a game volume of zero it is silent, which the compass's
/// own player never was (DESIGN-v7 §4.1's bug).
#[test]
fn the_turn_click_obeys_the_devices_volume() {
    let mut settings = crate::settings::ClientSettings::default();
    settings.audio.set_effects(0.0);
    let mut app = sink(settings);
    let mut duel = app.world_mut().resource_mut::<Duel>();
    duel.cues.note_turn(baylee_core::ids::PlayerId::new(0), 1);
    duel.cues.note_turn(baylee_core::ids::PlayerId::new(1), 2);
    app.update();
    let last = app.world().resource::<Duel>().cues.last().map(|b| b.cue);
    assert_eq!(last, Some(Cue::TurnPassed));
    assert_eq!(players(&mut app), 0, "the click ignored the game volume");
    let mut app = sink(crate::settings::ClientSettings::default());
    let mut duel = app.world_mut().resource_mut::<Duel>();
    duel.cues.note_turn(baylee_core::ids::PlayerId::new(0), 1);
    duel.cues.note_turn(baylee_core::ids::PlayerId::new(1), 2);
    app.update();
    assert_eq!(players(&mut app), 1, "and at full volume it plays");
}

/// A [`Prefs`] with the sound turned off.
///
/// Built through `Prefs::default` and one field, because the resource
/// owns its own storage slot and a test that constructed it by hand would
/// be asserting against a second spelling of it.
fn muted() -> Prefs {
    let mut prefs = Prefs::default();
    prefs.edit().sound = Loudness::Off;
    prefs
}

/// A muted client plays nothing and still moves the cycle.
///
/// Two claims in one test because they are the same decision: `Off` is a
/// playback setting and not a second kind of silence, so the model above
/// it goes on exactly as it did.
#[test]
fn off_spawns_nothing_and_still_advances() {
    let mut app = App::new();
    app.add_plugins(bevy::asset::AssetPlugin::default())
        .init_asset::<AudioSource>()
        .init_resource::<Duel>()
        .insert_resource(muted())
        .add_systems(Startup, voice_the_cues)
        .add_systems(Update, (collect_the_voices, play_the_cues).chain());
    voiced(&mut app);
    app.world_mut().resource_mut::<Duel>().cues.note_refusal();
    app.update();
    assert_eq!(
        app.world().resource::<Voices>().played,
        1,
        "a muted cue did not move the cycle"
    );
    let players = app
        .world_mut()
        .query::<&AudioPlayer>()
        .iter(app.world())
        .count();
    assert_eq!(players, 0, "something was played while muted");
}
