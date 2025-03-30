use std::ffi::{c_void, CString};
use std::path::Path;
use libc::{dlopen, dlsym, dlclose, RTLD_NOW};

/// Error type for dynamic library loading operations
#[derive(Debug, thiserror::Error)]
pub enum DLHandleError {
    /// Failed to load library
    #[error("Unable to load library: {0}")]
    LoadError(String),

    /// Failed to load symbol
    #[error("Unable to load symbol {1} from library {0}: {2}")]
    SymbolError(String, String, String),
}

/// Dynamic Loader wrapper
/// 
/// Allows to dynamically load a library and its symbols.
pub struct DLHandle {
    handle: *mut c_void,
    lib_name: String,
}

impl DLHandle {
    /// Load a library with the given mode flags
    pub fn new(lib_name: impl AsRef<Path>, mode: i32) -> Result<Self, DLHandleError> {
        let lib_name = lib_name.as_ref()
            .to_str()
            .ok_or_else(|| DLHandleError::LoadError("Invalid library path".to_string()))?;

        let c_lib_name = CString::new(lib_name)
            .map_err(|e| DLHandleError::LoadError(format!("Invalid library name: {}", e)))?;

        let handle = unsafe { dlopen(c_lib_name.as_ptr(), mode) };

        if handle.is_null() {
            let error = unsafe { CString::from_raw(dlerror() as *mut i8) }
                .into_string()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DLHandleError::LoadError(format!("{}: {}", lib_name, error)));
        }

        Ok(Self {
            handle,
            lib_name: lib_name.to_string(),
        })
    }

    /// Load one of the libraries listed in lib_names with the given mode flags
    pub fn try_load(lib_names: &[impl AsRef<Path>], mode: i32) -> Result<Self, DLHandleError> {
        let mut last_error = None;

        for lib_name in lib_names {
            match Self::new(lib_name, mode) {
                Ok(handle) => return Ok(handle),
                Err(e) => last_error = Some(e),
            }
        }

        Err(last_error.unwrap_or_else(|| DLHandleError::LoadError(
            format!("Unable to load any of: {}", 
                lib_names.iter()
                    .map(|p| p.as_ref().display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        )))
    }

    /// Load a symbol from the library
    pub fn load_symbol<T>(&self, symbol_name: &str) -> Result<T, DLHandleError> {
        let c_symbol_name = CString::new(symbol_name)
            .map_err(|e| DLHandleError::SymbolError(
                self.lib_name.clone(),
                symbol_name.to_string(),
                format!("Invalid symbol name: {}", e)
            ))?;

        let symbol = unsafe { dlsym(self.handle, c_symbol_name.as_ptr()) };

        if symbol.is_null() {
            let error = unsafe { CString::from_raw(dlerror() as *mut i8) }
                .into_string()
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DLHandleError::SymbolError(
                self.lib_name.clone(),
                symbol_name.to_string(),
                error
            ));
        }

        Ok(unsafe { std::mem::transmute_copy(&symbol) })
    }
}

impl Drop for DLHandle {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { dlclose(self.handle) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_dl_handle_creation() {
        let temp_dir = tempdir().unwrap();
        let lib_path = temp_dir.path().join("test.so");
        
        // Create a dummy shared library
        let mut file = File::create(&lib_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let handle = DLHandle::new(&lib_path, RTLD_NOW).unwrap();
        assert_eq!(handle.lib_name, lib_path.to_str().unwrap());
    }

    #[test]
    fn test_dl_handle_try_load() {
        let temp_dir = tempdir().unwrap();
        let lib_paths = vec![
            temp_dir.path().join("test1.so"),
            temp_dir.path().join("test2.so"),
        ];
        
        // Create a dummy shared library
        let mut file = File::create(&lib_paths[0]).unwrap();
        file.write_all(b"dummy content").unwrap();

        let handle = DLHandle::try_load(&lib_paths, RTLD_NOW).unwrap();
        assert_eq!(handle.lib_name, lib_paths[0].to_str().unwrap());
    }

    #[test]
    fn test_invalid_library() {
        let result = DLHandle::new("nonexistent.so", RTLD_NOW);
        assert!(matches!(result, Err(DLHandleError::LoadError(_))));
    }
} 