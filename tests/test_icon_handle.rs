use std::fs;
use std::path::Path;
use tempfile::tempdir;
use libappimage::utils::IconHandle;

const TEST_DATA_DIR: &str = "tests/data";

#[test]
fn test_load_file_png() {
    let handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.png")).unwrap();
    
    assert_eq!(handle.format(), "png");
    assert_eq!(handle.size(), 48);
}

#[test]
fn test_save_png_unchanged() {
    let handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.png")).unwrap();
    
    assert_eq!(handle.format(), "png");
    assert_eq!(handle.size(), 48);

    let tmp_dir = tempdir().unwrap();
    let tmp_file_path = tmp_dir.path().join("tempfile");

    handle.save(&tmp_file_path, None).unwrap();
    assert!(tmp_dir.path().exists());
    assert!(!fs::read_dir(tmp_dir.path()).unwrap().next().unwrap().unwrap().path().is_empty());
}

#[test]
fn test_save_png_resized() {
    let mut handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.png")).unwrap();
    
    assert_eq!(handle.format(), "png");
    assert_eq!(handle.size(), 48);

    handle.set_size(256);

    let tmp_dir = tempdir().unwrap();
    let tmp_file_path = tmp_dir.path().join("tempfile");

    handle.save(&tmp_file_path, None).unwrap();

    let handle2 = IconHandle::from_file(&tmp_file_path).unwrap();
    assert_eq!(handle2.format(), "png");
    assert_eq!(handle2.size(), 256);
}

#[test]
fn test_load_file_svg() {
    let handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.svg")).unwrap();
    
    assert_eq!(handle.format(), "svg");
    assert_eq!(handle.size(), 48);
}

#[test]
fn test_save_svg() {
    let handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.svg")).unwrap();
    
    let tmp_dir = tempdir().unwrap();
    let tmp_file_path = tmp_dir.path().join("tempfile");

    handle.save(&tmp_file_path, None).unwrap();

    let handle2 = IconHandle::from_file(&tmp_file_path).unwrap();
    assert_eq!(handle2.format(), "svg");
    assert_eq!(handle2.size(), 48);
}

#[test]
fn test_save_svg_as_png() {
    let mut handle = IconHandle::from_file(Path::new(TEST_DATA_DIR).join("squashfs-root/utilities-terminal.svg")).unwrap();
    
    let tmp_dir = tempdir().unwrap();
    let tmp_file_path = tmp_dir.path().join("tempfile");

    handle.set_size(256);
    handle.save(&tmp_file_path, Some("png")).unwrap();

    let handle2 = IconHandle::from_file(&tmp_file_path).unwrap();
    assert_eq!(handle2.format(), "png");
    assert_eq!(handle2.size(), 256);
} 