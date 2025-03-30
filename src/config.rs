/// Version information for the libappimage crate
pub mod version {
    /// Major version number
    pub const MAJOR: u32 = 0;
    
    /// Minor version number
    pub const MINOR: u32 = 1;
    
    /// Patch version number
    pub const PATCH: u32 = 0;
    
    /// Version suffix (e.g., "-alpha", "-beta", etc.)
    pub const SUFFIX: &str = "";
    
    /// Full version string
    pub const VERSION: &str = {
        const fn make_version() -> &'static str {
            concat!(
                stringify!(MAJOR),
                ".",
                stringify!(MINOR),
                ".",
                stringify!(PATCH),
                ""
            )
        }
        make_version()
    };
}

/// Feature flags for the libappimage crate
pub mod features {
    /// Whether desktop integration features are enabled
    #[cfg(feature = "desktop-integration")]
    pub const DESKTOP_INTEGRATION_ENABLED: bool = true;
    
    #[cfg(not(feature = "desktop-integration"))]
    pub const DESKTOP_INTEGRATION_ENABLED: bool = false;
    
    /// Whether thumbnailer features are enabled
    #[cfg(feature = "thumbnailer")]
    pub const THUMBNAILER_ENABLED: bool = true;
    
    #[cfg(not(feature = "thumbnailer"))]
    pub const THUMBNAILER_ENABLED: bool = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_format() {
        assert_eq!(
            version::VERSION,
            format!("{}.{}.{}{}", 
                version::MAJOR,
                version::MINOR,
                version::PATCH,
                version::SUFFIX
            )
        );
    }

    #[test]
    fn test_feature_flags() {
        // These tests will pass or fail based on the features enabled at compile time
        #[cfg(feature = "desktop-integration")]
        assert!(features::DESKTOP_INTEGRATION_ENABLED);
        
        #[cfg(not(feature = "desktop-integration"))]
        assert!(!features::DESKTOP_INTEGRATION_ENABLED);
        
        #[cfg(feature = "thumbnailer")]
        assert!(features::THUMBNAILER_ENABLED);
        
        #[cfg(not(feature = "thumbnailer"))]
        assert!(!features::THUMBNAILER_ENABLED);
    }
} 