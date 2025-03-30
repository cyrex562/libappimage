use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;
use std::ptr;

/// A thread-safe wrapper for read entries that supports atomic swap operations
pub struct ReadEntryPtr<T> {
    inner: AtomicPtr<T>,
    #[cfg(not(feature = "use_atomic_exchange"))]
    mutex: Mutex<()>,
}

impl<T> ReadEntryPtr<T> {
    /// Creates a new ReadEntryPtr with the given value
    pub fn new(value: *mut T) -> Self {
        ReadEntryPtr {
            inner: AtomicPtr::new(value),
            #[cfg(not(feature = "use_atomic_exchange"))]
            mutex: Mutex::new(()),
        }
    }

    /// Atomically swaps the current value with None and returns the previous value
    pub fn swap(&self) -> *mut T {
        #[cfg(feature = "use_atomic_exchange")]
        {
            self.inner.swap(ptr::null_mut(), Ordering::SeqCst)
        }

        #[cfg(not(feature = "use_atomic_exchange"))]
        {
            let _guard = self.mutex.lock().unwrap();
            let value = self.inner.load(Ordering::SeqCst);
            self.inner.store(ptr::null_mut(), Ordering::SeqCst);
            value
        }
    }

    /// Gets the current value without modifying it
    pub fn get(&self) -> *mut T {
        self.inner.load(Ordering::SeqCst)
    }

    /// Sets a new value
    pub fn set(&self, value: *mut T) {
        self.inner.store(value, Ordering::SeqCst);
    }
}

impl<T> Default for ReadEntryPtr<T> {
    fn default() -> Self {
        Self::new(ptr::null_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_swap() {
        let mut value = 42;
        let ptr = ReadEntryPtr::new(&mut value);
        
        // Test initial value
        assert_eq!(unsafe { *ptr.get() }, 42);
        
        // Test swap
        let old_value = ptr.swap();
        assert_eq!(unsafe { *old_value }, 42);
        assert!(ptr.get().is_null());
        
        // Test setting new value
        let mut new_value = 24;
        ptr.set(&mut new_value);
        assert_eq!(unsafe { *ptr.get() }, 24);
    }

    #[test]
    fn test_null_swap() {
        let ptr = ReadEntryPtr::<i32>::default();
        assert!(ptr.get().is_null());
        
        let result = ptr.swap();
        assert!(result.is_null());
        assert!(ptr.get().is_null());
    }
} 