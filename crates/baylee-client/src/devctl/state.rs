//! `/state`: everything the client believes, as one JSON dump — the view,
//! the interaction, every drawn card's box, the buttons, the shelves.

#[allow(clippy::wildcard_imports)] // the harness's own vocabulary
use super::*;

/// Everything `/state` reads, in one parameter.
///
/// A bundle rather than six more arguments on [`pump`], which is already at
/// bevy's sixteen-parameter ceiling — and the grouping is honest: these are
/// the things the client *believes*, as against the input queues and the
/// clock around them.
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Believed<'w, 's> {
    /// The lobby's rebuild count (`shell_nodes_json`'s companion).
    rebuilds: Option<Res<'w, crate::lobby::UiRebuilds>>,
    /// The report form, for its own redraw count beside the lobby's.
    report: Option<Res<'w, crate::report::ReportDesk>>,
    /// The builder's virtual lists: rows found visible before they were
    /// mounted, and rows mounted (§17 WP4: "0 placeholders").
    lists: Option<Res<'w, crate::buildui::virtual_rows::ListProbe>>,
    /// The lobby, for the builder's keyboard place.
    lobby: Option<Res<'w, crate::lobby::LobbyState>>,
    /// The report form's buttons, where a pointer would press them.
    desk_controls: Query<
        'w,
        's,
        (
            &'static crate::report::DeskPress,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    /// The lobby tree's root, and every node under it, for `shell_nodes`.
    shell_roots: Query<'w, 's, Entity, With<crate::lobby::LobbyRoot>>,
    /// The kit's gallery, dumped after the lobby with `"r":"gallery"` on
    /// its root.
    gallery_roots: Query<'w, 's, Entity, With<crate::shellkit::gallery::GalleryRoot>>,
    #[allow(clippy::type_complexity)] // one row of a tree walk
    shell_nodes: Query<
        'w,
        's,
        (
            Option<&'static Text>,
            Option<&'static TextSpan>,
            Option<&'static ComputedNode>,
            Option<&'static UiGlobalTransform>,
            Option<&'static Children>,
            bevy::ecs::query::Has<crate::lobby::Press>,
            Option<&'static crate::shellkit::Role>,
            Option<&'static bevy::text::TextLayoutInfo>,
            Option<&'static InheritedVisibility>,
        ),
    >,
    legal_text: Query<
        'w,
        's,
        (
            &'static Text,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    music_controls: Query<
        'w,
        's,
        (
            &'static crate::music::MusicAction,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
    >,
    duel: Option<Res<'w, Duel>>,
    settings: Option<Res<'w, ClientSettings>>,
    /// The music's last request: what the drivers decided (`score`).
    score: Option<Res<'w, crate::music::Heard>>,
    /// The card text the sheet draws its rows from, so an ability row can be
    /// reported as it reads.
    texts: Option<Res<'w, crate::cardtext::CardTexts>>,
    /// Which screen the client is on, which nothing here could say before.
    ///
    /// The sharper half of what #135 cost a reporter. *"The state was never
    /// entered"* and *"the state was entered and the input was ignored"* are
    /// different bugs with different fixes, and a probe that cannot separate
    /// them turns whoever is holding it into a guesser — #135's own honest
    /// limit, *"I did not photograph whatever was up during those 280
    /// seconds"*, was forced by this absence and not by the reporter.
    ///
    /// Never `null`: there is always a phase, so a missing one would be a
    /// fault in the probe rather than an answer from it.
    phase: Option<Res<'w, State<crate::DuelPhase>>>,
    journey: Option<Res<'w, crate::arrival::Journey>>,
    real_time: Option<Res<'w, Time<Real>>>,
    /// How many interface text faces have been built (WP6): a number that
    /// moves on every frame is a face rebuilt per frame.
    face_builds: Option<Res<'w, crate::face::FaceBuilds>>,
    /// The shell keyboard (WP0b-2): where focus stands, what the resolver
    /// read and answered, the stack it read it against, the `?` overlay.
    #[allow(clippy::type_complexity)] // four optional resources, read together
    shell_keys: (
        Option<Res<'w, crate::shellkit::focus::FocusReport>>,
        Option<Res<'w, crate::shellkit::keys::ShellLog>>,
        Option<Res<'w, crate::shellkit::keys::ShellStack>>,
        Option<Res<'w, crate::shellkit::overlay::Overlay>>,
    ),
    /// Every way out of a finished game, and which of them the keyboard can
    /// see.
    ///
    /// `Press` is the component both readers of the end screen's buttons
    /// agree on; `DuelExit` is the marker only one of them filters by
    /// (`lobby::systems::leave_keys`), while `leave_clicks` walks the clicked
    /// entity's ancestry instead. Reporting the pair is what lets a caller
    /// tell a control the keyboard cannot reach from one that is simply
    /// absent — the live question in #135, and one no count could answer.
    exits: Query<
        'w,
        's,
        (
            Entity,
            &'static crate::lobby::Press,
            Option<&'static crate::lobby::DuelExit>,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    /// Cards playing their way off the table; only the count is reported.
    leaving: Query<'w, 's, &'static crate::table::Departing>,
    shelves: Option<Res<'w, crate::hud::Shelves>>,
    /// Where the camera stood at the end of the last frame, which is the
    /// camera the last rendered frame was drawn with — so a rect measured
    /// here answers for the picture a `/screenshot` would return.
    rig: Option<Res<'w, crate::table::ShownRig>>,
    /// The camera's target rig and its pose (DESIGN-v7 §2.7): what
    /// `/state.camera` reports beside the shown rig above.
    camera: (
        Option<Res<'w, crate::table::CameraRig>>,
        Option<Res<'w, crate::table::CameraPose>>,
    ),
    /// What the dial shows (DESIGN-v7 §2.7): the two hands, the arcs, the
    /// hub's pulse and the turn number's drawn size.
    dial: Option<Res<'w, crate::dial::DialReport>>,
    /// The arrangement switcher's measure of the table (DESIGN-v8 §2.4),
    /// the cards' glide, and the pill and the menu as drawn.
    #[allow(clippy::type_complexity)] // four readings of one switcher
    arrangement: (
        Option<Res<'w, crate::arrangement::ArrangementFrame>>,
        Option<Res<'w, crate::table::GlideReport>>,
        Query<
            'w,
            's,
            (&'static ComputedNode, &'static UiGlobalTransform),
            With<crate::arrangement::ArrangementPill>,
        >,
        Query<
            'w,
            's,
            (&'static ComputedNode, &'static UiGlobalTransform),
            With<crate::arrangement::ArrangementPanel>,
        >,
        Query<'w, 's, (&'static crate::table::SeatMat, &'static Transform)>,
    ),
    /// The two top-edge lines (turn, priority) on the strip's seat buttons
    /// and the seats' plates, and whether each shows (shown and inked: the
    /// chip's turn line fades rather than hides); then the chips and the plates themselves:
    /// where each plate is drawn, what its crown, ∞ and pool line say, and
    /// every control's [`crate::hud::Hint`] (its name in words); and where
    /// each seat's steps are drawn, with their names over or under them.
    #[allow(clippy::type_complexity)] // six readings of the seats' surfaces
    chips: (
        Query<
            'w,
            's,
            (
                &'static crate::hud::ChipTag,
                &'static Visibility,
                &'static BackgroundColor,
            ),
        >,
        Query<
            'w,
            's,
            (
                &'static crate::hud::PlateTab,
                &'static Node,
                &'static ComputedNode,
                &'static UiGlobalTransform,
                Option<&'static crate::hud::Hint>,
            ),
        >,
        Query<
            'w,
            's,
            (
                &'static crate::hud::PlayerTab,
                &'static crate::hud::Hint,
                &'static ComputedNode,
                &'static UiGlobalTransform,
            ),
        >,
        Query<'w, 's, &'static crate::hud::PlateMark>,
        Query<'w, 's, &'static crate::hud::ChipCrown>,
        Query<
            'w,
            's,
            (
                &'static crate::hud::SeatBar,
                &'static crate::hud::seatbar::attached::Panel,
                &'static Node,
                &'static ComputedNode,
                &'static UiGlobalTransform,
            ),
        >,
    ),
    /// Every card on the table, with the transform `glide` has it at right
    /// now rather than the one it is heading for, and whether it is drawn:
    /// a scrolled row does not draw the cards outside the run it shows.
    cards: Query<
        'w,
        's,
        (
            &'static crate::table::CardVisual,
            &'static Transform,
            &'static Visibility,
        ),
    >,
    /// Every card standing in the player's own hand row.
    ///
    /// A different kind of thing entirely — the hand is `bevy_ui` and the
    /// table is a 3D scene — but the same question is being asked of it, and
    /// a caller that has to find a hand card by eye is no better off for the
    /// table's cards being free. `HandRowCard` and not the wider
    /// `HandCardVisual`, which the stack panel also puts on its slots.
    hand: Query<
        'w,
        's,
        (
            &'static crate::hud::HandCardVisual,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::HandRowCard>,
    >,
    /// Every row of the stack panel.
    ///
    /// The third zone, and it was missing for the same reason the hand once
    /// was: a driver could read `interaction.pending` and see a
    /// `ChooseTargets` naming object 200, and then had no way on earth to
    /// find object 200 on the screen — the stack is neither a card on the
    /// felt nor a card in the hand, and it is where every "target spell"
    /// lives. `StackRowCard` and not the wider `HandCardVisual` for the
    /// reason the hand gives above: the panel puts that one on target chips
    /// too, and a chip is a picture *of* an object elsewhere.
    stack: Query<
        'w,
        's,
        (
            &'static crate::hud::HandCardVisual,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::StackRowCard>,
    >,
    /// The prompt bar's answers, and the two choosers under it.
    ///
    /// Added for the same reason and by the same road as the cards: a
    /// shockland asked `PayLifeOrEnterTapped` and there was no way to answer
    /// it. `PromptAction::Yes` and `No` are reachable *only* through a
    /// pointer click — no key binding fires either — so a harness that cannot
    /// find the button cannot get past the question at all.
    prompts: Query<
        'w,
        's,
        (
            &'static crate::hud::PromptButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    abilities: Query<
        'w,
        's,
        (
            &'static crate::hud::AbilityButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    choices: Query<
        'w,
        's,
        (
            &'static crate::hud::ChoiceButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    tray_cards: Query<
        'w,
        's,
        (
            &'static crate::hud::TrayCard,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
    tray_filters: Query<
        'w,
        's,
        (
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::TrayFilter>,
    >,
    tray_none: Query<
        'w,
        's,
        (
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
        With<crate::hud::TrayNone>,
    >,
    /// Everything a player can press that is not an answer: the concession,
    /// the draw offer, the armed card's two halves and "resolve the stack".
    ///
    /// Left out until the shelf put one of them in the row of answers, where
    /// a harness reading only `prompt` rows sees a gap between two buttons and
    /// nothing in it. They were always worth having — a concession has no key
    /// at all, and the way to end a driven game was to close the window.
    menus: Query<
        'w,
        's,
        (
            &'static crate::hud::MenuButton,
            &'static bevy::ui::ComputedNode,
            &'static bevy::ui::UiGlobalTransform,
        ),
    >,
}

/// Every mana source the client can see, and what it believes each one makes.
///
/// The half of the hand's indigo offer that leaves no other trace.
/// `"reachable": 3` says three cards could be paid for by tapping; it does
/// not say what the client thinks it may tap, so a disagreement between the
/// engine's enumeration and the client's reading of it is invisible from
/// outside — and both of those have been the answer at different times.
///
/// Built through [`crate::manasources::sources`] and not by a second
/// reading, or the endpoint would photograph a list nothing acts on.
fn sources_json(duel: &Duel) -> String {
    let Some(view) = duel.view.as_ref() else {
        return "null".to_string();
    };
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        // Not an empty list: no priority question is standing, so the client
        // is not offering anything and has not decided that it cannot.
        return "null".to_string();
    };
    let rows: Vec<String> = crate::manasources::sources(view, legal)
        .into_iter()
        .map(|source| {
            let colors: Vec<String> = source
                .colors
                .iter()
                .map(|color| quoted(&format!("{color:?}")))
                .collect();
            format!(
                "{{\"object\":{},\"tap\":{},\"colors\":[{}],\"amount\":{}}}",
                source.id.slot(),
                quoted(&format!("{:?}", source.tap)),
                colors.join(","),
                source.amount
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// Every way out of a finished game, and which of them the keyboard sees.
///
/// `null` outside [`crate::DuelPhase::Finished`] and **not** `[]`: no end
/// screen is standing, so the question is being asked at the wrong moment,
/// and "nothing to report" and "not applicable" are different answers that a
/// bare empty list would merge. Inside `Finished` an empty list is itself
/// worth reading — it says the sheet has no way out at all.
///
/// `duel_exit` is the point of the row rather than a detail of it. The two
/// readers of these buttons do not ask the same question:
///
/// ```text
/// leave_clicks: presses: Query<&Press>                  // walks the ancestry
/// leave_keys:   exits:   Query<&Press, With<DuelExit>>  // filtered
/// ```
///
/// so a working click proves only that a `Press` sits *somewhere* in the
/// clicked entity's lineage, and says nothing about whether the filtered
/// query finds one. A list of ways out that did not carry this would be a
/// longer list answering the same unanswerable question.
fn exits_json(believed: &Believed) -> String {
    let finished = believed
        .phase
        .as_ref()
        .is_some_and(|phase| *phase.get() == crate::DuelPhase::Finished);
    if !finished {
        return "null".to_string();
    }
    lobby_controls_json(believed)
}

/// How often the lobby rebuilt its tree, by cause (`lobby::UiRebuilds`).
///
/// `null` outside the lobby plugin. The counter is monotonic: a caller
/// measures a span by reading it twice.
/// `/state.shell`: the shell keyboard's state (`KEYBOARD.md` §9.1 asks the
/// probe for the focused control, its context stack, `focus_visible`, open
/// sheets and the actions fired).
fn shell_keys_json(believed: &Believed) -> String {
    let (focus, log, stack, overlay) = &believed.shell_keys;
    let focus = focus.as_deref().copied().unwrap_or_default();
    let stop = focus.stop.map_or_else(
        || "null".to_string(),
        |s| {
            format!(
                "{{\"table\":\"{}\",\"id\":\"{}\",\"item\":{}}}",
                s.table, s.id, s.item
            )
        },
    );
    let fired = log.as_deref().map_or_else(String::new, |l| {
        l.fired
            .iter()
            .map(|a| format!("\"{}\"", a.name()))
            .collect::<Vec<_>>()
            .join(",")
    });
    let resolved = log.as_deref().map_or(0, |l| l.resolved);
    let stack = stack.as_deref().copied().unwrap_or_default();
    let screen = stack
        .stack
        .screen
        .map_or_else(|| "null".to_string(), |c| format!("\"{c:?}\""));
    format!(
        "{{\"table\":{},\"focus\":{stop},\"focus_visible\":{},\"field\":{},         \"overlay\":{},\"live\":{},\"screen\":{screen},\"modal\":{},\"menu\":{},         \"typing\":{},\"resolved\":{resolved},\"fired\":[{fired}],\"text_size\":{}}}",
        focus
            .table
            .map_or_else(|| "null".to_string(), |t| format!("\"{t}\"")),
        focus.visible,
        focus.field,
        overlay.as_deref().is_some_and(|o| o.open),
        stack.live,
        stack.stack.modal,
        stack.stack.menu,
        stack.stack.field,
        believed
            .settings
            .as_deref()
            .map_or(crate::shellkit::TextSize::default().step(), |s| s
                .text_size
                .step()),
    )
}

fn rebuilds_json(believed: &Believed) -> String {
    let report = believed.report.as_deref().map_or(0, |desk| desk.redraws);
    believed.rebuilds.as_deref().map_or_else(
        || "null".to_string(),
        |r| {
            let lists = believed.lists.as_deref().copied().unwrap_or_default();
            let nav = believed
                .lobby
                .as_deref()
                .map_or_else(String::new, |s| format!("{:?}", s.build.nav));
            format!(
                "{{\"total\":{},\"patches\":{},\"sections\":{},\"state\":{},\"prefs\":{},\
                 \"cast\":{},\"frame\":{},\"report\":{report},\"placeholders\":{},\
                 \"mounted\":{},\"nav\":{}}}",
                r.total,
                r.patches,
                r.sections,
                r.state,
                r.prefs,
                r.cast,
                r.frame,
                lists.placeholders,
                lists.mounted,
                quoted(&nav)
            )
        },
    )
}

/// The report form's buttons in logical pixels, as `lobby_controls` lists
/// the lobby's.
fn desk_controls_json(believed: &Believed) -> String {
    let rows: Vec<String> = believed
        .desk_controls
        .iter()
        .map(|(press, node, place)| {
            let scale = node.inverse_scale_factor;
            let size = node.size() * scale;
            let mid = place.translation * scale;
            format!(
                "{{\"press\":{press},\"at_x\":{x:.1},\"at_y\":{y:.1},\"w\":{w:.1},\"h\":{h:.1}}}",
                press = quoted(&format!("{press:?}")),
                x = mid.x,
                y = mid.y,
                w = size.x,
                h = size.y,
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// Every node of the lobby's tree, in tree order: depth, rect in logical
/// pixels, the text it draws (spans joined) and whether it is pressable.
///
/// The before-image a behaviour-free refactor of the lobby is held to (the
/// shell design's `WP0a`: "node dump identical"), so it carries nothing that
/// may legitimately change under such a refactor — no entity index, no
/// `Press` spelling — and nothing a player typed into a masked field: a
/// `Masked` field draws dots, and the dots are what is reported.
///
/// A node the shell's kit made also says what it is (`"k"`, a
/// `shellkit::Role`), which is what the shell's checks read
/// (`scripts/shell/check.py`): a button's label against its budget, a hit
/// wrapper against 44 × 44, a panel's rect as the ground for contrast.
fn shell_nodes_json(believed: &Believed) -> String {
    fn walk(
        believed: &Believed,
        entity: Entity,
        depth: usize,
        root: Option<&str>,
        out: &mut Vec<String>,
    ) {
        let Ok((text, _, node, place, children, pressable, role, laid, shown)) =
            believed.shell_nodes.get(entity)
        else {
            return;
        };
        let (Some(node), Some(place)) = (node, place) else {
            return;
        };
        let scale = node.inverse_scale_factor;
        let size = node.size() * scale;
        let mid = place.translation * scale;
        let mut row = format!(
            "{{\"d\":{depth},\"x\":{x:.1},\"y\":{y:.1},\"w\":{w:.1},\"h\":{h:.1}",
            x = mid.x - size.x / 2.0,
            y = mid.y - size.y / 2.0,
            w = size.x,
            h = size.y,
        );
        if let Some(text) = text {
            let mut said = text.0.clone();
            for span in children.into_iter().flatten() {
                if let Ok((_, Some(span), ..)) = believed.shell_nodes.get(*span) {
                    said.push_str(&span.0);
                }
            }
            row.push_str(",\"t\":");
            row.push_str(&quoted(&said));
            // What the glyphs measure, beside the box they were given: a
            // label wider than its box spills or is cut, which the box's
            // own rect cannot show (`check.py`'s `fit`), and how many lines
            // it broke into (`lines`: one inside a control).
            if let Some(laid) = laid {
                let extent = laid.size * scale;
                let lines = laid
                    .glyphs
                    .iter()
                    .map(|g| g.line_index + 1)
                    .max()
                    .unwrap_or(0);
                row.push_str(&format!(
                    ",\"tw\":{:.1},\"th\":{:.1},\"ln\":{lines}",
                    extent.x, extent.y
                ));
            }
        }
        if shown.is_some_and(|v| !v.get()) {
            row.push_str(",\"hid\":true");
        }
        if pressable {
            row.push_str(",\"i\":true");
        }
        if let Some(role) = role {
            row.push_str(",\"k\":");
            row.push_str(&quoted(role.name()));
        }
        if let Some(root) = root {
            row.push_str(",\"r\":");
            row.push_str(&quoted(root));
        }
        row.push('}');
        out.push(row);
        for child in children.into_iter().flatten() {
            walk(believed, *child, depth + 1, None, out);
        }
    }
    let mut out = Vec::new();
    for root in &believed.shell_roots {
        walk(believed, root, 0, None, &mut out);
    }
    for root in &believed.gallery_roots {
        walk(believed, root, 0, Some("gallery"), &mut out);
    }
    format!("[{}]", out.join(","))
}

/// All front-door controls in logical pixels, without field values or credentials.
fn lobby_controls_json(believed: &Believed) -> String {
    let rows: Vec<String> = believed
        .exits
        .iter()
        .map(|(entity, press, marked, node, place)| {
            let scale = node.inverse_scale_factor;
            let size = node.size() * scale;
            let mid = place.translation * scale;
            format!(
                "{{\"press\":{press},\"entity\":{entity},\"duel_exit\":{marked},\
                 \"at_x\":{x:.1},\"at_y\":{y:.1},\"w\":{w:.1},\"h\":{h:.1}}}",
                press = quoted(&format!("{press:?}")),
                entity = entity.index(),
                marked = marked.is_some(),
                x = mid.x,
                y = mid.y,
                w = size.x,
                h = size.y,
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// The refusal in the prompt bar's one slot, as **who wrote it** and **what
/// it says**.
///
/// Two fields and not a string, for the reason [`sources_json`] is a list
/// and not a count. Since #121 that slot holds a [`Refusal`] —
/// `Said(Phrase)` for a sentence this client owns and translates, `Verbatim`
/// for prose another process sent and nobody may translate — and that is
/// *the distinction the bug was about*.
/// A probe printing only the rendered sentence could not tell a translated
/// phrase from untranslated prose, so it could not refuse a regression of the
/// very fault it would be watching.
///
/// So `said` names the `Phrase` when this client owns the sentence and is
/// `null` when it does not, and `text` is what the player actually reads, in
/// the language the client is set to. The whole field is `null` when nothing
/// has been refused.
pub(super) fn refusal_json(refusal: Option<&Refusal>, lang: Lang) -> String {
    let Some(refusal) = refusal else {
        return "null".to_string();
    };
    let said = match refusal {
        Refusal::Said(phrase) => quoted(&format!("{phrase:?}")),
        Refusal::Verbatim(_) => "null".to_string(),
    };
    format!(
        "{{\"said\":{said},\"text\":{}}}",
        quoted(&refusal.text(lang))
    )
}

/// What the client believes, as JSON.
///
/// Deliberately the *client's* answer and not the engine's: this is the thing
/// under test. `view` is what the host last sent, `interaction` is what the
/// client made of it, and a disagreement between them is exactly the class of
/// bug this endpoint exists to show.
fn presentation_json(believed: &Believed) -> serde_json::Value {
    let bounds = |node: &ComputedNode, place: &UiGlobalTransform| {
        let size = node.size() * node.inverse_scale_factor;
        let mid = place.translation * node.inverse_scale_factor;
        serde_json::json!({"x":mid.x-size.x/2.0,"y":mid.y-size.y/2.0,"w":size.x,"h":size.y})
    };
    let legal: Vec<_> = believed
        .legal_text
        .iter()
        .filter(|(text, _, _)| {
            text.0.contains("unofficial Fan Content") || text.0.contains("github.com/")
        })
        .map(|(text, node, place)| serde_json::json!({"text":text.0,"bounds":bounds(node,place)}))
        .collect();
    let music: Vec<_> = believed.music_controls.iter()
        .map(|(action,node,place)|serde_json::json!({"action":format!("{action:?}"),"bounds":bounds(node,place)})).collect();
    let lang = believed
        .settings
        .as_ref()
        .map_or(Lang::En, |s| Lang::of(&s.lang));
    let hints = [
        Phrase::PreviewTurn,
        Phrase::PreviewAlternate,
        Phrase::PreviewTurnCompact,
        Phrase::PreviewAlternateCompact,
    ]
    .map(|phrase| phrase.text(lang));
    let preview_hints: Vec<_> = believed
        .legal_text
        .iter()
        .filter(|(text, _, _)| hints.contains(&text.0.as_str()))
        .map(|(text, node, place)| serde_json::json!({"text":text.0,"bounds":bounds(node,place)}))
        .collect();
    serde_json::json!({"legal":legal,"music_controls":music,"preview_hints":preview_hints,
        "volume":believed.settings.as_ref().map(|s|s.music.volume()),
        "muted":believed.settings.as_ref().map(|s|s.music.muted()),
        "score":believed.score.as_ref().map(|heard| score_json(heard.0))})
}

/// The music's last request, as the drivers decided it.
fn score_json(r: baylee_client_core::music::ScoreRequest) -> serde_json::Value {
    serde_json::json!({"scene":format!("{:?}", r.scene),"tension":r.tension,"combat":r.combat,
        "big_spell":r.big_spell,"low_life":r.low_life,"lethal":r.lethal,"own_turn":r.own_turn,
        "about_to_lose":r.about_to_lose,"hunts":r.hunts,"hunt_mine":r.hunt_mine,
        "monarchs":r.monarchs,"spells":r.spells,"arrivals":r.arrivals})
}

fn loading_json(believed: &Believed) -> serde_json::Value {
    believed
        .journey
        .as_ref()
        .map_or(serde_json::Value::Null, |j| {
            j.diagnostic(
                believed
                    .real_time
                    .as_ref()
                    .map_or(0.0, |t| t.elapsed_secs_f64()),
            )
        })
}

#[allow(clippy::too_many_lines)] // one diagnostic snapshot, including presentation bounds
pub(super) fn state_dump(believed: &Believed, window: Vec2) -> String {
    let Some(duel) = believed.duel.as_deref() else {
        return "{\"duel\":null}".to_string();
    };
    let settings = believed.settings.as_deref();
    let departing = believed.leaving.iter().count();
    let shelves = believed.shelves.as_deref();
    let view = duel
        .view
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok())
        .unwrap_or_else(|| "null".to_string());
    // `Interaction` itself is not serialisable, and giving it derives to
    // suit this endpoint would be a real API change to client-core for a
    // debugging convenience. Its substance is public anyway: the choice the
    // engine posed, and what has been picked towards answering it.
    let interaction = duel.interaction.as_ref().map_or_else(
        || "null".to_string(),
        |i| {
            let pending = serde_json::to_string(i.pending()).unwrap_or_else(|_| "null".to_string());
            // `selected` is empty in both combat modes — an attack and a
            // block are *pairs*, and they live in `assignments` instead. A
            // caller reading only the count therefore watches a declaration
            // being built and sees nothing happen, which is a morning this
            // harness has already cost once.
            let pairs: Vec<String> = i
                .assignments()
                .into_iter()
                .map(|(creature, at)| {
                    format!(
                        "{{\"creature\":{},\"at\":{}}}",
                        creature.slot(),
                        quoted(&format!("{at:?}"))
                    )
                })
                .collect();
            // `focus` is combat's alone, and `aim` is the same question asked
            // of every mode that has an answer to it — the row a dialog's
            // keyboard is standing on included. Both, rather than the second
            // in place of the first: `CombatFocus` says whether the thing
            // aimed at is a defender or an attacker, which `Pick` drops.
            //
            // Without `aim`, a `Mode::Objects` focus is invisible here, and
            // proving that a key moved it takes a photograph and a pixel
            // diff — which is what it took once.
            format!(
                "{{\"pending\":{pending},\"selected\":{selected},\"selected_players\":{seats},\
                 \"assignments\":[{pairs}],\"focus\":{focus},\"aim\":{aim}}}",
                selected = i.selected().count(),
                seats = i.selected_players().count(),
                pairs = pairs.join(","),
                focus = quoted(&format!("{:?}", i.combat_focus())),
                aim = quoted(&format!("{:?}", i.aim())),
            )
        },
    );
    // The two-stage arm: the first tap on anything irreversible only arms it,
    // and a caller that does not know a tap armed rather than fired reads the
    // second tap as the one that did nothing.
    let armed = duel.armed.as_ref().map_or_else(
        || "null".to_string(),
        |armed| {
            format!(
                "{{\"object\":{},\"deed\":{}}}",
                armed.object.slot(),
                quoted(&format!("{:?}", armed.deed))
            )
        },
    );
    let face_builds = believed.face_builds.as_deref().map_or(0, |b| b.0);
    let lang = settings.map_or_else(|| "null".to_string(), |s| quoted(&s.lang));
    let error = refusal_json(
        duel.last_error.as_ref(),
        settings.map_or(Lang::En, |s| Lang::of(&s.lang)),
    );
    format!(
        "{{\"view\":{view},\"interaction\":{interaction},\"hovered\":{hovered},\
         \"autopilot\":{autopilot},\"last_error\":{error},\"lang\":{lang},\
         \"reachable\":{reachable},\"sources\":{sources},\
         \"activatable\":{activatable},\"armed\":{armed},\
         \"outbox\":{outbox},\"mana_run\":{mana_run},\"ability_menu\":{menu},\
         \"ability_tap\":{tap},\"cast_menu\":{cast_menu},\"cast_answer\":{cast_answer},\
         \"last_cue\":{last_cue},\"last_count\":{last_count},\"cues_suppressed\":{cues_suppressed},\
         \"departing\":{departing},\"cards\":{cards},\"buttons\":{buttons},\"browser\":{browser},\"shelves\":{shelves},\
         \"presentation\":{presentation},\"phase\":{phase},\"loading\":{loading},\"lobby_controls\":{lobby_controls},\"exits\":{exits},\"face_builds\":{face_builds},\
         \"ui_rebuilds\":{ui_rebuilds},\"shell_nodes\":{shell_nodes},\"desk_controls\":{desk_controls},\
         \"shell\":{shell},\"camera\":{camera},\"arrangement\":{arrangement},\"dial\":{dial},\"chips\":{chips},\"plates\":{plates}}}",
        shell = shell_keys_json(believed),
        camera = camera_json(believed, duel),
        dial = dial_json(believed),
        chips = chips_json(believed),
        plates = plates_json(believed),
        arrangement = arrangement_json(believed, duel),
        ui_rebuilds = rebuilds_json(believed),
        desk_controls = desk_controls_json(believed),
        shell_nodes = shell_nodes_json(believed),
        // Which screen this is, and — on the end screen only — the ways off
        // it with `duel_exit` saying which the keyboard can see. See
        // [`exits_json`] for why that flag is the row rather than a detail
        // of it, and `Believed::phase` for what a probe that cannot name its
        // own screen costs a reporter.
        phase = believed
            .phase
            .as_ref()
            .map_or_else(|| "null".to_string(), |p| quoted(&format!("{:?}", p.get()))),
        presentation = presentation_json(believed),
        loading = loading_json(believed),
        exits = exits_json(believed),
        lobby_controls = lobby_controls_json(believed),
        cards = cards_json(believed, duel, window),
        buttons = buttons_json(believed),
        browser = browser_json(duel),
        shelves = shelves_json(
            shelves,
            duel.view.as_ref().is_some_and(|v| v.day_night.is_some())
        ),
        hovered = duel
            .hovered
            .map_or_else(|| "null".to_string(), |h| quoted(&format!("{h:?}"))),
        autopilot = duel
            .autopilot
            .map_or_else(|| "null".to_string(), |a| quoted(&format!("{a:?}"))),
        reachable = duel.reachable.len(),
        // And *which lands it could tap*, which is the one thing the count
        // above could never be compared against. `reachable` is a number
        // derived from this list, so a caller reading only the number can
        // tell that the client offered fewer cards than it should have and
        // cannot tell whether the planner refused them or never saw a source
        // to pay with — a measurement that can refuse nothing. It cost three
        // round trips to establish that on #127; it is one line to answer.
        sources = sources_json(duel),
        activatable = duel.activatable.len(),
        // Four states that answer silently and are all but invisible in a
        // screenshot: an action queued but never sent, a mana run that owns
        // the next few keys, an ability menu that swallows the keyboard
        // whole, and a card still playing its way off the table. The first
        // three look exactly like "the key did nothing"; the fourth is the
        // opposite problem — it is over in half a second, so a caller that
        // wants to photograph it has to be told when to look.
        outbox = duel.outbox().len(),
        mana_run = duel.mana_run.is_some(),
        menu = duel
            .ability_menu
            .map_or_else(|| "null".to_string(), |m| quoted(&format!("{m:?}"))),
        // And which tap of it the sheet has stepped into, which is the sixth
        // silent state: the sheet is a bubble of one ability's colours and
        // the screenshot of that is a row of five discs — the same picture a
        // permanent whose own pips those are would draw.
        tap = duel
            .asking_tap()
            .map_or_else(|| "null".to_string(), |t| t.to_string()),
        // The seventh and eighth, and they are one state read at its two
        // ends. The cast chooser swallows the keyboard exactly as the ability
        // sheet does, and the way it was answered with then travels silently
        // through a whole mana run to meet the engine's own question several
        // round trips later — so a caller that could see neither could not
        // tell "the chooser is standing" from "the click did nothing", nor
        // "the evoke was chosen" from "the engine picked for us again". See
        // [`crate::CastMenu`].
        cast_menu = cast_menu_json(duel),
        cast_answer = cast_answer_json(duel),
        // The fifth thing that happens without leaving a mark on the screen,
        // and the only one that is meant to leave none: the client decides
        // what is worth hearing (`baylee_client_core::cue`) before anything
        // can play it, so the last cue is how that decision is *proved* —
        // by a read, rather than by somebody listening at the right moment.
        last_cue = duel
            .cues
            .last()
            .map_or_else(|| "null".to_string(), |beat| quoted(beat.cue.name())),
        // And how many of it, which is the half a name cannot carry: three
        // cards drawn and one drawn are the same cue and two different
        // sounds, so a harness that could read only the name could not tell a
        // burst from a tap. `0` when nothing has been heard yet, and `1` for
        // every cue that has no amount in it.
        last_count = duel.cues.last().map_or(0, |beat| beat.count),
        // What the priority cue's policy held back (DESIGN-v7 §4.3): the
        // debounce and the quiet after this seat's own action, counted, so
        // a harness can prove the policy without ears.
        cues_suppressed = duel.cues.suppressed(),
    )
}

/// The arrangement (DESIGN-v8 §2.4): the one in effect, what this game's
/// switch, the device's per-count memory and its default say, which
/// arrangements are offered here and why the others are not, the follow
/// switch, the seat of interest, whether anything is still moving and for
/// how many frames nothing has, and the pill's and the menu's rectangles.
fn arrangement_json(believed: &Believed, duel: &Duel) -> String {
    use baylee_client_core::tableview::Arrangement;
    let (frame, glide, pill, panel, seat_mats) = &believed.arrangement;
    let measured = frame.as_deref().copied().unwrap_or_default();
    let lang = believed
        .settings
        .as_deref()
        .map_or(Lang::En, |s| Lang::of(&s.lang));
    let table = believed
        .settings
        .as_deref()
        .map(|s| s.table)
        .unwrap_or_default();
    let class = measured.class();
    let offered: Vec<&str> = Arrangement::offered_at(measured.seats, class)
        .into_iter()
        .map(Arrangement::tag)
        .collect();
    let reason: serde_json::Map<String, serde_json::Value> = Arrangement::ALL
        .into_iter()
        .filter_map(|a| {
            a.offered(measured.seats, class).err().map(|why| {
                let text = if why == Phrase::ArrComing {
                    why.fill(lang, &[a.package()])
                } else {
                    why.text(lang).to_string()
                };
                (a.tag().to_string(), serde_json::Value::String(text))
            })
        })
        .collect();
    let by_seats: serde_json::Map<String, serde_json::Value> = table
        .arrangement_by_seats
        .iter()
        .map(|(n, a)| {
            (
                n.to_string(),
                serde_json::Value::String(a.tag().to_string()),
            )
        })
        .collect();
    let rect = |(node, place): (&ComputedNode, &UiGlobalTransform)| {
        let size = node.size() * node.inverse_scale_factor;
        let mid = place.translation * node.inverse_scale_factor;
        serde_json::json!({"x":mid.x-size.x/2.0,"y":mid.y-size.y/2.0,"w":size.x,"h":size.y})
    };
    let glide = glide.as_deref().copied().unwrap_or_default();
    let orbiting = believed
        .rig
        .as_deref()
        .copied()
        .is_some_and(crate::table::ShownRig::moving);
    serde_json::json!({
        "current": duel.arrangement.tag(),
        "chosen": crate::arrangement::chosen(duel, &table, measured.seats).tag(),
        "game": duel.arrangement_game.map(Arrangement::tag),
        "default": table.arrangement.tag(),
        "by_seats": by_seats,
        "offered": offered,
        "reason": reason,
        "follow": table.follow,
        "interest": duel.visiting.map(baylee_core::ids::PlayerId::get),
        "moves_cards": duel.arrangement.moves_cards(),
        "seats": measured.seats,
        "frame": class.name(),
        "moving": glide.moving > 0 || orbiting,
        "cards_moving": glide.moving,
        "settled_frames": glide.settled_frames,
        "menu": duel.arrangement_menu,
        "remember": duel.arrangement_remember,
        "flash": measured.flash,
        "pill": pill.iter().next().map(rect),
        "panel": panel.iter().next().map(rect),
        "parked": duel.layout.as_ref().map(|l| l.slots.iter().filter(|s| s.parked).map(|s| s.player.get()).collect::<Vec<_>>()),
        // Every mat drawn, where it is (table space) and how it is turned,
        // beside where its seat's slot says it belongs.
        "mats": seat_mats.iter().map(|(mat, at)| {
            let slot = duel.layout.as_ref().and_then(|l| l.slot(mat.0));
            serde_json::json!({
                "seat": mat.0.get(),
                "at": [at.translation.x, -at.translation.z, at.translation.y],
                "slot": slot.map(|s| [s.center.x, s.center.y, s.facing]),
            })
        }).collect::<Vec<_>>(),
        "tear": duel.tear.as_ref().map(|t| t.t),
    })
    .to_string()
}

/// The camera (DESIGN-v7 §2.7): the rig it is going to, the rig it is at,
/// the seat it visits, whether a visit or a return is under way, which fit
/// bound the shot, the pose and its frame.
fn camera_json(believed: &Believed, duel: &Duel) -> String {
    let rig_json = |rig: crate::table::CameraRig| {
        format!(
            "{{\"target\":[{:.3},{:.3}],\"distance\":{:.3},\"yaw\":{:.4},\"lean\":{:.3}}}",
            rig.target.x, rig.target.y, rig.distance, rig.yaw, rig.lean
        )
    };
    let Some(rig) = believed.camera.0.as_deref().copied() else {
        return "null".to_string();
    };
    let shown = believed.rig.as_deref().copied();
    let pose = believed.camera.1.as_deref().copied().unwrap_or_default();
    format!(
        "{{\"rig\":{},\"shown\":{},\"visiting\":{},\"moving\":{},\"binds\":{},\"pose\":{},\"frame\":{},\"dial_in_frame\":{}}}",
        rig_json(rig),
        shown
            .and_then(crate::table::ShownRig::rig)
            .map_or_else(|| "null".to_string(), rig_json),
        duel.visiting
            .map_or_else(|| "null".to_string(), |seat| seat.get().to_string()),
        shown.is_some_and(crate::table::ShownRig::moving),
        quoted(pose.binds.name()),
        quoted(if duel.visiting.is_some() {
            "visit"
        } else {
            "home"
        }),
        quoted(
            pose.frame
                .map_or("ring", baylee_client_core::tableview::VisitFrame::name)
        ),
        pose.dial_in_frame,
    )
}

/// Which of a seat's two top-edge lines show, on its chip (`plate` false)
/// or its plate: `{"turn":…,"priority":…}`.
fn lines_json(believed: &Believed, player: u8, plate: bool) -> String {
    let shows = |kind: crate::hud::TagKind| {
        believed.chips.0.iter().any(|(tag, seen, ink)| {
            tag.player.get() == player
                && tag.plate == plate
                && tag.kind == kind
                && *seen != Visibility::Hidden
                && ink.0.alpha() >= 0.5
        })
    };
    format!(
        "{{\"turn\":{},\"priority\":{}}}",
        shows(crate::hud::TagKind::Turn),
        shows(crate::hud::TagKind::Priority)
    )
}

/// The seat buttons (DESIGN-v7 WT5, and the owner's lines of 08.10.2026):
/// per seat, which of its top-edge lines show, its crown, its words and its
/// box.
fn chips_json(believed: &Believed) -> String {
    let seats: std::collections::BTreeSet<u8> = believed
        .chips
        .0
        .iter()
        .filter(|(tag, ..)| !tag.plate)
        .map(|(tag, ..)| tag.player.get())
        .collect();
    let rows: Vec<String> = seats
        .iter()
        .map(|seat| {
            let lines = lines_json(believed, *seat, false);
            let chip = believed
                .chips
                .2
                .iter()
                .find(|(tab, ..)| tab.player.get() == *seat);
            let crown = believed
                .chips
                .4
                .iter()
                .any(|c| c.player.get() == *seat);
            let (hint, rect) = chip.map_or_else(
                || ("null".to_string(), "null".to_string()),
                |(_, hint, node, at)| (quoted(&hint.0), rect_json(node, at)),
            );
            format!(
                "{{\"seat\":{seat},\"lines\":{lines},\"crown\":{crown},\"hint\":{hint},\"rect\":{rect}}}"
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// A UI node's drawn box in logical pixels, `[x, y, w, h]`: its middle and
/// its unturned size (a turned plate reports the box before the turn).
fn rect_json(node: &ComputedNode, at: &UiGlobalTransform) -> String {
    let scale = node.inverse_scale_factor();
    let size = node.size() * scale;
    let middle = at.translation * scale;
    format!(
        "[{:.1},{:.1},{:.1},{:.1}]",
        middle.x - size.x * 0.5,
        middle.y - size.y * 0.5,
        size.x,
        size.y
    )
}

/// The seats' plates on the table (the owner's requests of 08.10.2026): per
/// seat, whether it is drawn, its box, its crown, ∞ and pool line, and its
/// name in words.
fn plates_json(believed: &Believed) -> String {
    let mut rows: Vec<(u8, String)> = believed
        .chips
        .1
        .iter()
        .map(|(tab, node, computed, at, hint)| {
            let seat = tab.player.get();
            let marked = |kind: crate::hud::PlateMarkKind| {
                believed
                    .chips
                    .3
                    .iter()
                    .any(|m| m.player == tab.player && m.kind == kind)
            };
            (
                seat,
                format!(
                    "{{\"seat\":{seat},\"shown\":{},\"rect\":{},\"lines\":{},\"crown\":{},\"unlimited\":{},\"pool\":{},\"hint\":{},\"steps\":{}}}",
                    node.display != Display::None,
                    rect_json(computed, at),
                    lines_json(believed, seat, true),
                    marked(crate::hud::PlateMarkKind::Crown),
                    marked(crate::hud::PlateMarkKind::Unlimited),
                    marked(crate::hud::PlateMarkKind::Pool),
                    hint.map_or_else(|| "null".to_string(), |h| quoted(&h.0)),
                    steps_json(believed, tab.player),
                ),
            )
        })
        .collect();
    rows.sort_by_key(|(seat, _)| *seat);
    let rows: Vec<String> = rows.into_iter().map(|(_, row)| row).collect();
    format!("[{}]", rows.join(","))
}

/// Where `player`'s steps are drawn — `{"shown", "rect", "names"}`, the
/// names `"over"` or `"under"` the tiles — or `null` with no steps panel.
fn steps_json(believed: &Believed, player: baylee_core::ids::PlayerId) -> String {
    believed
        .chips
        .5
        .iter()
        .find(|(bar, panel, ..)| {
            bar.player == player && matches!(panel, crate::hud::seatbar::attached::Panel::Phases)
        })
        .map_or_else(
            || "null".to_string(),
            |(_, _, node, computed, at)| {
                let names = if node.flex_direction == FlexDirection::ColumnReverse {
                    "under"
                } else {
                    "over"
                };
                format!(
                    "{{\"shown\":{},\"rect\":{},\"names\":\"{names}\"}}",
                    node.display != Display::None,
                    rect_json(computed, at)
                )
            },
        )
}

/// The dial (DESIGN-v7 §2.7): where the hands point, who is deciding, the
/// hub's pulse, and how big the turn number and the dial are drawn.
fn dial_json(believed: &Believed) -> String {
    let Some(dial) = believed.dial.as_deref() else {
        return "null".to_string();
    };
    let vector = |v: Option<Vec2>| {
        v.map_or_else(
            || "null".to_string(),
            |v| format!("[{:.4},{:.4}]", v.x, v.y),
        )
    };
    // A hand's bearing in degrees, counter-clockwise from the table's x axis
    // (y away from me): what a screenshot's hand is checked against.
    let angle = |v: Option<Vec2>| {
        v.map_or_else(
            || "null".to_string(),
            |v| format!("{:.2}", v.y.atan2(v.x).to_degrees()),
        )
    };
    let deciding: Vec<String> = dial.deciding.iter().map(|p| p.get().to_string()).collect();
    format!(
        "{{\"turn_hand\":{},\"priority_hand\":{},\"turn_deg\":{},\"priority_deg\":{},\"deciding\":[{}],\"hub_pulse\":{:.3},\"number_px\":{:.2},\"number_w\":{:.2},\"dial_px\":{:.2},\"scale\":{:.4},\"centre\":{},\"radius\":{:.3},\"effects\":{},\"uploads\":{}}}",
        vector(dial.turn_hand),
        vector(dial.priority_hand),
        angle(dial.turn_hand),
        angle(dial.priority_hand),
        deciding.join(","),
        dial.hub_pulse,
        dial.number_px,
        dial.number_w,
        dial.dial_px,
        dial.scale,
        vector(dial.centre),
        dial.radius,
        dial.effects,
        dial.uploads,
    )
}

/// The cast chooser, while it stands: which card, how many ways, which row.
///
/// The near end of a state that is invisible in a screenshot in the way the
/// ability sheet's is — it swallows the keyboard, and a caller that could not
/// see it could not tell it from a click that did nothing.
fn cast_menu_json(duel: &Duel) -> String {
    duel.cast_menu.as_ref().map_or_else(
        || "null".to_string(),
        |menu| {
            format!(
                "{{\"card\":{},\"ways\":{},\"pick\":{}}}",
                menu.card.slot(),
                menu.modes.len(),
                menu.pick
            )
        },
    )
}

/// The far end of the same state: the way the player picked, still owed to a
/// question the engine has not asked yet.
///
/// It travels through a whole mana run — several round trips — before it is
/// spent, and nothing on the screen says so. Without it a caller cannot tell
/// "the evoke was chosen" from "the engine picked for us again", which is the
/// distinction the whole of [`crate::CastMenu`] exists to make.
fn cast_answer_json(duel: &Duel) -> String {
    duel.cast_answer.as_ref().map_or_else(
        || "null".to_string(),
        |(card, kind)| {
            format!(
                "{{\"card\":{},\"kind\":{}}}",
                card.slot(),
                quoted(&format!("{kind:?}"))
            )
        },
    )
}

/// Where every drawn card is on screen, and which card it is.
///
/// This is the endpoint's answer to the thing that has cost this harness the
/// most time by a distance: **finding a card to click**. The advice was to
/// read the object out of the view, the lane out of the board and the pixels
/// out of a screenshot — three lookups, the last of them by eye on a
/// downscaled image, and every one of them repeated after the lane repacked.
/// `at_x`/`at_y` are logical pixels and go straight into `/pointer`.
///
/// **Every part of the question**, because a caller that can find a permanent
/// but not a card in hand still cannot play a game, and one that can find
/// both but not a spell on the stack cannot answer a counterspell: `zone` is
/// `table` for the 3D scene, `hand` for the row and `stack` for the panel,
/// and all three answer in the same logical pixels, so a caller need not know
/// which kind of thing it is clicking.
///
/// Two things it is careful about. A table card's rect is measured from the
/// **live** `Transform`, so a card mid-glide reports where it is rather than
/// where it is going. And the box is the card's own four corners put through
/// that transform, so a tapped card reports the wider, shorter box it
/// actually covers rather than an upright one around its middle. The height a
/// card is drawn at is *not* one of the careful parts: `CARD_LIFT` moves a
/// card 0.14 px at a duel, which is why aiming at the felt under one has
/// worked all along.
fn cards_json(believed: &Believed, duel: &Duel, window: Vec2) -> String {
    let Some(rig) = believed.rig.as_deref().and_then(|shown| shown.rig()) else {
        return "null".to_string();
    };
    if window.x <= 0.0 || window.y <= 0.0 {
        return "null".to_string();
    }
    let lens = crate::table::Lens::new(rig, window);
    let on_the_table = believed.cards.iter().filter_map(|(visual, at, seen)| {
        if *seen == Visibility::Hidden {
            return None;
        }
        let (mid, size) = crate::table::card_box(&lens, at)?;
        Some(card_row(
            duel,
            "table",
            visual.object,
            visual.count,
            mid,
            size,
        ))
    });
    // The hand is `bevy_ui` and needs no projection at all: the layout has
    // already put the node somewhere, in *physical* pixels, and
    // `inverse_scale_factor` is the way back to the logical ones `/pointer`
    // speaks. Reading the computed node rather than recomputing the row's
    // arithmetic is also what carries the scroll offset and whatever `touch`
    // has the card doing under the finger.
    let in_the_hand = believed.hand.iter().map(|(visual, computed, place)| {
        let scale = computed.inverse_scale_factor;
        card_row(
            duel,
            "hand",
            visual.object,
            1,
            place.translation * scale,
            computed.size() * scale,
        )
    });
    // The stack reads exactly like the hand — a `bevy_ui` node whose computed
    // box is already in physical pixels — and reports the *row*, not the
    // picture on it, because the row is what answers a click now.
    let on_the_stack = believed.stack.iter().map(|(visual, computed, place)| {
        let scale = computed.inverse_scale_factor;
        card_row(
            duel,
            "stack",
            visual.object,
            1,
            place.translation * scale,
            computed.size() * scale,
        )
    });
    // By zone and then by object, so two runs of the same board answer in the
    // same order and a diff between them is about the table rather than about
    // the ECS.
    let mut rows: Vec<(&str, u32, String)> = on_the_table
        .chain(in_the_hand)
        .chain(on_the_stack)
        .collect();
    rows.sort_unstable_by_key(|(zone, object, _)| (*zone, *object));
    let rows: Vec<String> = rows.into_iter().map(|(_, _, row)| row).collect();
    format!("[{}]", rows.join(","))
}

fn browser_json(duel: &Duel) -> serde_json::Value {
    serde_json::json!({
        "open": duel.browser.is_open(),
        "typing": duel.browser.is_typing(),
        "filter": duel.browser.filter(),
        "for_choice": duel.browser.for_choice(),
        "dismissible": duel.browser.may_be_put_away(),
    })
}

/// Where the answers are: the shelf's buttons, the two choosers, and
/// everything that acts on the game without answering it.
///
/// `kind` says which list a button came from and `label` which one it is —
/// the prompt or menu action by name (`Yes`, `No`, `Confirm`,
/// `DeclareNothing`, `Step(1)`, `Concede`, `HoldForStack`), and a position for
/// the ability and choice rows, which is what those carry themselves: both are
/// rebuilt from the current `LegalActions` when pressed, so an index is the
/// only stable handle there is.
///
/// This exists because the keyboard does not reach all of it. `Yes` and `No`
/// have no binding at all — see `docs/observed-faults.md` — so without these
/// coordinates a driven client stops dead at the first shockland.
fn buttons_json(believed: &Believed) -> String {
    let mut rows: Vec<String> = Vec::new();
    let mut push =
        |kind: &str, label: String, node: &bevy::ui::ComputedNode, at: Vec2, extra: String| {
            let scale = node.inverse_scale_factor;
            let size = node.size() * scale;
            let mid = at * scale;
            rows.push(format!(
                "{{\"kind\":\"{kind}\",\"label\":{label},\"at_x\":{x:.1},\"at_y\":{y:.1},\
             \"w\":{w:.1},\"h\":{h:.1}{extra}}}",
                label = quoted(&label),
                x = mid.x,
                y = mid.y,
                w = size.x,
                h = size.y,
            ));
        };
    for (button, node, place) in &believed.prompts {
        push(
            "prompt",
            format!("{:?}", button.action),
            node,
            place.translation,
            String::new(),
        );
    }
    for (button, node, place) in &believed.abilities {
        push(
            "ability",
            button.index.to_string(),
            node,
            place.translation,
            ability_row_words(believed, button.index),
        );
    }
    for (button, node, place) in &believed.choices {
        push(
            "choice",
            button.index.to_string(),
            node,
            place.translation,
            String::new(),
        );
    }
    for (button, node, place) in &believed.menus {
        push(
            "menu",
            format!("{:?}", button.action),
            node,
            place.translation,
            String::new(),
        );
    }
    for (card, node, place) in &believed.tray_cards {
        push(
            "browser-card",
            card.object.slot().to_string(),
            node,
            place.translation,
            format!(",\"object\":{}", card.object.slot()),
        );
    }
    for (node, place) in &believed.tray_filters {
        push(
            "browser-control",
            "Filter".into(),
            node,
            place.translation,
            String::new(),
        );
    }
    for (node, place) in &believed.tray_none {
        push(
            "browser-control",
            "Decline".into(),
            node,
            place.translation,
            String::new(),
        );
    }
    rows.sort_unstable();
    format!("[{}]", rows.join(","))
}

/// What an ability row reads, as three more fields on its button.
///
/// `words` is what the row says: the whole printed sentence, cost and all
/// (for a prepared cast, the spell's name and text; for a grant, the
/// grantor's name and the sentence that grants it), or the client's
/// one-line name where the card prints none, `null` where it draws neither.
/// `head` is its cost column as drawn: the sentence's own head, the
/// ability's symbols where there is no sentence, `null` where the sentence
/// is drawn whole. `source` is where the words came from: `localized` (the
/// player's printing), `oracle` (the compiled English), `token` (a registry
/// token's row, which no card prints) or `none` (no printed sentence: the
/// CR 305.6 tap, a grant whose grantor is hidden, a sentence the count guard
/// refused). A grant's is its sentence's, or its name's where the view names
/// no sentence. A pour pip reports all three empty: it draws a colour and no
/// words.
///
/// Read through the doors the sheet itself draws through
/// ([`crate::cardtext::said`], [`crate::abilities::printed_words`],
/// [`crate::abilities::prepared_words`], [`crate::abilities::grant_words`]),
/// so it cannot say one thing while the row says another. A driver checking
/// "is this row German" could only read a screenshot before.
fn ability_row_words(believed: &Believed, index: usize) -> String {
    let unknown = ",\"words\":null,\"head\":null,\"source\":\"none\"".to_string();
    let Some(duel) = believed.duel.as_deref() else {
        return unknown;
    };
    let (Some(object), Some(view)) = (duel.ability_menu, duel.view.as_ref()) else {
        return unknown;
    };
    let lang = believed
        .settings
        .as_deref()
        .map_or(Lang::En, |s| Lang::of(&s.lang));
    let Some(option) =
        crate::hud::ability_options(duel, lang, object).and_then(|all| all.into_iter().nth(index))
    else {
        return unknown;
    };
    // A pour pip draws its colour and no words (`abilities::Split`).
    if option.pour.is_some() {
        return unknown;
    }
    let texts = believed.texts.as_deref();
    let shown = view.object(object);
    let said = option
        .printed
        .zip(shown.and_then(|o| o.rules))
        .and_then(|(at, rules)| crate::cardtext::said(texts, rules.card, at))
        .or_else(|| {
            texts.and_then(|texts| crate::abilities::prepared_words(texts, view, object, &option))
        })
        .or_else(|| {
            texts.and_then(|texts| crate::abilities::grant_words(texts, view, object, &option))
        });
    let cut = crate::abilities::printed_words(texts, view, object, &option);
    let source = match &said {
        Some((_, crate::cardtext::Said::Localized)) => "localized",
        Some((_, crate::cardtext::Said::Oracle)) => "oracle",
        None if shown.is_some_and(|o| o.token.is_some()) => "token",
        None => "none",
    };
    // The sheet's own rule for its `line`: the one-line name only where the
    // card prints nothing for the row, so a printed row whose sentence is
    // refused reports no words, as it draws none.
    let words = said.map_or_else(
        || {
            if let Some(token) = shown.and_then(|o| o.token)
                && let baylee_engine::choice::PlayerAction::ActivateAbility {
                    ability_index, ..
                } = option.action
                && let Some(blocks) = crate::cardtext::token_sentence(token, ability_index)
            {
                return Some(prose(&blocks));
            }
            (option.printed_index().is_none() && !option.label.is_empty())
                .then(|| option.label.clone())
        },
        |(blocks, _)| Some(prose(&blocks)),
    );
    let head = match cut {
        Some(cut) => cut.head,
        None => option.cost.clone(),
    };
    let text = |value: Option<String>| value.map_or_else(|| "null".to_string(), |v| quoted(&v));
    format!(
        ",\"words\":{words},\"head\":{head},\"source\":\"{source}\"",
        words = text(words),
        head = text(head),
    )
}

/// Blocks as one line, a reminder back in the parentheses it was printed in.
fn prose(blocks: &[baylee_client_core::card_face::TextBlock]) -> String {
    use baylee_client_core::card_face::TextBlock;
    blocks
        .iter()
        .map(|block| match block {
            TextBlock::Rules(text) => text.clone(),
            TextBlock::Reminder(text) => format!("({text})"),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One card of the answer, wherever it is drawn.
fn card_row(
    duel: &Duel,
    zone: &'static str,
    object: baylee_core::ids::ObjectId,
    count: usize,
    mid: Vec2,
    size: Vec2,
) -> (&'static str, u32, String) {
    (
        zone,
        object.slot(),
        format!(
            "{{\"object\":{object},\"zone\":\"{zone}\",\"count\":{count},\"name\":{name},\
             \"at_x\":{x:.1},\"at_y\":{y:.1},\"w\":{w:.1},\"h\":{h:.1}}}",
            object = object.slot(),
            name = name_of(duel, object),
            x = mid.x,
            y = mid.y,
            w = size.x,
            h = size.y,
        ),
    )
}

/// One string, as JSON.
///
/// `str::escape_default` is the obvious thing and is **not** JSON: it writes
/// an apostrophe as `\'` and anything outside ASCII as `\u{2014}`, neither of
/// which a JSON parser accepts. `Earth King's Lieutenant` is in the dev
/// board, so the first card name carrying an apostrophe made the whole dump
/// unreadable — every field in it, not just the name. Only the quote, the
/// backslash and the control characters need escaping; UTF-8 is already JSON.
pub(super) fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What the board model calls this object, if it is drawing it.
///
/// The board's name and not the view's: it is the name on the card the player
/// is looking at, and for a group of identical permanents it is the one name
/// that stands for all of them. The lanes, the piles and the hand are all
/// searched, because every one of them draws a card a caller may want to
/// click and the handle has to be the same in each.
///
/// `null` is a real answer rather than a failure: a library is face down to
/// everybody, its owner included (CR 401.2), so it has no name to give. The
/// object is still there and still clickable.
fn name_of(duel: &Duel, object: baylee_core::ids::ObjectId) -> String {
    let Some(board) = duel.board.as_ref() else {
        return "null".to_string();
    };
    board
        .group(object)
        .map(|group| group.name.clone())
        .or_else(|| {
            board
                .pods
                .iter()
                .flat_map(|pod| pod.piles.iter())
                .find(|pile| pile.top == Some(object))
                .and_then(|pile| pile.name.clone())
        })
        .or_else(|| {
            board
                .hand
                .iter()
                .find(|held| held.id == object)
                .map(|held| held.name.clone())
        })
        .or_else(|| {
            board
                .stack
                .iter()
                .find(|item| item.id == object)
                .map(|item| item.name.clone())
        })
        .map_or_else(|| "null".to_string(), |name| quoted(&name))
}

/// Where each seat's bar is drawn, and what it was allowed to be.
///
/// A bar is placed from a projection, not from a layout pass, so "the bar is
/// in the wrong place" is a claim about arithmetic that a screenshot can only
/// ever suggest. These are the numbers the placement was made from, in the
/// same logical pixels `/pointer` takes: `mid` is the centre the box is hung
/// on, `along` and `depth` are the projected ledge, and `ink` is what the
/// depth has to be able to hold. A shelf whose `ink` is close to its `depth`
/// is a bar about to stand on the creature lane behind it.
fn shelves_json(shelves: Option<&crate::hud::Shelves>, designated: bool) -> String {
    let Some(shelves) = shelves else {
        return "null".to_string();
    };
    let rows: Vec<String> = shelves
        .0
        .iter()
        .map(|(player, shelf)| {
            let box_size = shelf.box_size(designated);
            format!(
                "{{\"player\":{player},\"mid_x\":{mx:.1},\"mid_y\":{my:.1},\
                 \"along\":{along:.1},\"depth\":{depth:.1},\"tilt\":{tilt:.3},\
                 \"density\":\"{density:?}\",\"box_w\":{bw:.1},\"box_h\":{bh:.1},\
                 \"ink\":{ink:.1}}}",
                player = player.get(),
                mx = shelf.middle.x,
                my = shelf.middle.y,
                along = shelf.along,
                depth = shelf.depth,
                tilt = shelf.tilt,
                density = shelf.density,
                bw = box_size.x,
                bh = box_size.y,
                ink = shelf.density.ink_height(),
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}
