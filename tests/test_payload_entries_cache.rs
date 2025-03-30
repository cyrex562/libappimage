use std::path::Path;
use libappimage::utils::PayloadEntriesCache;
use libappimage::AppImage;

const TEST_DATA_DIR: &str = "tests/data";

#[test]
fn test_get_entries_paths() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let paths = entries_cache.get_entries_paths();
    let expected_paths = vec![
        ".DirIcon",
        "AppRun",
        "appimagetool.desktop",
        "appimagetool.svg",
        "usr",
        "usr/bin",
        "usr/bin/appimagetool",
        "usr/bin/desktop-file-validate",
        "usr/bin/file",
        "usr/bin/zsyncmake",
        "usr/lib",
        "usr/lib/appimagekit",
        "usr/lib/appimagekit/mksquashfs",
        "usr/share",
        "usr/share/metainfo",
        "usr/share/metainfo/appimagetool.appdata.xml",
    ];

    assert_eq!(paths, expected_paths);
}

#[test]
fn test_get_entry_type_regular() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let entry_type = entries_cache.get_entry_type("appimagetool.svg");
    assert_eq!(entry_type, PayloadEntryType::File);
}

#[test]
fn test_get_entry_type_link() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let entry_type = entries_cache.get_entry_type(".DirIcon");
    assert_eq!(entry_type, PayloadEntryType::Symlink);
}

#[test]
fn test_get_entry_type_dir() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let entry_type = entries_cache.get_entry_type("usr");
    assert_eq!(entry_type, PayloadEntryType::Directory);
}

#[test]
fn test_get_link_target1() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let target = entries_cache.get_entry_link_target(".DirIcon").unwrap();
    assert_eq!(target, "appimagetool.svg");
}

#[test]
fn test_get_link_target_not_link() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let result = entries_cache.get_entry_link_target("echo.destkop");
    assert!(result.is_err());
}

#[test]
fn test_get_missing_link_target() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let entries_cache = PayloadEntriesCache::new(&app_image);
    
    let result = entries_cache.get_entry_link_target("missing");
    assert!(result.is_err());
} 