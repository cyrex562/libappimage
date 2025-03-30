pub mod error;
pub mod editor;
pub mod desktop_entry;
pub mod integrator;
pub mod constants;
pub mod manager;
pub mod thumbnailer;

pub use error::{DesktopIntegrationError, DesktopEntryEditError};
pub use editor::DesktopEntryEditor;
pub use desktop_entry::DesktopEntry;
pub use integrator::Integrator;
pub use constants::VENDOR_PREFIX;
pub use manager::IntegrationManager;
pub use thumbnailer::Thumbnailer; 