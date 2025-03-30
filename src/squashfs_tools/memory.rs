use std::io;
use libc::{sysconf, _SC_PHYS_PAGES, _SC_PAGESIZE};
use crate::squashfs_tools::error::SquashError;

#[cfg(target_os = "linux")]
use libc::{sysinfo, sysinfo as SysInfo};

/// Get the physical memory size in megabytes
/// 
/// This function attempts to get the physical memory size using different methods:
/// 1. First tries sysconf(_SC_PHYS_PAGES) which relies on /proc being mounted
/// 2. If that fails, on Linux systems tries sysinfo()
/// 3. If both fail, returns 0
/// 
/// The function uses 64-bit integers to handle systems with PAE (Physical Address Extension)
/// where a 32-bit machine can have more than 4GB of physical memory.
pub fn get_physical_memory() -> i64 {
    // Try sysconf first
    let num_pages = unsafe { sysconf(_SC_PHYS_PAGES) };
    let page_size = unsafe { sysconf(_SC_PAGESIZE) };

    if num_pages != -1 && page_size != -1 {
        return (num_pages * page_size) >> 20;
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, try sysinfo as fallback
        let mut sys = unsafe { std::mem::zeroed::<sysinfo>() };
        let res = unsafe { sysinfo(&mut sys) };

        if res == 0 {
            return (sys.totalram as i64 * sys.mem_unit as i64) >> 20;
        }
    }

    0
}

/// Check if the requested memory size is usable
/// 
/// This function validates if the requested memory size is reasonable and can be used
/// by the application. It performs several checks:
/// 1. Verifies that physical memory can be determined
/// 2. Ensures the requested memory is not more than 75% of physical memory
/// 3. For 32-bit processes, ensures the requested memory doesn't exceed addressable limits
/// 
/// # Arguments
/// * `total_mem` - The requested memory size in MB
/// * `name` - The name of the application/component requesting the memory
/// 
/// # Returns
/// * `Result<(), SquashError>` - Ok if the memory size is valid, Err with details if invalid
pub fn check_usable_phys_mem(total_mem: i64, name: &str) -> Result<(), SquashError> {
    let mem = get_physical_memory();

    if mem == 0 {
        return Err(SquashError::MemoryError("Failed to get available system memory".to_string()));
    }

    // Calculate 75% of physical memory
    let max_mem = (mem >> 1) + (mem >> 2);

    if total_mem > max_mem {
        return Err(SquashError::MemoryError(format!(
            "Total memory requested ({}MB) is more than 75% of physical memory ({}MB).\n\
            {} uses memory to cache data from disk to optimise performance.\n\
            It is pointless to ask it to use more than this amount of memory, as this\n\
            causes thrashing and it is thus self-defeating.",
            total_mem, max_mem, name
        )));
    }

    // Check 32-bit process limitations
    if std::mem::size_of::<*const ()>() == 4 && total_mem > 2048 {
        return Err(SquashError::MemoryError(
            "Total memory requested may exceed maximum addressable memory by this process.\n\
            On 32-bit systems, processes are limited to 2-3GB of addressable memory.".to_string()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_physical_memory() {
        let memory = get_physical_memory();
        assert!(memory >= 0, "Physical memory should be non-negative");
        assert!(memory <= 1024 * 1024, "Physical memory should be reasonable (less than 1TB)");
    }

    #[test]
    fn test_check_usable_phys_mem() {
        let name = "test_app";
        
        // Test with reasonable memory size
        assert!(check_usable_phys_mem(1024, name).is_ok());
        
        // Test with too large memory size
        let result = check_usable_phys_mem(1024 * 1024 * 1024, name);
        assert!(result.is_err());
        
        // Test with zero memory
        let result = check_usable_phys_mem(0, name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_32bit_limitations() {
        let name = "test_app";
        
        // Test 32-bit process limitations
        if std::mem::size_of::<*const ()>() == 4 {
            let result = check_usable_phys_mem(2049, name);
            assert!(result.is_err());
            
            let result = check_usable_phys_mem(2048, name);
            assert!(result.is_ok());
        }
    }
} 