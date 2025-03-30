use std::path::Path;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::error::Error;
use std::fmt;

#[cfg(target_endian = "little")]
const ELFDATANATIVE: u8 = 1; // ELFDATA2LSB
#[cfg(target_endian = "big")]
const ELFDATANATIVE: u8 = 2; // ELFDATA2MSB
#[cfg(not(any(target_endian = "little", target_endian = "big")))]
compile_error!("Unknown machine endian");

// ELF constants
const EI_NIDENT: usize = 16;
const EI_DATA: usize = 5;
const EI_CLASS: usize = 4;
const ELFCLASS32: u8 = 1;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ELFDATA2MSB: u8 = 2;

#[derive(Debug)]
pub enum ElfFileError {
    Io(io::Error),
    InvalidElf(String),
    UnsupportedElf(String),
}

impl fmt::Display for ElfFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElfFileError::Io(e) => write!(f, "IO error: {}", e),
            ElfFileError::InvalidElf(e) => write!(f, "Invalid ELF file: {}", e),
            ElfFileError::UnsupportedElf(e) => write!(f, "Unsupported ELF file: {}", e),
        }
    }
}

impl Error for ElfFileError {}

impl From<io::Error> for ElfFileError {
    fn from(err: io::Error) -> Self {
        ElfFileError::Io(err)
    }
}

/// Utility struct to read ELF files. Not meant to be feature complete.
pub struct ElfFile {
    path: String,
    ehdr: Elf64_Ehdr,
}

#[repr(C)]
struct Elf64_Ehdr {
    e_ident: [u8; EI_NIDENT],
    e_shoff: u64,
    e_shentsize: u16,
    e_shnum: u16,
}

impl ElfFile {
    /// Create a new ElfFile instance
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ElfFileError> {
        let path = path.as_ref()
            .to_str()
            .ok_or_else(|| ElfFileError::InvalidElf("Invalid file path".to_string()))?
            .to_string();

        let mut ehdr = Elf64_Ehdr {
            e_ident: [0; EI_NIDENT],
            e_shoff: 0,
            e_shentsize: 0,
            e_shnum: 0,
        };

        Ok(Self { path, ehdr })
    }

    /// Calculate the size of an ELF file on disk based on the information in its header
    /// 
    /// Example:
    /// ```
    /// ls -l   126584
    /// 
    /// Calculation using the values also reported by readelf -h:
    /// Start of section headers	e_shoff		124728
    /// Size of section headers		e_shentsize	64
    /// Number of section headers	e_shnum		29
    /// 
    /// e_shoff + ( e_shentsize * e_shnum ) =	126584
    /// ```
    pub fn get_size(&mut self) -> Result<i64, ElfFileError> {
        let mut file = File::open(&self.path)?;

        // Read ELF identification
        file.read_exact(&mut self.ehdr.e_ident)?;

        // Validate ELF data encoding
        if self.ehdr.e_ident[EI_DATA] != ELFDATA2LSB && self.ehdr.e_ident[EI_DATA] != ELFDATA2MSB {
            return Err(ElfFileError::InvalidElf(
                format!("Unknown ELF data order: {}", self.ehdr.e_ident[EI_DATA])
            ));
        }

        // Read ELF header based on class
        let size = match self.ehdr.e_ident[EI_CLASS] {
            ELFCLASS32 => self.read_elf32(&mut file)?,
            ELFCLASS64 => self.read_elf64(&mut file)?,
            class => return Err(ElfFileError::UnsupportedElf(
                format!("Unknown ELF class: {}", class)
            )),
        };

        Ok(size)
    }

    fn file16_to_cpu(&self, val: u16) -> u16 {
        if self.ehdr.e_ident[EI_DATA] != ELFDATANATIVE {
            val.swap_bytes()
        } else {
            val
        }
    }

    fn file32_to_cpu(&self, val: u32) -> u32 {
        if self.ehdr.e_ident[EI_DATA] != ELFDATANATIVE {
            val.swap_bytes()
        } else {
            val
        }
    }

    fn file64_to_cpu(&self, val: u64) -> u64 {
        if self.ehdr.e_ident[EI_DATA] != ELFDATANATIVE {
            val.swap_bytes()
        } else {
            val
        }
    }

    fn read_elf32(&mut self, file: &mut File) -> Result<i64, ElfFileError> {
        #[repr(C)]
        struct Elf32_Ehdr {
            e_ident: [u8; EI_NIDENT],
            e_shoff: u32,
            e_shentsize: u16,
            e_shnum: u16,
        }

        #[repr(C)]
        struct Elf32_Shdr {
            sh_offset: u32,
            sh_size: u32,
        }

        let mut ehdr32 = Elf32_Ehdr {
            e_ident: [0; EI_NIDENT],
            e_shoff: 0,
            e_shentsize: 0,
            e_shnum: 0,
        };

        let mut shdr32 = Elf32_Shdr {
            sh_offset: 0,
            sh_size: 0,
        };

        // Read ELF header
        file.seek(SeekFrom::Start(0))?;
        unsafe {
            let ptr = &mut ehdr32 as *mut _ as *mut u8;
            file.read_exact(std::slice::from_raw_parts_mut(ptr, std::mem::size_of::<Elf32_Ehdr>()))?;
        }

        // Convert header values
        self.ehdr.e_shoff = self.file32_to_cpu(ehdr32.e_shoff) as u64;
        self.ehdr.e_shentsize = self.file16_to_cpu(ehdr32.e_shentsize);
        self.ehdr.e_shnum = self.file16_to_cpu(ehdr32.e_shnum);

        // Read last section header
        let last_shdr_offset = self.ehdr.e_shoff + (self.ehdr.e_shentsize as u64 * (self.ehdr.e_shnum as u64 - 1));
        file.seek(SeekFrom::Start(last_shdr_offset))?;
        unsafe {
            let ptr = &mut shdr32 as *mut _ as *mut u8;
            file.read_exact(std::slice::from_raw_parts_mut(ptr, std::mem::size_of::<Elf32_Shdr>()))?;
        }

        // Calculate file size
        let sht_end = self.ehdr.e_shoff + (self.ehdr.e_shentsize as u64 * self.ehdr.e_shnum as u64);
        let last_section_end = self.file32_to_cpu(shdr32.sh_offset) as u64 + self.file32_to_cpu(shdr32.sh_size) as u64;
        
        Ok(std::cmp::max(sht_end, last_section_end) as i64)
    }

    fn read_elf64(&mut self, file: &mut File) -> Result<i64, ElfFileError> {
        #[repr(C)]
        struct Elf64_Shdr {
            sh_offset: u64,
            sh_size: u64,
        }

        let mut shdr64 = Elf64_Shdr {
            sh_offset: 0,
            sh_size: 0,
        };

        // Read ELF header
        file.seek(SeekFrom::Start(0))?;
        unsafe {
            let ptr = &mut self.ehdr as *mut _ as *mut u8;
            file.read_exact(std::slice::from_raw_parts_mut(ptr, std::mem::size_of::<Elf64_Ehdr>()))?;
        }

        // Convert header values
        self.ehdr.e_shoff = self.file64_to_cpu(self.ehdr.e_shoff);
        self.ehdr.e_shentsize = self.file16_to_cpu(self.ehdr.e_shentsize);
        self.ehdr.e_shnum = self.file16_to_cpu(self.ehdr.e_shnum);

        // Read last section header
        let last_shdr_offset = self.ehdr.e_shoff + (self.ehdr.e_shentsize as u64 * (self.ehdr.e_shnum as u64 - 1));
        file.seek(SeekFrom::Start(last_shdr_offset))?;
        unsafe {
            let ptr = &mut shdr64 as *mut _ as *mut u8;
            file.read_exact(std::slice::from_raw_parts_mut(ptr, std::mem::size_of::<Elf64_Shdr>()))?;
        }

        // Calculate file size
        let sht_end = self.ehdr.e_shoff + (self.ehdr.e_shentsize as u64 * self.ehdr.e_shnum as u64);
        let last_section_end = self.file64_to_cpu(shdr64.sh_offset) + self.file64_to_cpu(shdr64.sh_size);
        
        Ok(std::cmp::max(sht_end, last_section_end) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_elf_file_creation() {
        let temp_dir = tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");
        
        // Create a dummy ELF file
        let mut file = File::create(&elf_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let mut elf_file = ElfFile::new(&elf_path).unwrap();
        assert!(elf_file.get_size().is_ok());
    }

    #[test]
    fn test_invalid_elf_file() {
        let temp_dir = tempdir().unwrap();
        let elf_path = temp_dir.path().join("test.elf");
        
        // Create an invalid ELF file
        let mut file = File::create(&elf_path).unwrap();
        file.write_all(b"invalid elf content").unwrap();

        let mut elf_file = ElfFile::new(&elf_path).unwrap();
        assert!(elf_file.get_size().is_err());
    }
} 