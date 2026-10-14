//! Decode the actual binary store and check the former Finder layout's
//! literal contract, independently of encode/verify's shared construction.
use super::*;

fn identity() -> BackgroundIdentity {
    BackgroundIdentity {
        volume_created: 3_870_000_000,
        root_cnid: 2,
        file_created: 3_870_000_003,
        file_cnid: 42,
    }
}
fn value<'a>(records: &'a [Record], name: &str, field: FieldKey) -> &'a Value {
    &records
        .iter()
        .find(|r| r.name == name && r.field == field)
        .unwrap()
        .value
}

#[test]
fn headless_finder_layout_has_background_fixed_icons_and_no_auto_arrangement() {
    let bytes = encode(identity()).unwrap();
    assert_eq!(&bytes[..8], b"\0\0\0\x01Bud1");
    let store = ds_parser::parse(&bytes).unwrap();
    assert!(store.warnings.is_empty(), "{:?}", store.warnings);
    assert_eq!(store.records.len(), 6);
    let Value::BrowserWindowSettings(window) = value(&store.records, ".", FieldKey::bwsp) else {
        panic!("window settings")
    };
    assert_eq!(
        window.window_bounds,
        Some(WindowBounds {
            x: 200.0,
            y: 140.0,
            width: 660.0,
            height: 420.0
        })
    );
    for flag in [
        window.show_sidebar,
        window.show_toolbar,
        window.show_status_bar,
        window.show_path_bar,
        window.show_tab_view,
        window.container_show_sidebar,
    ] {
        assert_eq!(flag, Some(false));
    }
    let Value::IconViewProperties(icons) = value(&store.records, ".", FieldKey::icvp) else {
        panic!("icon settings")
    };
    assert_eq!(icons.arrange_by.as_deref(), Some("none"));
    assert_eq!(icons.background_type, Some(2));
    assert_eq!(icons.icon_size, Some(112.0));
    assert_eq!(icons.text_size, Some(13.0));
    assert_eq!(icons.label_on_bottom, Some(true));
    assert_eq!(icons.show_icon_preview, Some(false));
    assert_eq!(icons.show_item_info, Some(false));
    assert_eq!(
        value(&store.records, ".", FieldKey::icvl),
        &Value::Type(FourCC::from_array(*b"icnv"))
    );
    for (name, x) in [("Baylee.app", 170), ("Applications", 490)] {
        let Value::IconLocation(location) = value(&store.records, name, FieldKey::Iloc) else {
            panic!("icon position")
        };
        assert_eq!((location.x, location.y), (x, 205));
        assert_eq!(location.trailing, [255, 255, 255, 255, 255, 255, 0, 0]);
    }
    let PropertyList::Data(alias) = &icons.unknown["backgroundImageAlias"] else {
        panic!("background alias")
    };
    let alias = AliasRecord::from_bytes(alias).unwrap();
    assert_eq!(alias.header.version, 2);
    assert_eq!(alias.header.kind, AliasKind::File);
    assert_eq!(alias.header.volume_name, b"Baylee");
    assert_eq!(alias.header.target_name, b".background.tiff");
    assert_eq!(alias.header.disk_type, DiskType::Ejectable);
    assert_eq!(alias.header.volume_date.0, 3_870_000_000);
    assert_eq!(alias.header.target_date.0, 3_870_000_003);
    assert_eq!(alias.header.parent_catalog_node_id.0, 2);
    assert_eq!(alias.header.target_catalog_node_id.0, 42);
    // The serialized alias must not contain the builder's temporary mount.
    assert!(!bytes.windows(5).any(|part| part == b"/tmp/"));
}

#[test]
fn layout_check_rejects_missing_moved_icons_and_background_from_another_image() {
    let id = identity();
    let bytes = encode(id).unwrap();
    verify(&bytes, id).unwrap();
    assert!(
        verify(
            &bytes,
            BackgroundIdentity {
                file_cnid: 43,
                ..id
            }
        )
        .is_err()
    );
    assert!(
        verify(
            &bytes,
            BackgroundIdentity {
                volume_created: id.volume_created + 1,
                ..id
            }
        )
        .is_err()
    );
    let store = ds_parser::parse(&bytes).unwrap();
    for i in 0..store.records.len() {
        let mut records = store.records.clone();
        records.remove(i);
        assert!(verify(&ds_parser::write(&records).unwrap(), id).is_err());
    }
    let mut records = store.records;
    records
        .iter_mut()
        .find(|r| r.name == "Applications")
        .unwrap()
        .value = IconLocation::new(170, 205).into();
    assert!(verify(&ds_parser::write(&records).unwrap(), id).is_err());
    assert!(verify(b"not a DS_Store", id).is_err());
}
