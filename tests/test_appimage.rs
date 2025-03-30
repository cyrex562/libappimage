use std::path::Path;
use std::fs;
use std::io::Read;
use libappimage::AppImage;
use libappimage::AppImageFormat;
use libappimage::AppImageError;

const TEST_DATA_DIR: &str = "tests/data";

fn random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn get_tmp_file_path() -> String {
    format!("/tmp/libappimage-test-{}", random_string(16))
}

fn file_size(filename: &str) -> u64 {
    fs::metadata(filename).unwrap().len()
}

#[test]
fn test_instantiate() {
    // Test valid AppImages
    assert!(AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).is_ok());
    assert!(AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).is_ok());

    // Test invalid files
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("minimal.iso")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("none")).unwrap_err(),
        AppImageError::Io(_)
    ));
}

#[test]
fn test_get_format() {
    // Test valid AppImages
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_format(), AppImageFormat::Type1);

    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6_no_magic_bytes-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_format(), AppImageFormat::Type1);

    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_format(), AppImageFormat::Type2);

    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("appimaged-i686.AppImage")).unwrap();
    assert_eq!(appimage.get_format(), AppImageFormat::Type2);

    // Test invalid files
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("minimal.iso")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("Cura.desktop")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("non_existend_file")).unwrap_err(),
        AppImageError::Io(_)
    ));
}

#[test]
fn test_get_payload_offset() {
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_payload_offset().unwrap(), 28040);

    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6_no_magic_bytes-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_payload_offset().unwrap(), 28040);

    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    assert_eq!(appimage.get_payload_offset().unwrap(), 187784);

    assert!(matches!(
        AppImage::new(Path::new(TEST_DATA_DIR).join("elffile")).unwrap_err(),
        AppImageError::InvalidFormat(_)
    ));
}

#[test]
fn test_list_type1_entries() {
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    let mut expected = vec![
        "AppImageExtract.desktop",
        ".DirIcon",
        "AppImageExtract.png",
        "usr/bin/appimageextract",
        "AppRun",
        "usr/bin/xorriso",
        "usr/lib/libburn.so.4",
        "usr/lib/libisoburn.so.1",
        "usr/lib/libisofs.so.6",
    ];

    for file in appimage.files().unwrap() {
        if let Some(pos) = expected.iter().position(|x| x == &file) {
            expected.remove(pos);
        }
    }

    assert!(expected.is_empty());
}

#[test]
fn test_list_type2_entries() {
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let mut expected = vec![
        "echo.desktop",
        "AppRun",
        "usr",
        "usr/bin",
        "usr/bin/echo",
        "usr/share",
        "usr/share/applications",
        "usr/share/applications/echo.desktop",
        "usr/share/applications",
        "usr/share",
        "usr",
        "utilities-terminal.svg"
    ];

    for file in appimage.files().unwrap() {
        if let Some(pos) = expected.iter().position(|x| x == &file) {
            expected.remove(pos);
        }
    }

    assert!(expected.is_empty());
}

#[test]
fn test_type1_extract_file() {
    let tmp_file_path = get_tmp_file_path();
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    
    for file in appimage.files().unwrap() {
        if file == "AppImageExtract.desktop" {
            let mut file = fs::File::create(&tmp_file_path).unwrap();
            appimage.extract_file(&file, &tmp_file_path).unwrap();
            assert!(file_size(&tmp_file_path) > 0);
            break;
        }
    }

    fs::remove_file(&tmp_file_path).unwrap();
}

#[test]
fn test_type2_extract_file() {
    let tmp_file_path = get_tmp_file_path();
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    
    for file in appimage.files().unwrap() {
        if file == "usr/share/applications/echo.desktop" {
            let mut file = fs::File::create(&tmp_file_path).unwrap();
            appimage.extract_file(&file, &tmp_file_path).unwrap();
            assert!(file_size(&tmp_file_path) > 0);
            break;
        }
    }

    fs::remove_file(&tmp_file_path).unwrap();
}

#[test]
fn test_type1_read_file() {
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    let mut desktop_data = Vec::new();
    let mut icon_data = Vec::new();

    for file in appimage.files().unwrap() {
        if file == "AppImageExtract.desktop" {
            let mut reader = appimage.read_file(&file).unwrap();
            reader.read_to_end(&mut desktop_data).unwrap();
        }
        if file == ".DirIcon" {
            let mut reader = appimage.read_file(&file).unwrap();
            reader.read_to_end(&mut icon_data).unwrap();
        }
    }

    assert!(!desktop_data.is_empty());
    assert!(!icon_data.is_empty());
}

#[test]
fn test_type2_read_file() {
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let mut desktop_data = Vec::new();
    let mut icon_data = Vec::new();

    for file in appimage.files().unwrap() {
        if file == "usr/share/applications/echo.desktop" {
            let mut reader = appimage.read_file(&file).unwrap();
            reader.read_to_end(&mut desktop_data).unwrap();
        }
        if file == "utilities-terminal.svg" {
            let mut reader = appimage.read_file(&file).unwrap();
            reader.read_to_end(&mut icon_data).unwrap();
        }
    }

    assert!(!desktop_data.is_empty());
    assert!(!icon_data.is_empty());
}

#[test]
fn test_extract_entry_multiple_times() {
    let tmp_file_path = get_tmp_file_path();
    let appimage = AppImage::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    let mut files = appimage.files().unwrap();

    // Extract two times
    let mut file = fs::File::create(&tmp_file_path).unwrap();
    assert!(appimage.extract_file(&file, &tmp_file_path).is_ok());
    assert!(matches!(
        appimage.extract_file(&file, &tmp_file_path).unwrap_err(),
        AppImageError::OperationFailed(_)
    ));
    fs::remove_file(&tmp_file_path).unwrap();

    // Extract and read
    let mut file = fs::File::create(&tmp_file_path).unwrap();
    assert!(appimage.extract_file(&file, &tmp_file_path).is_ok());
    assert!(matches!(
        appimage.read_file(&tmp_file_path).unwrap_err(),
        AppImageError::OperationFailed(_)
    ));
    fs::remove_file(&tmp_file_path).unwrap();

    // Read two times
    let mut reader = appimage.read_file(&tmp_file_path).unwrap();
    let mut contents = Vec::new();
    assert!(reader.read_to_end(&mut contents).is_ok());
    assert!(matches!(
        appimage.read_file(&tmp_file_path).unwrap_err(),
        AppImageError::OperationFailed(_)
    ));

    // Read and extract
    let mut reader = appimage.read_file(&tmp_file_path).unwrap();
    let mut contents = Vec::new();
    assert!(reader.read_to_end(&mut contents).is_ok());
    let mut file = fs::File::create(&tmp_file_path).unwrap();
    assert!(matches!(
        appimage.extract_file(&file, &tmp_file_path).unwrap_err(),
        AppImageError::OperationFailed(_)
    ));
    fs::remove_file(&tmp_file_path).unwrap();
} 