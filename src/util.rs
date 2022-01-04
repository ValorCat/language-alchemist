use eframe::egui::Ui;
use serde::{Deserialize, Serialize};

/// A Vec that is guaranteed to have at least one element.
#[derive(Default, Deserialize, Serialize)]
pub struct NonEmptyList<T> {
    pub head: T,
    pub tail: Vec<T>
}

impl<T> NonEmptyList<T> {
    /// Create a new NonEmptyList with `head` as the first element.
    pub fn new(head: T) -> Self {
        Self { head, tail: vec![] }
    }

    /// Return an iterator over the elements of this list.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(&self.tail)
    }

    /// Return a mutable iterator over the elements of this list.
    #[allow(dead_code)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        std::iter::once(&mut self.head).chain(&mut self.tail)
    }

    /// Insert a new element as the head, pushing the previous head to the beginning of the tail.
    pub fn prepend(&mut self, element: T) {
        self.tail.insert(0, std::mem::replace(&mut self.head, element));
    }
}

/// The edit mode for some portion of the UI.
#[derive(Copy, Clone, PartialEq)]
pub enum EditMode {
    View, Edit, Delete
}

impl Default for EditMode {
    fn default() -> Self {
        Self::View
    }
}

impl EditMode {
    /// Render a small widget that allows changing the mode.
    pub fn draw_mode_picker(ui: &mut Ui, mode: &mut Self) {
        ui.horizontal(|ui| {
            ui.selectable_value(mode, Self::View, "View Mode");
            ui.selectable_value(mode, Self::Edit, "Edit Mode");
            ui.selectable_value(mode, Self::Delete, "Delete Mode");
        });
    }

    /// Returns `true` if the edit mode is `View`.
    #[allow(dead_code)]
    pub fn is_view(&self) -> bool {
        matches!(self, Self::View)
    }

    /// Returns `true` if the edit mode is `Edit`.
    pub fn is_edit(&self) -> bool {
        matches!(self, Self::Edit)
    }

    /// Returns `true` if the edit mode is `Delete`.
    pub fn is_delete(&self) -> bool {
        matches!(self, Self::Delete)
    }
}