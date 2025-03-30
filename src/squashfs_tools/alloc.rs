use std::alloc::{self, Layout};
use std::ffi::CString;
use std::fmt;
use std::ptr;

#[derive(Debug)]
pub enum AllocError {
    OutOfMemory,
    InvalidLayout,
    NullPointer,
    StringConversionError,
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AllocError::OutOfMemory => write!(f, "Out of memory"),
            AllocError::InvalidLayout => write!(f, "Invalid memory layout"),
            AllocError::NullPointer => write!(f, "Null pointer returned"),
            AllocError::StringConversionError => write!(f, "String conversion error"),
        }
    }
}

impl std::error::Error for AllocError {}

pub type Result<T> = std::result::Result<T, AllocError>;

/// Allocates memory for `num` elements of size `size` and initializes it to zero
pub fn calloc<T>(num: usize, size: usize) -> Result<*mut T> {
    let layout = Layout::array::<T>(num).map_err(|_| AllocError::InvalidLayout)?;
    let ptr = unsafe { alloc::alloc_zeroed(layout) as *mut T };
    
    if ptr.is_null() {
        Err(AllocError::OutOfMemory)
    } else {
        Ok(ptr)
    }
}

/// Allocates memory of size `size`
pub fn malloc<T>(size: usize) -> Result<*mut T> {
    let layout = Layout::array::<T>(1).map_err(|_| AllocError::InvalidLayout)?;
    let ptr = unsafe { alloc::alloc(layout) as *mut T };
    
    if ptr.is_null() {
        Err(AllocError::OutOfMemory)
    } else {
        Ok(ptr)
    }
}

/// Reallocates memory for `ptr` to new size `size`
pub unsafe fn realloc<T>(ptr: *mut T, size: usize) -> Result<*mut T> {
    if ptr.is_null() {
        return malloc(size);
    }

    let old_layout = Layout::array::<T>(1).map_err(|_| AllocError::InvalidLayout)?;
    let new_layout = Layout::array::<T>(1).map_err(|_| AllocError::InvalidLayout)?;
    
    let new_ptr = alloc::realloc(ptr as *mut u8, old_layout, new_layout.size()) as *mut T;
    
    if new_ptr.is_null() {
        Err(AllocError::OutOfMemory)
    } else {
        Ok(new_ptr)
    }
}

/// Duplicates a string
pub fn strdup(s: &str) -> Result<CString> {
    CString::new(s).map_err(|_| AllocError::StringConversionError)
}

/// Duplicates a string up to `n` characters
pub fn strndup(s: &str, n: usize) -> Result<CString> {
    let truncated = if n < s.len() {
        &s[..n]
    } else {
        s
    };
    CString::new(truncated).map_err(|_| AllocError::StringConversionError)
}

/// Formats a string using the provided format string and arguments
pub fn asprintf(format: &str, args: std::fmt::Arguments<'_>) -> Result<CString> {
    let formatted = format!("{}", args);
    CString::new(formatted).map_err(|_| AllocError::StringConversionError)
}

/// Frees allocated memory
pub unsafe fn free<T>(ptr: *mut T) {
    if !ptr.is_null() {
        let layout = Layout::array::<T>(1).unwrap();
        alloc::dealloc(ptr as *mut u8, layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calloc() {
        let ptr = calloc::<u32>(10, 1).unwrap();
        unsafe {
            for i in 0..10 {
                assert_eq!(*ptr.add(i), 0);
            }
            free(ptr);
        }
    }

    #[test]
    fn test_malloc() {
        let ptr = malloc::<u32>(1).unwrap();
        unsafe {
            *ptr = 42;
            assert_eq!(*ptr, 42);
            free(ptr);
        }
    }

    #[test]
    fn test_realloc() {
        let ptr = malloc::<u32>(1).unwrap();
        unsafe {
            *ptr = 42;
            let new_ptr = realloc(ptr, 2).unwrap();
            assert_eq!(*new_ptr, 42);
            free(new_ptr);
        }
    }

    #[test]
    fn test_strdup() {
        let s = "Hello, World!";
        let dup = strdup(s).unwrap();
        assert_eq!(dup.to_str().unwrap(), s);
    }

    #[test]
    fn test_strndup() {
        let s = "Hello, World!";
        let dup = strndup(s, 5).unwrap();
        assert_eq!(dup.to_str().unwrap(), "Hello");
    }

    #[test]
    fn test_asprintf() {
        let result = asprintf("Hello, {}!", "World").unwrap();
        assert_eq!(result.to_str().unwrap(), "Hello, World!");
    }
} 