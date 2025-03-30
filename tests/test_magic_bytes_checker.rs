use std::path::Path;
use libappimage::utils::MagicBytesChecker;

const TEST_DATA_DIR: &str = "tests/data";

#[test]
fn test_has_iso9660_signature() {
    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("minimal.iso")).unwrap();
    assert!(checker.has_iso9660_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert!(checker.has_iso9660_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap();
    assert!(!checker.has_iso9660_signature().unwrap());
}

#[test]
fn test_has_elf_signature() {
    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap();
    assert!(checker.has_elf_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    assert!(checker.has_elf_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap();
    assert!(!checker.has_elf_signature().unwrap());
}

#[test]
fn test_has_appimage_type1_signature() {
    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert!(checker.has_appimage_type1_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    assert!(!checker.has_appimage_type1_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap();
    assert!(!checker.has_appimage_type1_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap();
    assert!(!checker.has_appimage_type1_signature().unwrap());
}

#[test]
fn test_has_appimage_type2_signature() {
    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("appimagetool-x86_64.AppImage")).unwrap();
    assert!(checker.has_appimage_type2_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert!(!checker.has_appimage_type2_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap();
    assert!(!checker.has_appimage_type2_signature().unwrap());

    let mut checker = MagicBytesChecker::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap();
    assert!(!checker.has_appimage_type2_signature().unwrap());
} 