use std::path::Path;
use libappimage::utils::ElfFile;

const TEST_DATA_DIR: &str = "tests/data";

#[test]
fn test_get_size() {
    let mut elf_file = ElfFile::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6-x86_64.AppImage")).unwrap();
    assert_eq!(elf_file.get_size().unwrap(), 28040);

    let mut elf_file = ElfFile::new(Path::new(TEST_DATA_DIR).join("AppImageExtract_6_no_magic_bytes-x86_64.AppImage")).unwrap();
    assert_eq!(elf_file.get_size().unwrap(), 28040);

    let mut elf_file = ElfFile::new(Path::new(TEST_DATA_DIR).join("Echo-x86_64.AppImage")).unwrap();
    assert_eq!(elf_file.get_size().unwrap(), 187784);

    let mut elf_file = ElfFile::new(Path::new(TEST_DATA_DIR).join("appimaged-i686.AppImage")).unwrap();
    assert_eq!(elf_file.get_size().unwrap(), 91148);
} 