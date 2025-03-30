//! Lightweight ELF file format definitions
//! 
//! Based on the Linux kernel ELF definitions
//! Copyright (C) 2017 Linus Torvalds
//! Modified work Copyright (C) 2017 @teras
//! 
//! Original work found here:
//! https://github.com/torvalds/linux/blob/master/include/uapi/linux/elf.h

use std::fmt;

/// Size of the ELF identification array
pub const EI_NIDENT: usize = 16;

/// ELF class constants
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

/// ELF data encoding constants
pub const ELFDATA2LSB: u8 = 1;
pub const ELFDATA2MSB: u8 = 2;

/// ELF identification indices
pub const EI_CLASS: usize = 4;
pub const EI_DATA: usize = 5;

/// 32-bit ELF header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf32_Ehdr {
    /// ELF identification
    pub e_ident: [u8; EI_NIDENT],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u32,
    /// Program header table file offset
    pub e_phoff: u32,
    /// Section header table file offset
    pub e_shoff: u32,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size in bytes
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section header string table index
    pub e_shstrndx: u16,
}

/// 64-bit ELF header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64_Ehdr {
    /// ELF identification
    pub e_ident: [u8; EI_NIDENT],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size in bytes
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section header string table index
    pub e_shstrndx: u16,
}

/// 32-bit ELF section header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf32_Shdr {
    /// Section name, index in string tbl
    pub sh_name: u32,
    /// Type of section
    pub sh_type: u32,
    /// Miscellaneous section attributes
    pub sh_flags: u32,
    /// Section virtual addr at execution
    pub sh_addr: u32,
    /// Section file offset
    pub sh_offset: u32,
    /// Size of section in bytes
    pub sh_size: u32,
    /// Index of another section
    pub sh_link: u32,
    /// Additional section information
    pub sh_info: u32,
    /// Section alignment
    pub sh_addralign: u32,
    /// Entry size if section holds table
    pub sh_entsize: u32,
}

/// 64-bit ELF section header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64_Shdr {
    /// Section name, index in string tbl
    pub sh_name: u32,
    /// Type of section
    pub sh_type: u32,
    /// Miscellaneous section attributes
    pub sh_flags: u64,
    /// Section virtual addr at execution
    pub sh_addr: u64,
    /// Section file offset
    pub sh_offset: u64,
    /// Size of section in bytes
    pub sh_size: u64,
    /// Index of another section
    pub sh_link: u32,
    /// Additional section information
    pub sh_info: u32,
    /// Section alignment
    pub sh_addralign: u64,
    /// Entry size if section holds table
    pub sh_entsize: u64,
}

/// 32-bit ELF note header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf32_Nhdr {
    /// Name size
    pub n_namesz: u32,
    /// Content size
    pub n_descsz: u32,
    /// Content type
    pub n_type: u32,
}

/// Helper trait for ELF headers
pub trait ElfHeader {
    /// Get the ELF class (32-bit or 64-bit)
    fn get_class(&self) -> u8;
    
    /// Get the ELF data encoding (little-endian or big-endian)
    fn get_data_encoding(&self) -> u8;
}

impl ElfHeader for Elf32_Ehdr {
    fn get_class(&self) -> u8 {
        self.e_ident[EI_CLASS]
    }
    
    fn get_data_encoding(&self) -> u8 {
        self.e_ident[EI_DATA]
    }
}

impl ElfHeader for Elf64_Ehdr {
    fn get_class(&self) -> u8 {
        self.e_ident[EI_CLASS]
    }
    
    fn get_data_encoding(&self) -> u8 {
        self.e_ident[EI_DATA]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_header_sizes() {
        assert_eq!(std::mem::size_of::<Elf32_Ehdr>(), 52);
        assert_eq!(std::mem::size_of::<Elf64_Ehdr>(), 64);
        assert_eq!(std::mem::size_of::<Elf32_Shdr>(), 40);
        assert_eq!(std::mem::size_of::<Elf64_Shdr>(), 64);
        assert_eq!(std::mem::size_of::<Elf32_Nhdr>(), 12);
    }

    #[test]
    fn test_elf_header_alignment() {
        assert_eq!(std::mem::align_of::<Elf32_Ehdr>(), 4);
        assert_eq!(std::mem::align_of::<Elf64_Ehdr>(), 8);
        assert_eq!(std::mem::align_of::<Elf32_Shdr>(), 4);
        assert_eq!(std::mem::align_of::<Elf64_Shdr>(), 8);
        assert_eq!(std::mem::align_of::<Elf32_Nhdr>(), 4);
    }
} 