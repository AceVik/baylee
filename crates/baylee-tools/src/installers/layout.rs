//! Headless Finder metadata. The wire encoders are the maintained Rust
//! `ds_parser` and `alias_record` crates; no Finder/AppleScript/Python runs.
//! Settings retain the former dmg-settings.py/dmgbuild 1.6.7 contract.
// Layout serialization is also exercised on non-macOS hosts in offline tests.
#![cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
use anyhow::{Context, Result, ensure};
use dmg_layout::{
    AliasHeader, AliasKind, AliasMetadata, AliasRecord, AliasTarget, AliasVolume, CatalogNodeId,
    DiskType, FilesystemSignature, HfsTimestampSeconds, VolumeMountPath,
};
use ds_parser::{
    BrowserWindowSettings, FieldKey, FourCC, IconLocation, IconViewProperties, PropertyList,
    Record, Value, WindowBounds,
};

pub(super) const WIDTH: u32 = 660;
pub(super) const HEIGHT: u32 = 420;
pub(super) const APP_X: u32 = 170;
pub(super) const APPS_X: u32 = 490;
pub(super) const ICON_Y: u32 = 205;
pub(super) const BACKGROUND: &str = ".background.tiff";

/// Actual HFS+ identities, obtained only after mounting the writable image.
#[derive(Clone, Copy)]
pub(super) struct BackgroundIdentity {
    pub volume_created: u32,
    pub root_cnid: u32,
    pub file_created: u32,
    pub file_cnid: u32,
}
fn background_alias(id: BackgroundIdentity) -> Result<Vec<u8>> {
    let volume = AliasVolume::new(
        b"Baylee",
        HfsTimestampSeconds(id.volume_created),
        FilesystemSignature::HIERARCHICAL_FILE_SYSTEM_PLUS,
        DiskType::Ejectable,
    )?;
    let target = AliasTarget::new(
        AliasKind::File,
        CatalogNodeId(id.root_cnid),
        BACKGROUND.as_bytes(),
        CatalogNodeId(id.file_cnid),
        HfsTimestampSeconds(id.file_created),
    )?;
    Ok(AliasRecord::from_metadata(AliasMetadata {
        header: AliasHeader::new(volume, target),
        parent_folder_name: Some(b"Baylee".to_vec()),
        catalog_node_id_path: vec![id.root_cnid],
        unicode_target_name: Some(BACKGROUND.into()),
        unicode_volume_name: Some("Baylee".into()),
        target_path: Some(format!("/{BACKGROUND}").into_bytes()),
        // A stable mount hint; identity/CNIDs resolve it at another mount path.
        volume_path: Some(VolumeMountPath::new(b"/Volumes/Baylee")?),
    })?
    .to_bytes()?)
}

pub(super) fn encode(id: BackgroundIdentity) -> Result<Vec<u8>> {
    let mut window = BrowserWindowSettings {
        window_bounds: Some(WindowBounds {
            x: 200.0,
            y: 140.0,
            width: f64::from(WIDTH),
            height: f64::from(HEIGHT),
        }),
        show_sidebar: Some(false),
        show_toolbar: Some(false),
        show_status_bar: Some(false),
        show_path_bar: Some(false),
        show_tab_view: Some(false),
        container_show_sidebar: Some(false),
        sidebar_width: Some(180.0),
        ..Default::default()
    };
    window
        .unknown
        .insert("PreviewPaneVisibility".into(), PropertyList::Boolean(false));
    let mut icons = IconViewProperties {
        arrange_by: Some("none".into()),
        background_type: Some(2),
        background_color: Some((1.0, 1.0, 1.0)),
        grid_offset: Some((0.0, 0.0)),
        grid_spacing: Some(100.0),
        icon_size: Some(112.0),
        text_size: Some(13.0),
        label_on_bottom: Some(true),
        show_icon_preview: Some(false),
        show_item_info: Some(false),
        view_options_version: Some(1),
        ..Default::default()
    };
    icons.unknown.insert(
        "backgroundImageAlias".into(),
        PropertyList::Data(background_alias(id)?),
    );
    icons
        .unknown
        .insert("scrollPositionX".into(), PropertyList::Real(0.0));
    icons
        .unknown
        .insert("scrollPositionY".into(), PropertyList::Real(0.0));
    Ok(ds_parser::write(&[
        Record::new(".", FieldKey::vSrn, 1_u32),
        Record::new(".", FieldKey::bwsp, window),
        Record::new(".", FieldKey::icvp, icons),
        Record::new(
            ".",
            FieldKey::icvl,
            Value::Type(FourCC::from_array(*b"icnv")),
        ),
        Record::new(
            "Baylee.app",
            FieldKey::Iloc,
            IconLocation::new(APP_X, ICON_Y),
        ),
        Record::new(
            "Applications",
            FieldKey::Iloc,
            IconLocation::new(APPS_X, ICON_Y),
        ),
    ])?)
}

pub(super) fn verify(bytes: &[u8], id: BackgroundIdentity) -> Result<()> {
    let store = ds_parser::parse(bytes)?;
    ensure!(
        store.warnings.is_empty(),
        "Finder store has parser warnings"
    );
    let expected = ds_parser::parse(&encode(id)?)?;
    // Record order is an on-disk B-tree detail; every required property and
    // the real background identity must match, including opaque alias bytes.
    for record in expected.records {
        ensure!(
            store.records.contains(&record),
            "missing/changed Finder setting: {} {:?}",
            record.name,
            record.field
        );
    }
    let icons = store
        .records
        .iter()
        .find(|r| r.field == FieldKey::icvp)
        .context("missing icon view")?;
    let Value::IconViewProperties(icons) = &icons.value else {
        anyhow::bail!("invalid icon settings")
    };
    let PropertyList::Data(alias) = icons
        .unknown
        .get("backgroundImageAlias")
        .context("missing background alias")?
    else {
        anyhow::bail!("invalid background alias")
    };
    let alias = AliasRecord::from_bytes(alias)?;
    ensure!(
        alias.header.target_catalog_node_id.0 == id.file_cnid,
        "background alias points at another file"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
