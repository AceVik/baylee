//! The settings sections that are rows of controls (`DESIGN-v5` §12):
//! Graphics, Audio, Display & Interface, Account, Network & Gateway,
//! Language models, Updates, Privacy & Data. Controls and Gameplay, whose
//! rows are lists, are `bindings` and `gameplay`.
//!
//! A row is drawn only where its mechanism exists (principle 5); the map
//! (`baylee_client_core::settings_map`) says which, and in what order.

use baylee_client_core::atmosphere::Atmosphere;
use baylee_client_core::bugreport::{Category, CrashConsent};
use baylee_client_core::cue::Loudness;
use baylee_client_core::graphics::{
    AntiAliasing, Backdrop, BackgroundLimit, DisplayMode, Effects, FrameLimit, Preset, RestLimit,
    VSync,
};
use baylee_client_core::i18n::{Lang, Phrase};
use baylee_client_core::settings_map::{self, Row, Section};
use baylee_client_core::sky::SkyMode;
use baylee_client_core::tableview::{RingLean, VisitCamera};
use bevy::prelude::*;

use crate::lobby::{Press, SettingsPress, SharedPress};
use crate::shellkit::TextSize;
use crate::shellkit::controls::Weight;

use super::View;
use super::rows::{Out, Volume};

fn labels(lang: Lang, phrases: &[Phrase]) -> Vec<&'static str> {
    phrases.iter().map(|p| p.text(lang)).collect()
}

/// The anti-aliasing row's segments, in [`AntiAliasing::ALL`]'s order: the
/// first is a word and so a [`Phrase`], the rest are names of methods.
fn anti_aliasing_names(lang: Lang) -> [&'static str; 4] {
    [Phrase::SwitchOff.text(lang), "FXAA", "2\u{d7}", "4\u{d7}"]
}

fn index<T: PartialEq>(all: &[T], now: &T) -> Option<usize> {
    all.iter().position(|v| v == now)
}

/// Whether the map draws `row` in this build.
fn drawn(view: &View, row: Row) -> bool {
    settings_map::offered(row, view.builds, view.monitors)
}

/// Graphics: the device's rows, then the account's three under a rule.
#[allow(clippy::too_many_lines)] // one section, row after row as the map lists them
pub(crate) fn graphics(out: &mut Out, view: &View) {
    let lang = out.lang;
    let g = view.graphics;
    // The preset, with Custom shown (never pressed) once a knob differs.
    let mut names = labels(
        lang,
        &[
            Phrase::QualityLow,
            Phrase::QualityMedium,
            Phrase::QualityHigh,
            Phrase::QualityUltra,
        ],
    );
    if g.preset == Preset::Custom {
        names.push(Phrase::QualityCustom.text(lang));
    }
    let at = if g.preset == Preset::Custom {
        Some(4)
    } else {
        index(&Preset::NAMED, &g.preset)
    };
    let control = out.seg("preset", &names, at, |i| {
        Press::Settings(SettingsPress::GraphicsPreset(
            Preset::NAMED.get(i).copied().unwrap_or(Preset::Custom),
        ))
    });
    out.row(Row::Preset, control);
    if drawn(view, Row::DisplayMode) {
        let names = labels(
            lang,
            &[
                Phrase::DisplayWindowed,
                Phrase::DisplayBorderless,
                Phrase::DisplayFullscreen,
            ],
        );
        let control = out.seg(
            "display-mode",
            &names,
            index(&DisplayMode::ALL, &g.display_mode),
            |i| Press::Settings(SettingsPress::DisplayMode(DisplayMode::ALL[i])),
        );
        out.row(Row::DisplayMode, control);
        if let Some(left) = view.trial {
            let keep = out.button(
                "keep",
                Phrase::KeepIt.text(lang),
                Weight::Primary,
                Press::Settings(SettingsPress::KeepDisplay(true)),
            );
            let revert = out.button(
                "revert",
                Phrase::RevertIt.text(lang),
                Weight::Secondary,
                Press::Settings(SettingsPress::KeepDisplay(false)),
            );
            let line = out.line(
                &Phrase::KeepDisplayMode.fill(lang, &[&left.to_string()]),
                None,
                &[keep, revert],
            );
            out.commands.entity(line).insert(super::keys::TrialLine);
            out.commands.entity(out.column).add_child(line);
        }
    }
    if drawn(view, Row::Monitor) {
        let names: Vec<String> = (1..=view.monitors).map(|n| n.to_string()).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let control = out.seg("monitor", &refs, Some(0), |_| {
            Press::Shared(SharedPress::PickerNothing)
        });
        out.row(Row::Monitor, control);
    }
    let control = if view.builds.phone {
        out.words(Phrase::AntiAliasingLocked.text(lang), true)
    } else {
        let names = anti_aliasing_names(lang);
        out.seg(
            "anti-aliasing",
            &names,
            index(&AntiAliasing::ALL, &g.anti_aliasing),
            |i| Press::Settings(SettingsPress::AntiAliasing(AntiAliasing::ALL[i])),
        )
    };
    out.row(Row::AntiAliasing, control);
    if drawn(view, Row::VSync) {
        let names = [
            Phrase::SwitchOn.text(lang),
            Phrase::VSyncAdaptive.text(lang),
            Phrase::SwitchOff.text(lang),
        ];
        let control = out.seg("vsync", &names, index(&VSync::ALL, &g.vsync), |i| {
            Press::Settings(SettingsPress::VSync(VSync::ALL[i]))
        });
        out.row(Row::VSync, control);
    }
    if drawn(view, Row::FrameLimit) {
        let names = ["30", "60", "120", Phrase::Unlimited.text(lang)];
        let control = out.seg(
            "frame-limit",
            &names,
            index(&FrameLimit::ALL, &g.frame_limit),
            |i| Press::Settings(SettingsPress::FrameLimit(FrameLimit::ALL[i])),
        );
        out.row(Row::FrameLimit, control);
    }
    if drawn(view, Row::BackgroundLimit) {
        let names = ["5", "15", "30", "60"];
        let control = out.seg(
            "background-limit",
            &names,
            index(&BackgroundLimit::ALL, &g.background_limit),
            |i| Press::Settings(SettingsPress::BackgroundLimit(BackgroundLimit::ALL[i])),
        );
        out.row(Row::BackgroundLimit, control);
    }
    if drawn(view, Row::RestLimit) {
        let names = ["30", "60"];
        let control = out.seg(
            "rest-limit",
            &names,
            index(&RestLimit::ALL, &g.rest_limit),
            |i| Press::Settings(SettingsPress::RestLimit(RestLimit::ALL[i])),
        );
        out.row(Row::RestLimit, control);
    }
    let control = out.toggle(
        "show-frame-rate",
        g.show_frame_rate,
        Press::Settings(SettingsPress::ShowFrameRate),
    );
    out.row(Row::ShowFrameRate, control);
    let names = labels(
        lang,
        &[
            Phrase::BackdropPainting,
            Phrase::BackdropDimmed,
            Phrase::BackdropPlain,
        ],
    );
    let control = out.seg(
        "backdrop",
        &names,
        index(&Backdrop::ALL, &g.backdrop),
        |i| Press::Settings(SettingsPress::Backdrop(Backdrop::ALL[i])),
    );
    out.row(Row::Backdrop, control);
    let names = labels(
        lang,
        &[
            Phrase::AmbientStill,
            Phrase::AmbientSoft,
            Phrase::AmbientFull,
        ],
    );
    let control = out.seg("ambient", &names, index(&Effects::ALL, &g.effects), |i| {
        Press::Settings(SettingsPress::Ambient(Effects::ALL[i]))
    });
    out.row(Row::Ambient, control);
    // The table's shot (DESIGN-v7 D20, D21): this device's, beside the
    // other knobs that are about the screen.
    let table = view.settings.map(|s| s.table).unwrap_or_default();
    let names = labels(lang, &[Phrase::LeanSteep, Phrase::LeanGentle]);
    let control = out.seg(
        "table-lean",
        &names,
        index(&RingLean::ALL, &table.lean),
        |i| Press::Settings(SettingsPress::TableLean(RingLean::ALL[i])),
    );
    out.row(Row::TableLean, control);
    let names = labels(
        lang,
        &[
            Phrase::VisitAuto,
            Phrase::VisitBehind,
            Phrase::VisitBehindDial,
            Phrase::VisitAcross,
        ],
    );
    let control = out.seg(
        "visit-camera",
        &names,
        index(&VisitCamera::ALL, &table.visit),
        |i| Press::Settings(SettingsPress::VisitCamera(VisitCamera::ALL[i])),
    );
    out.row(Row::VisitCamera, control);
    arrangement_rows(out, &table);
    out.caption_rule(Phrase::SettingsAccountRule.text(lang));
    let names = labels(
        lang,
        &[
            Phrase::AtmosphereOff,
            Phrase::AtmosphereSoft,
            Phrase::AtmosphereFull,
        ],
    );
    let control = out.seg(
        "atmosphere",
        &names,
        index(&Atmosphere::ALL, &view.prefs.atmosphere),
        |i| Press::Settings(SettingsPress::PickAtmosphere(Atmosphere::ALL[i])),
    );
    out.row(Row::Atmosphere, control);
    let control = out.toggle(
        "hold-still",
        view.prefs.reduce_motion,
        Press::Settings(SettingsPress::ToggleMotion),
    );
    out.row(Row::HoldStill, control);
    let names = labels(lang, &[Phrase::SkyAuto, Phrase::SkyDay, Phrase::SkyNight]);
    let control = out.seg("sky", &names, index(&SkyMode::ALL, &view.prefs.sky), |i| {
        Press::Settings(SettingsPress::PickSky(SkyMode::ALL[i]))
    });
    out.row(Row::Sky, control);
}

/// The arrangement rows (DESIGN-v8 §2.4): the default as one button per
/// arrangement (an arrangement not built yet is drawn dead), *Tisch folgt
/// dem Zug*, and the per-count memory, a stepper per seat count.
fn arrangement_rows(out: &mut Out, table: &baylee_client_core::tableview::TableView) {
    use baylee_client_core::tableview::{Arrangement, BySeats};
    let lang = out.lang;
    let mut buttons = Vec::new();
    let coming: Vec<String> = Arrangement::ALL
        .iter()
        .map(|a| Phrase::ArrComing.fill(lang, &[a.package()]))
        .collect();
    for (i, arrangement) in Arrangement::ALL.into_iter().enumerate() {
        let live = if arrangement.built() {
            crate::shellkit::controls::Live::Yes
        } else {
            crate::shellkit::controls::Live::No(&coming[i])
        };
        let text = format!("{} {}", arrangement.letter(), arrangement.name().text(lang));
        buttons.push(crate::shellkit::controls::button(
            out.commands,
            out.kit,
            &text,
            // The default stands apart, as a segmented control's choice does.
            if arrangement == table.arrangement {
                crate::shellkit::controls::Weight::Primary
            } else {
                crate::shellkit::controls::Weight::Secondary
            },
            live,
            None,
            (
                Press::Settings(SettingsPress::Arrangement(arrangement)),
                super::rows::item("arrangement", i),
                crate::shellkit::focus::Current(arrangement == table.arrangement),
            ),
        ));
    }
    let wrap = out
        .commands
        .spawn((
            Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: out.kit.m.px(6.0),
                row_gap: out.kit.m.px(6.0),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .add_children(&buttons)
        .id();
    out.block(Row::Arrangement, &[wrap]);
    let control = out.toggle(
        "follow-turn",
        table.follow,
        Press::Settings(SettingsPress::FollowTurn),
    );
    out.row(Row::FollowTurn, control);
    let mut lines = Vec::new();
    for seats in BySeats::FIRST..=BySeats::LAST {
        let shown = table
            .arrangement_by_seats
            .get(seats)
            .map_or(Phrase::ArrAsDefault.text(lang), |a| a.name().text(lang));
        let n = u8::try_from(seats).unwrap_or(u8::MAX);
        let i = (seats - BySeats::FIRST) * 2;
        let stepper = crate::shellkit::controls::stepper(
            out.commands,
            out.kit,
            shown,
            (
                Press::Settings(SettingsPress::ArrangementForSeats(n, -1)),
                super::rows::item("arrangement-seats", i),
            ),
            (
                Press::Settings(SettingsPress::ArrangementForSeats(n, 1)),
                super::rows::item("arrangement-seats", i + 1),
            ),
        );
        let label = out.words(&Phrase::ArrSeats.fill(lang, &[&seats.to_string()]), false);
        let line = out
            .commands
            .spawn((
                Node {
                    align_items: AlignItems::Center,
                    column_gap: out.kit.m.px(12.0),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .add_children(&[label, stepper])
            .id();
        lines.push(line);
    }
    out.block(Row::ArrangementBySeats, &lines);
}

/// A volume, 0 to 1, as a slider's 0 to 100.
fn percent(volume: f32) -> u8 {
    // Clamped into 0..=100 first.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (volume.clamp(0.0, 1.0) * 100.0).round() as u8;
    value
}

/// Audio: one loudness per cue, one rule (S4-4).
pub(crate) fn audio(out: &mut Out, view: &View) {
    let lang = out.lang;
    let (master, effects, music, mute) = view.settings.map_or((1.0, 1.0, 0.5, false), |s| {
        (
            s.audio.master(),
            s.audio.effects(),
            s.music.volume(),
            s.audio.mute_in_background,
        )
    });
    let priority = view.settings.is_none_or(|s| s.audio.priority_cue);
    let control = out.slider("master", percent(master), Volume::Master);
    out.row(Row::Master, control);
    let control = out.slider("music", percent(music), Volume::Music);
    out.row(Row::Music, control);
    let themes = baylee_client_core::music::MusicTheme::ALL;
    let chosen = view
        .settings
        .map_or_else(baylee_client_core::music::MusicTheme::default, |s| {
            s.music.theme()
        });
    let names = labels(
        lang,
        &[
            Phrase::MusicThemeBallad,
            Phrase::MusicThemeDance,
            Phrase::MusicThemeEpic,
            Phrase::MusicThemeJig,
            Phrase::MusicThemeRotating,
        ],
    );
    let control = out.seg("music-theme", &names, index(&themes, &chosen), |i| {
        Press::Settings(SettingsPress::PickMusicTheme(themes[i]))
    });
    out.row(Row::MusicTheme, control);
    let control = out.slider("effects", percent(effects), Volume::Effects);
    out.row(Row::Effects, control);
    let control = out.toggle(
        "mute-unfocused",
        mute,
        Press::Settings(SettingsPress::MuteUnfocused),
    );
    out.row(Row::MuteUnfocused, control);
    let control = out.toggle(
        "priority-cue",
        priority,
        Press::Settings(SettingsPress::PriorityCue),
    );
    out.row(Row::PriorityCue, control);
    out.caption_rule(Phrase::SettingsAccountRule.text(lang));
    let names = labels(
        lang,
        &[Phrase::SoundOff, Phrase::SoundHalf, Phrase::SoundFull],
    );
    let all = [Loudness::Off, Loudness::Half, Loudness::Full];
    let control = out.seg(
        "table-sounds",
        &names,
        index(&all, &view.prefs.sound),
        |i| Press::Settings(SettingsPress::PickSound(all[i])),
    );
    out.row(Row::TableSounds, control);
}

/// Display & Interface: the language, the text size, the preview.
pub(crate) fn display(out: &mut Out, view: &View) {
    let lang = out.lang;
    let names: Vec<&str> = Lang::ALL.iter().map(|l| l.name()).collect();
    let control = out.seg("language", &names, index(&Lang::ALL, &lang), |i| {
        Press::Shared(SharedPress::PickLang(Lang::ALL[i]))
    });
    out.row(Row::Language, control);
    let step = view
        .settings
        .map_or_else(TextSize::default, |s| s.text_size);
    let names = ["XS", "S", "M", "L", "XL"];
    let control = out.seg("text-size", &names, index(&TextSize::ALL, &step), |i| {
        Press::Settings(SettingsPress::TextSize(TextSize::ALL[i]))
    });
    out.row(Row::TextSize, control);
    let scale = view.settings.map_or(1.0, |s| s.preview_scale);
    let shown = format!("{} %", percent(scale / 2.0) * 2);
    let control = crate::shellkit::controls::stepper(
        out.commands,
        out.kit,
        &shown,
        (
            Press::Settings(SettingsPress::PreviewSize(-1)),
            super::rows::item("preview-size", 0),
        ),
        (
            Press::Settings(SettingsPress::PreviewSize(1)),
            super::rows::item("preview-size", 1),
        ),
    );
    out.row(Row::PreviewSize, control);
    let on = view.settings.is_some_and(|s| s.prefer_text_view);
    let control = out.toggle("text-face", on, Press::Settings(SettingsPress::TextFace));
    out.row(Row::TextFace, control);
}

/// Account: the handle, signing out, deleting the account.
pub(crate) fn account(out: &mut Out, view: &View) {
    let lang = out.lang;
    let handle = view
        .state
        .lobby
        .me()
        .map(|me| me.handle.clone())
        .unwrap_or_default();
    let copy = out.button(
        "copy-handle",
        Phrase::CopyHandle.text(lang),
        Weight::Secondary,
        Press::Settings(SettingsPress::CopyHandle),
    );
    let shown = out.words(&handle, false);
    let both = out.wrap(&[shown, copy]);
    out.row(Row::Handle, both);
    let sign_out = out.button(
        "sign-out",
        Phrase::RowSignOut.text(lang),
        Weight::Secondary,
        Press::Header(crate::lobby::HeaderPress::SignOut),
    );
    out.row(Row::SignOut, sign_out);
    let delete = out.button(
        "delete-account",
        Phrase::DeleteAccount.text(lang),
        Weight::Danger,
        Press::Settings(SettingsPress::AskToDeleteAccount),
    );
    out.row(Row::DeleteAccount, delete);
}

/// Network & Gateway: where, which version, the way back, diagnostics.
pub(crate) fn network(out: &mut Out, view: &View) {
    let lang = out.lang;
    let state = view.state;
    let facts = if state.lobby.offline() {
        Phrase::NetworkOffline.text(lang).to_string()
    } else {
        crate::lobby::gateway_facts(state, lang)
    };
    let shown = out.words(&facts, false);
    out.row(Row::Gateway, shown);
    let switch = out.button(
        "switch-gateway",
        Phrase::ShellSwitchGateway.text(lang),
        Weight::Secondary,
        Press::Settings(SettingsPress::SwitchGateway),
    );
    out.row(Row::SwitchGateway, switch);
    let copy = out.button(
        "copy-diagnostics",
        Phrase::CopyAsText.text(lang),
        Weight::Secondary,
        Press::Settings(SettingsPress::CopyDiagnostics),
    );
    let text = out.words(&crate::lobby::diagnostics(state), true);
    let both = out.wrap(&[text, copy]);
    out.row(Row::Diagnostics, both);
}

/// Privacy & Data: the true list of what is kept and sent, report consent,
/// crash reports, the report form.
pub(crate) fn privacy(out: &mut Out, view: &View) {
    let lang = out.lang;
    let empty = out.commands.spawn((Node::default(), Pickable::IGNORE)).id();
    out.row(Row::WhatIsKept, empty);
    let consent = view.settings.map(|s| s.reports.clone()).unwrap_or_default();
    let mut lines = Vec::new();
    for (i, category) in Category::ALL.into_iter().enumerate() {
        let toggle = crate::shellkit::controls::toggle(
            out.commands,
            out.kit,
            consent.allows(category),
            (
                Press::Settings(SettingsPress::Consent(category)),
                super::rows::item("consent", i),
            ),
        );
        let line = out.line(
            category.phrase().text(lang),
            Some(category.hint().text(lang)),
            &[toggle],
        );
        lines.push(line);
    }
    out.block(Row::ReportConsent, &lines);
    let names = labels(
        lang,
        &[Phrase::CrashAsk, Phrase::CrashSend, Phrase::CrashNever],
    );
    let all = [
        CrashConsent::Unasked,
        CrashConsent::Send,
        CrashConsent::Never,
    ];
    let control = out.seg("crash", &names, index(&all, &consent.crashes), |i| {
        Press::Settings(SettingsPress::Crash(all[i]))
    });
    out.row(Row::CrashReports, control);
    let report = out.button(
        "report-problem",
        Phrase::ReportOpen.text(lang),
        Weight::Secondary,
        Press::Header(crate::lobby::HeaderPress::Report),
    );
    out.row(Row::ReportProblem, report);
}

/// Language models: this machine's seat profiles, or one line where the
/// build has no seat.
pub(crate) fn models(out: &mut Out, view: &View) {
    let lang = out.lang;
    if !crate::seatpanel::DESKTOP {
        let line = out.words(Phrase::LanguageModelsElsewhere.text(lang), true);
        out.commands.entity(out.column).add_child(line);
        return;
    }
    let empty = out.commands.spawn((Node::default(), Pickable::IGNORE)).id();
    out.row(Row::Profiles, empty);
    crate::seatpanel::draw(
        out.commands,
        out.column,
        &view.state.seat,
        lang,
        out.kit.fonts,
        view.metrics,
    );
}

/// Updates (desktop builds): the updater's own controls.
pub(crate) fn updates(out: &mut Out, view: &View) {
    let lang = out.lang;
    // The updater draws its own switches (`update::controls`), the
    // `CheckAutomatically` row's among them; a settings row here as well
    // said the same words over an empty control (beta.6 QA).
    if let Some(controls) = crate::update::controls(out.commands, out.kit.fonts, view.metrics, lang)
    {
        out.commands.entity(out.column).add_child(controls);
    }
}

/// The section's rows.
pub(crate) fn draw(section: Section, out: &mut Out, view: &View) {
    match section {
        Section::Graphics => graphics(out, view),
        Section::Audio => audio(out, view),
        Section::Display => display(out, view),
        Section::Controls => super::bindings::controls(out, view),
        Section::Gameplay => super::gameplay::gameplay(out, view),
        Section::Account => account(out, view),
        Section::Network => network(out, view),
        Section::LanguageModels => models(out, view),
        Section::Updates => updates(out, view),
        Section::Privacy => privacy(out, view),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row's first segment is a word, and words are `Phrase`s: it read
    /// "Off" in a German Grafik section beside the `VSync` row's "aus"
    /// (beta.6 QA).
    #[test]
    fn the_anti_aliasing_segments_speak_the_interface_language() {
        for lang in [Lang::En, Lang::De] {
            assert_eq!(
                anti_aliasing_names(lang)[0],
                Phrase::SwitchOff.text(lang),
                "{lang:?}"
            );
        }
        assert_eq!(anti_aliasing_names(Lang::De).len(), AntiAliasing::ALL.len());
    }
}
