//! The HUD roots, their teardown, and the surfaces the rebuild draws on.

#[allow(clippy::wildcard_imports)] // the overlay's shared vocabulary
use super::*;

/// Removes the overlay when the duel hands the screen back.
///
/// The 3D stage has always been torn down on `Close`; the overlay was not,
/// because until the client grew a lobby nothing ever closed a duel and came
/// back to something else. The revision goes with it: it describes a tree that
/// no longer exists, and the next duel's first frame has to rebuild rather
/// than compare against it.
pub fn despawn_overlay(
    mut commands: Commands,
    existing: Query<Entity, HudRoots>,
    mut revision: ResMut<HudRevision>,
    mut ledge: ResMut<ledge::LedgeRevision>,
    ui_materials: Option<ResMut<UiCardMaterials>>,
    (strips, badges, plates, shells): PreviewObjects<'_>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    *revision = HudRevision::default();
    // The shelf goes with the root it hangs off, so its own counter describes
    // a tree that is not there either. Same reason, one level down.
    *ledge = ledge::LedgeRevision::default();
    // The cache is what holds those materials alive, so letting go of it here
    // is what actually frees them: a duel that ended must not leave a hand's
    // worth behind for the next one.
    if let Some(mut cache) = ui_materials {
        cache.clear();
    }
    if let Some(mut strips) = strips {
        strips.clear();
    }
    if let Some(mut badges) = badges {
        badges.clear();
    }
    if let Some(mut plates) = plates {
        plates.clear();
    }
    if let Some(mut shells) = shells {
        shells.clear();
    }
}

/// Every top-level node owned by the table's HUD.
pub type HudRoots = Or<(With<HudRoot>, With<super::DetachedHud>)>;

/// Everything this bar paints *with* that only exists when there is a render
/// world to paint in.
///
/// They travel as one because they are one answer to the same question, and
/// because `sync_overlay` is a system with sixteen parameters
/// and bevy implements `SystemParam` for tuples no longer than that — the
/// seventeenth is not a compile error about the limit, it is "`sync_overlay`
/// is not a system set" at every `.after()` in `lib.rs`, which is a long way
/// from the cause.
#[derive(bevy::ecs::system::SystemParam)]
pub struct Surfaces<'w> {
    /// The parchment, generated at startup — a headless app that never ran
    /// `setup_sheets` simply draws the flat colour the sheet is grained
    /// around.
    pub(super) sheets: Option<Res<'w, UiSheets>>,
    /// The cloth: two handles, each minted on the first frame there is
    /// somewhere to mint it and cloned on every frame after — the skirt over
    /// the whole hand zone, the rail over the actions row.
    ///
    /// Optional like the rest of this bundle, and for the same reason twice
    /// over: `FrontalPlugin` is what puts it there, and a test app that
    /// builds this tree adds no plugins at all. A required resource here is
    /// not a missing cloth — it is every overlay test panicking inside the
    /// scheduler, with the parameter's name switched off unless the `debug`
    /// feature is on.
    cloth: Option<ResMut<'w, crate::frontal::Cloth>>,
    /// Where they are minted. Absent headless, and then there is no cloth and
    /// both surfaces draw the flat dye instead.
    cloth_assets: Option<ResMut<'w, Assets<crate::frontal::FrontalMaterial>>>,
    /// The preview's keyword strip materials (#274), one per word. Optional
    /// for the cloth's reason: `MarksMaterialPlugin` puts them there.
    strips: Option<ResMut<'w, crate::marksmat::UiMarksMaterials>>,
    /// Where they are minted.
    strip_assets: Option<ResMut<'w, Assets<crate::marksmat::MarksUiMaterial>>>,
    /// The preview's count badge materials (#261), one per count. Optional
    /// for the cloth's reason: `BadgeMaterialPlugin` puts them there.
    badges: Option<ResMut<'w, crate::badgemat::UiBadgeMaterials>>,
    /// Where they are minted.
    badge_assets: Option<ResMut<'w, Assets<crate::badgemat::BadgeUiMaterial>>>,
    /// The preview's plate materials, one per thing a plate says. Optional
    /// for the cloth's reason: `PlateMaterialPlugin` puts them there.
    plates: Option<ResMut<'w, crate::platemat::UiPlateMaterials>>,
    /// Where they are minted.
    plate_assets: Option<ResMut<'w, Assets<crate::platemat::PlateUiMaterial>>>,
    /// The preview's shell materials, one per look. Optional for the cloth's
    /// reason: `ShellUiPlugin` puts them there.
    shells: Option<ResMut<'w, crate::shellui::UiShellMaterials>>,
    /// Where they are minted.
    shell_assets: Option<ResMut<'w, Assets<crate::shellui::ShellUiMaterial>>>,
}

impl Surfaces<'_> {
    /// The skirt's handle, or `None` when there is nowhere to draw it.
    pub(super) fn skirt(&mut self) -> Option<Handle<crate::frontal::FrontalMaterial>> {
        let assets = self.cloth_assets.as_deref_mut();
        self.cloth.as_mut()?.skirt(assets)
    }

    /// The rail's, which is the same cloth cut for a node 40 pixels tall.
    pub(super) fn rail(&mut self) -> Option<Handle<crate::frontal::FrontalMaterial>> {
        let assets = self.cloth_assets.as_deref_mut();
        self.cloth.as_mut()?.rail(assets)
    }

    /// The keyword strip for `bits`, or `None` when there is nowhere to draw
    /// it.
    pub(super) fn strip(
        &mut self,
        strip: baylee_client_core::cardrail::Strip,
    ) -> Option<Handle<crate::marksmat::MarksUiMaterial>> {
        let assets = self.strip_assets.as_deref_mut()?;
        Some(self.strips.as_mut()?.get(strip, assets))
    }

    /// The count badge saying `count`, or `None` when there is nowhere to
    /// draw it.
    pub(super) fn badge(&mut self, count: u32) -> Option<Handle<crate::badgemat::BadgeUiMaterial>> {
        let assets = self.badge_assets.as_deref_mut()?;
        Some(self.badges.as_mut()?.get(count, assets))
    }

    /// The plate saying `words`, or `None` when there is nowhere to draw
    /// it.
    pub(super) fn plate(
        &mut self,
        words: crate::platemat::PlateWords,
    ) -> Option<Handle<crate::platemat::PlateUiMaterial>> {
        let assets = self.plate_assets.as_deref_mut()?;
        Some(self.plates.as_mut()?.get(words, assets))
    }

    /// The shell `look`, or `None` when there is nowhere to draw it.
    pub(super) fn shell(
        &mut self,
        look: crate::shellmat::ShellLook,
    ) -> Option<Handle<crate::shellui::ShellUiMaterial>> {
        let assets = self.shell_assets.as_deref_mut()?;
        Some(self.shells.as_mut()?.get(look, assets))
    }
}
