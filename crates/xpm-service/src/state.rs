//! Application state management.

use xpm_core::{
    operation::OperationResult,
    package::{Package, PackageBackend, SearchResult, UpdateInfo},
};

/// Current view in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewState {
    /// Showing installed packages.
    #[default]
    Installed,
    /// Showing available updates.
    Updates,
    /// Showing search results.
    Search,
    /// Showing flatpak apps.
    Flatpak,
    /// Showing settings.
    Settings,
    /// Showing system maintenance.
    Maintenance,
}

/// Filter options for package lists.
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Text search filter.
    pub search_text: String,
    /// Filter by backend.
    pub backend: Option<PackageBackend>,
    /// Show only explicitly installed packages.
    pub explicit_only: bool,
    /// Show only packages with updates.
    pub updates_only: bool,
}

/// Application state.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current view.
    pub view: ViewState,
    /// Current filter options.
    pub filter: FilterOptions,
    /// Currently selected package (for details view).
    pub selected_package: Option<String>,
    /// List of installed packages.
    pub installed_packages: Vec<Package>,
    /// List of available updates.
    pub updates: Vec<UpdateInfo>,
    /// Search results.
    pub search_results: Vec<SearchResult>,
    /// Last operation result.
    pub last_operation: Option<OperationResult>,
    /// Whether an operation is in progress.
    pub operation_in_progress: bool,
    /// Error message to display.
    pub error_message: Option<String>,
}

impl AppState {
    /// Creates a new application state.
    pub fn new() -> Self {
        Self {
            view: ViewState::Installed,
            filter: FilterOptions::default(),
            selected_package: None,
            installed_packages: Vec::new(),
            updates: Vec::new(),
            search_results: Vec::new(),
            last_operation: None,
            operation_in_progress: false,
            error_message: None,
        }
    }

    /// Sets the current view.
    pub fn set_view(&mut self, view: ViewState) {
        self.view = view;
        self.selected_package = None;
    }

    /// Sets the search filter.
    pub fn set_search(&mut self, text: String) {
        self.filter.search_text = text;
    }

    /// Clears any error message.
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Sets an error message.
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Filters installed packages based on current filter options.
    pub fn filtered_installed(&self) -> Vec<&Package> {
        self.installed_packages
            .iter()
            .filter(|p| {
                // Text filter.
                if !self.filter.search_text.is_empty() {
                    let search = self.filter.search_text.to_lowercase();
                    if !p.name.to_lowercase().contains(&search)
                        && !p.description.to_lowercase().contains(&search)
                    {
                        return false;
                    }
                }

                // Backend filter.
                if let Some(backend) = self.filter.backend {
                    if p.backend != backend {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Gets the count of installed packages by backend.
    pub fn installed_count_by_backend(&self, backend: PackageBackend) -> usize {
        self.installed_packages
            .iter()
            .filter(|p| p.backend == backend)
            .count()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
