//! Desktop Experience Service
//! Provides modern desktop features including window management, file organization,
//! virtual desktops, and productivity tools for RaeenOS.

pub mod rae_stacks;
pub mod rae_spaces;
pub mod snap_designer;
pub mod rae_finder;
pub mod rae_dock;
pub mod rae_start;
pub mod rae_spot;
pub mod window_manager;
pub mod file_operations;

use crate::services::contracts::graphics::GraphicsContract;
use alloc::vec::Vec;
use alloc::string::String;
use core::fmt;

/// Desktop service error types
#[derive(Debug, Clone)]
pub enum DesktopError {
    WindowNotFound,
    InvalidConfiguration,
    FileOperationFailed,
    ServiceUnavailable,
    PermissionDenied,
    InvalidArgument,
    IoError,
    FileNotFound,
}

impl fmt::Display for DesktopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DesktopError::WindowNotFound => write!(f, "Window not found"),
            DesktopError::InvalidConfiguration => write!(f, "Invalid configuration"),
            DesktopError::FileOperationFailed => write!(f, "File operation failed"),
            DesktopError::ServiceUnavailable => write!(f, "Service unavailable"),
            DesktopError::PermissionDenied => write!(f, "Permission denied"),
            DesktopError::InvalidArgument => write!(f, "Invalid argument"),
            DesktopError::IoError => write!(f, "I/O error"),
            DesktopError::FileNotFound => write!(f, "File not found"),
        }
    }
}

/// Main desktop service coordinator
pub struct DesktopService {
    pub rae_stacks: rae_stacks::RaeStacks,
    pub rae_spaces: rae_spaces::RaeSpaces,
    pub snap_designer: snap_designer::SnapDesigner,
    pub rae_finder: rae_finder::RaeFinder,
    pub rae_dock: rae_dock::RaeDock,
    pub rae_start: rae_start::RaeStart,
    pub rae_spot: rae_spot::RaeSpot,
    pub window_manager: window_manager::WindowManager,
    pub file_operations: file_operations::FileOperations,
}

impl DesktopService {
    /// Initialize the desktop service with all components
    pub fn new() -> Result<Self, DesktopError> {
        Ok(DesktopService {
            rae_stacks: rae_stacks::RaeStacks::new()?,
            rae_spaces: rae_spaces::RaeSpaces::new()?,
            snap_designer: snap_designer::SnapDesigner::new()?,
            rae_finder: rae_finder::RaeFinder::new()?,
            rae_dock: rae_dock::RaeDock::new()?,
            rae_start: rae_start::RaeStart::new()?,
            rae_spot: rae_spot::RaeSpot::new()?,
            window_manager: window_manager::WindowManager::new()?,
            file_operations: file_operations::FileOperations::new()?,
        })
    }

    /// Start all desktop services
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.rae_stacks.start()?;
        self.rae_spaces.start()?;
        self.snap_designer.start()?;
        self.rae_finder.start()?;
        self.rae_dock.start()?;
        self.rae_start.start()?;
        self.rae_spot.start()?;
        self.window_manager.start()?;
        self.file_operations.start()?;
        Ok(())
    }

    /// Stop all desktop services
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.rae_stacks.stop()?;
        self.rae_spaces.stop()?;
        self.snap_designer.stop()?;
        self.rae_finder.stop()?;
        self.rae_dock.stop()?;
        self.rae_start.stop()?;
        self.rae_spot.stop()?;
        self.window_manager.stop()?;
        self.file_operations.stop()?;
        Ok(())
    }
}