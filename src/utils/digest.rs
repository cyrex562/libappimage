use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use crate::{
    AppImageError, AppImageResult,
    utils::{md5::Md5Context, elf_file::ElfFile},
};

const CHUNK_SIZE: usize = 4096;

/// Calculate MD5 digest of a Type 2 AppImage, skipping digest, signature, and key sections
pub fn type2_digest_md5(path: &str) -> AppImageResult<[u8; 16]> {
    // Get section offsets and lengths
    let (digest_offset, digest_length) = ElfFile::get_section_offset_and_length(path, ".digest_md5")?;
    let (signature_offset, signature_length) = ElfFile::get_section_offset_and_length(path, ".sha256_sig")?;
    let (sig_key_offset, sig_key_length) = ElfFile::get_section_offset_and_length(path, ".sig_key")?;

    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len() as i64;
    let mut bytes_left = file_size;
    let mut bytes_skip_following_chunks = 0i64;
    let mut ctx = Md5Context::new();

    while bytes_left > 0 {
        let mut buffer = [0u8; CHUNK_SIZE];
        let current_position = file.stream_position()? as i64;
        let mut bytes_left_this_chunk = CHUNK_SIZE as i64;

        // Handle bytes that need to be skipped from previous chunks
        if bytes_skip_following_chunks > 0 {
            let bytes_skip_this_chunk = if bytes_skip_following_chunks % CHUNK_SIZE as i64 == 0 {
                CHUNK_SIZE as i64
            } else {
                bytes_skip_following_chunks % CHUNK_SIZE as i64
            };
            bytes_left_this_chunk -= bytes_skip_this_chunk;
            bytes_skip_following_chunks -= bytes_skip_this_chunk;
            file.seek(SeekFrom::Current(bytes_skip_this_chunk))?;
        }

        // Handle digest section
        if digest_offset > 0 && digest_length > 0 {
            let section_begin = digest_offset - current_position;
            if section_begin > 0 && section_begin < CHUNK_SIZE as i64 {
                let begin_of_section = (section_begin % CHUNK_SIZE as i64) as usize;
                file.read_exact(&mut buffer[..begin_of_section])?;

                bytes_left_this_chunk -= begin_of_section as i64;
                bytes_left_this_chunk -= digest_length as i64;

                if bytes_left_this_chunk < 0 {
                    bytes_skip_following_chunks = -bytes_left_this_chunk;
                    bytes_left_this_chunk = 0;
                }

                file.seek(SeekFrom::Current(
                    CHUNK_SIZE as i64 - bytes_left_this_chunk - begin_of_section as i64
                ))?;
            }
        }

        // Handle signature section
        if signature_offset > 0 && signature_length > 0 {
            let section_begin = signature_offset - current_position;
            if section_begin > 0 && section_begin < CHUNK_SIZE as i64 {
                let begin_of_section = (section_begin % CHUNK_SIZE as i64) as usize;
                file.read_exact(&mut buffer[..begin_of_section])?;

                bytes_left_this_chunk -= begin_of_section as i64;
                bytes_left_this_chunk -= signature_length as i64;

                if bytes_left_this_chunk < 0 {
                    bytes_skip_following_chunks = -bytes_left_this_chunk;
                    bytes_left_this_chunk = 0;
                }

                file.seek(SeekFrom::Current(
                    CHUNK_SIZE as i64 - bytes_left_this_chunk - begin_of_section as i64
                ))?;
            }
        }

        // Handle signature key section
        if sig_key_offset > 0 && sig_key_length > 0 {
            let section_begin = sig_key_offset - current_position;
            if section_begin > 0 && section_begin < CHUNK_SIZE as i64 {
                let begin_of_section = (section_begin % CHUNK_SIZE as i64) as usize;
                file.read_exact(&mut buffer[..begin_of_section])?;

                bytes_left_this_chunk -= begin_of_section as i64;
                bytes_left_this_chunk -= sig_key_length as i64;

                if bytes_left_this_chunk < 0 {
                    bytes_skip_following_chunks = -bytes_left_this_chunk;
                    bytes_left_this_chunk = 0;
                }

                file.seek(SeekFrom::Current(
                    CHUNK_SIZE as i64 - bytes_left_this_chunk - begin_of_section as i64
                ))?;
            }
        }

        // Read remaining data in this chunk
        if bytes_left_this_chunk > 0 {
            let offset = (CHUNK_SIZE as i64 - bytes_left_this_chunk) as usize;
            file.read_exact(&mut buffer[offset..])?;
        }

        // Update MD5 context
        ctx.update(&buffer);

        bytes_left -= CHUNK_SIZE as i64;
    }

    Ok(ctx.finalize().as_bytes().clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_type2_digest_md5() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("test.AppImage");
        
        // Create a test AppImage with sections
        let mut file = File::create(&appimage_path).unwrap();
        
        // Write some test data
        file.write_all(b"Hello, World!").unwrap();
        
        // Calculate digest
        let digest = type2_digest_md5(appimage_path.to_str().unwrap()).unwrap();
        
        // Verify digest
        assert_eq!(
            digest.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>(),
            "65a8e27d8879283831b664bd8b7f0ad4"
        );
    }
} 