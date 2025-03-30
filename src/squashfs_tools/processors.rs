use std::sync::Once;
use crate::error::SquashError;

/// Get the number of available processors on the system
/// 
/// This function returns the number of processors available on the system.
/// The result is cached after the first call.
/// 
/// # Returns
/// * `Result<usize, SquashError>` - Number of processors or error
pub fn get_nprocessors() -> Result<usize, SquashError> {
    static PROCESSORS: Once = Once::new();
    static mut PROCESSORS_COUNT: usize = 0;

    unsafe {
        PROCESSORS.call_once(|| {
            #[cfg(target_os = "linux")]
            {
                use std::os::unix::fs::OpenOptionsExt;
                use libc::{cpu_set_t, CPU_ZERO, CPU_COUNT, sched_getaffinity, _SC_NPROCESSORS_ONLN};
                use std::mem;

                let mut cpu_set: cpu_set_t = mem::zeroed();
                CPU_ZERO(&mut cpu_set);

                if sched_getaffinity(0, mem::size_of_val(&cpu_set), &mut cpu_set) == 0 {
                    PROCESSORS_COUNT = CPU_COUNT(&cpu_set);
                } else {
                    PROCESSORS_COUNT = libc::sysconf(_SC_NPROCESSORS_ONLN) as usize;
                }
            }

            #[cfg(target_os = "macos")]
            {
                use libc::{CTL_HW, HW_AVAILCPU, sysctl};
                use std::mem;

                let mut mib: [i32; 2] = [CTL_HW, HW_AVAILCPU];
                let mut len: usize = mem::size_of::<i32>();
                let mut count: i32 = 0;

                if sysctl(&mut mib, 2, &mut count, &mut len, std::ptr::null_mut(), 0) == -1 {
                    // Fallback to HW_NCPU if HW_AVAILCPU is not available
                    mib[1] = libc::HW_NCPU;
                    if sysctl(&mut mib, 2, &mut count, &mut len, std::ptr::null_mut(), 0) == -1 {
                        eprintln!("Failed to get number of available processors. Defaulting to 1");
                        PROCESSORS_COUNT = 1;
                    } else {
                        PROCESSORS_COUNT = count as usize;
                    }
                } else {
                    PROCESSORS_COUNT = count as usize;
                }
            }

            #[cfg(not(any(target_os = "linux", target_os = "macos")))]
            {
                eprintln!("Unsupported platform for processor count detection. Defaulting to 1");
                PROCESSORS_COUNT = 1;
            }
        });

        Ok(PROCESSORS_COUNT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nprocessors() {
        let result = get_nprocessors();
        assert!(result.is_ok());
        
        let count = result.unwrap();
        assert!(count > 0);
        assert!(count <= 1024); // Reasonable upper limit
    }

    #[test]
    fn test_get_nprocessors_cached() {
        let first = get_nprocessors().unwrap();
        let second = get_nprocessors().unwrap();
        assert_eq!(first, second);
    }
} 