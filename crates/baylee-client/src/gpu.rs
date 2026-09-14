//! What this particular graphics driver cannot be trusted with.
//!
//! Two entries, and the hope is that it stays two. The rule for anything that
//! lands here: it is a **measured** defect in a named driver, not a guess and
//! not a precaution, and the measurement is written down in `docs/mobile.md`
//! beside it.
//!
//! The condition should be the *adapter* wherever it can be, and not the
//! operating system: a GPU family is what has a bug, so `cfg(target_os =
//! "android")` takes a workaround away from every other device that shares
//! the family and hands it to every device that does not. Bevy asks the
//! adapter too — `is_non_supported_android_device` reads an Adreno model and
//! a Mali driver version — and `is_preprocessing_only_android_device` is the
//! cautionary tale beside it, because it asks through
//! `get_pixel10_driver_version`, which compares the name against one literal
//! string. A Pixel 11 is not that string, falls through, and crashes on its
//! first frame. [`disabled_features`] is the one thing here that *cannot*
//! ask, and it says why.

use bevy::render::settings::WgpuFeatures;
use bevy::render::view::Msaa;

/// The multisampling a camera should ask for on this adapter.
///
/// [`Msaa::Off`] on `PowerVR`, [`Msaa::Sample4`] — bevy's own default —
/// everywhere else.
///
/// `PowerVR` is a tiler, and the end-of-pass resolve is where tile memory is
/// written back. The driver in a Pixel 11 Pro XL (`"PowerVR C-Series
/// CXTP-48-1536 MC1"`, `25.3@6908880`) gets that wrong, and the damage falls
/// on whatever was written last, which is the UI pass: the lobby drew three
/// or four oversized glyphs out of a screen of text, a panel in two halves at
/// different offsets, and a different subset on every frame. It reads exactly
/// like a font that failed to load and is nothing of the kind — the Vulkan
/// validation layers, synchronisation checks included, report the client's
/// commands as valid.
///
/// Measured on two phones, same APK, clock stopped, two screenshots five
/// seconds apart — differing pixels, then lit pixels where the text is:
///
/// | | Pixel 11 Pro XL (`PowerVR`) | Pixel 6 Pro (Mali-G78) |
/// | --- | --- | --- |
/// | `Sample4` | 105 000 / 1 200 | 0 / 180 077 |
/// | `Off` | 4 300 / 4 700 | 0 / 180 209 |
///
/// The Mali column is why this asks the adapter instead of asking Android:
/// 4x costs that phone nothing, and a blanket rule would have taken smooth
/// edges off every non-`PowerVR` Android device to work around a driver they do
/// not have.
///
/// `None` — no renderer, which is how the lobby's decisions are tested —
/// answers with the default, because a headless app draws nothing to spoil.
#[must_use]
pub fn msaa(adapter: Option<&bevy::render::renderer::RenderAdapterInfo>) -> Msaa {
    match adapter {
        Some(info) if info.name.starts_with("PowerVR") => Msaa::Off,
        _ => Msaa::Sample4,
    }
}

/// Features to refuse regardless of what the adapter claims to support.
///
/// [`WgpuFeatures::INDIRECT_FIRST_INSTANCE`] on Android, which is how bevy
/// reads `culling_feature_support` in
/// `GpuPreprocessingSupport::from_world` — and it reads that feature nowhere
/// else, so dropping it asks for `GpuPreprocessingMode::PreprocessingOnly`
/// and clamps no limit.
///
/// That mode has no depth pyramid, and therefore no `mesh_preprocess.wgsl`,
/// and therefore no sampled image inside a compute shader — which is the one
/// thing `PowerVR`'s SPIR-V compiler aborts on, in
/// `spvcompiler::getMangledImageTypeString` under
/// `IMG_vkCreateComputePipelines`. Bevy already holds this GPU family to that
/// mode; it just recognises it by a literal comparison against the Pixel
/// 10's adapter name.
///
/// Unlike [`msaa`] this one **cannot** ask the adapter: `WgpuSettings` is
/// built before a device exists, so there is nothing to name yet. Android-wide
/// is therefore the widest honest gate, and it is wider than the defect —
/// every Android device pays for a driver only one family has.
///
/// What that costs is **not** measured, and should not be written down as if
/// it were: no build has ever run a Mali phone with the feature left on, so
/// the Pixel 6 Pro's flawless lobby says the workaround is harmless there and
/// says nothing about what it took away. The argument for paying it anyway is
/// arithmetic rather than a reading — what moves to CPU-side culling is a
/// table of a few dozen cards. If that ever stops being true, the way out is
/// bevy's own: ask for the feature, and put the phone back on
/// `PreprocessingOnly` from the render world once there is an adapter to name.
#[must_use]
pub fn disabled_features() -> Option<WgpuFeatures> {
    if cfg!(target_os = "android") {
        Some(WgpuFeatures::INDIRECT_FIRST_INSTANCE)
    } else {
        None
    }
}
