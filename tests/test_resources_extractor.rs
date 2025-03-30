use std::path::Path;
use std::collections::HashMap;
use libappimage::utils::ResourcesExtractor;
use libappimage::AppImage;
use tempfile::tempdir;

const TEST_DATA_DIR: &str = "tests/data";

#[test]
fn test_get_desktop_entry_path() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    let extractor = ResourcesExtractor::new(app_image);

    let desktop_entry_path = extractor.get_desktop_entry_path().unwrap();
    assert_eq!(desktop_entry_path, "appimagetool.desktop");
}

#[test]
fn test_get_icon_paths() {
    // Note: This test is commented out in the C++ version as it requires editing the Echo AppImage
    // We'll keep it commented out here as well
    /*
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let extractor = ResourcesExtractor::new(app_image);

    let icon_file_paths = extractor.get_icon_file_paths("utilities-terminal");
    let expected = vec!["usr/share/icons/hicolor/scalable/utilities-terminal.svg"];
    assert_eq!(icon_file_paths, expected);
    */
}

#[test]
fn test_extract_entries_to() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let extractor = ResourcesExtractor::new(app_image);

    let temp_dir = tempdir().unwrap();
    let temp_file_path = temp_dir.path().join("libappimage-0000-0000-0000-0000");

    let mut targets = HashMap::new();
    targets.insert(".DirIcon".to_string(), temp_file_path.clone());
    
    extractor.extract_to(&targets).unwrap();

    assert!(temp_file_path.exists());
    assert!(temp_file_path.metadata().unwrap().len() > 0);
}

#[test]
fn test_extract_one() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let extractor = ResourcesExtractor::new(app_image);

    let file_data = extractor.extract("echo.desktop").unwrap();
    assert!(!file_data.is_empty());

    let result = extractor.extract("missing_file");
    assert!(result.is_err());
}

#[test]
fn test_extract_many() {
    let app_image = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let extractor = ResourcesExtractor::new(app_image);

    let paths = vec!["echo.desktop".to_string(), ".DirIcon".to_string()];
    let files_data = extractor.extract_multiple(&paths).unwrap();

    assert!(!files_data.is_empty());
    assert!(files_data.values().all(|data| !data.is_empty()));

    let result = extractor.extract_multiple(&["missing_file".to_string()]);
    assert!(result.is_err());
} 