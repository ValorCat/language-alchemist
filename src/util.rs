use std::sync::Arc;

use eframe::egui::{Color32, Id, LayerId, Order, Response, Sense, Stroke, Ui};
use serde::{Deserialize, Serialize};

/// A Vec that is guaranteed to have at least one element.
#[derive(Default, Deserialize, Serialize)]
pub struct NonEmptyList<T> {
    pub head: T,
    pub tail: Vec<T>
}

#[allow(dead_code)]
impl<T> NonEmptyList<T> {
    /// Create a new NonEmptyList with `head` as the first element.
    pub fn new(head: T) -> Self {
        Self { head, tail: vec![] }
    }

    /// Return the number of elements in the list.
    pub fn len(&self) -> usize {
        self.tail.len() + 1
    }

    /// Return an iterator over the elements of this list.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(&self.tail)
    }

    /// Return a mutable iterator over the elements of this list.
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

    /// Return `true` if the edit mode is `View`.
    #[allow(dead_code)]
    pub fn is_view(&self) -> bool {
        matches!(self, Self::View)
    }

    /// Return `true` if the edit mode is `Edit`.
    pub fn is_edit(&self) -> bool {
        matches!(self, Self::Edit)
    }

    /// Return `true` if the edit mode is `Delete`.
    pub fn is_delete(&self) -> bool {
        matches!(self, Self::Delete)
    }
}

/// If in delete mode and the pointer is over the passed response, draw a red overlay
/// over the contents. Return true if the overlay is clicked.
pub fn draw_deletion_overlay(mode: EditMode, ui: &mut Ui, response: &Response) -> bool {
    draw_multipart_deletion_overlay(mode, ui, response, response)
}

/// If in delete mode and the pointer is over `click_area`, draw a red overlay over
/// `highlight_area`. Return true if `click_area` is clicked.
pub fn draw_multipart_deletion_overlay(mode: EditMode, ui: &mut Ui, click_area: &Response, highlight_area: &Response) -> bool {
    if mode.is_delete() && click_area.hovered() {
        ui.painter().rect_filled(highlight_area.rect.expand(2.0), 3.0, Color32::from_rgba_unmultiplied(255, 0, 0, 90));
        click_area.interact(Sense::click()).clicked()
    } else {
        false
    }
}

/// A reordering of an item in a list. Used for drag-and-drop reorderable lists.
pub struct Reordering {
    from_index: usize,
    to_index: usize
}

impl Reordering {
    pub fn apply<T>(&self, list: &mut Vec<T>) {
        let moved_item = list.remove(self.from_index);
        let to_index = if self.to_index <= self.from_index {
            self.to_index
        } else {
            self.to_index - 1
        };
        list.insert(to_index, moved_item);
    }
}

/// Render a drag-and-drop reorderable item. The passed in closure should return two Responses:
/// the first corresponding to the entire item, and the second corresponding to only the item's
/// "drag handle" (the portion that can be clicked and dragged to move the whole thing).
pub fn draw_reorderable(
    mode: EditMode, ui: &mut Ui, id: Id, index: usize, reordering: &mut Option<Reordering>,
    add_contents: impl FnOnce(&mut Ui) -> (Response, Response)
) -> bool {
    let (full_response, label_response) = if ui.memory(|mem| mem.is_being_dragged(id)) {
        // currently being dragged
        let layer_id = LayerId::new(Order::Tooltip, id);
        let (full_response, label_response) = ui.with_layer_id(layer_id, add_contents).inner;
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            ui.ctx().translate_layer(layer_id, pointer_pos - label_response.rect.center());
        }
        (full_response, label_response)
    } else {
        // not being dragged
        let (full_response, label_response) = add_contents(ui);
        if mode.is_edit() {
            ui.interact(label_response.rect, id, Sense::drag()).dnd_set_drag_payload(index);
        }
        (full_response, label_response)
    };
    draw_reorder_drop_area(ui, index, reordering, &full_response);
    draw_multipart_deletion_overlay(mode, ui, &label_response, &full_response)
}

/// Allow dropping a reorderable item on the given Response, and draw the drag-and-drop hint line
/// when such an item is hovered over it.
pub fn draw_reorder_drop_area(ui: &mut Ui, this_index: usize, reordering: &mut Option<Reordering>, response: &Response) {
    if let Some(from_index) =  response.dnd_hover_payload::<usize>() {
        draw_drag_hint_line(ui, response.rect.top());
        if ui.ctx().input(|input| input.pointer.any_released()) {
            *reordering = Some(Reordering {
                from_index: Arc::unwrap_or_clone(from_index),
                to_index: this_index
            });
        }
    }
}

fn draw_drag_hint_line(ui: &mut Ui, y_coord: f32) {
    const WIDTH: f32 = 0.8;
    let x = ui.available_rect_before_wrap().x_range();
    let y = y_coord - ui.spacing().item_spacing.y / 2.0 - WIDTH / 2.0;
    let stroke = Stroke::new(WIDTH, ui.visuals().widgets.hovered.fg_stroke.color);
    ui.painter().hline(x, y, stroke);
}