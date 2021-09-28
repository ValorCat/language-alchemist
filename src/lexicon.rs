use std::collections::HashMap;
use egui::{Button, Grid, Ui, popup};

pub type Lexicon = HashMap<String, String>;

/// The popup window for updating the lexicon.
pub struct LexiconEditWindow {
    original_native_phrase: Option<String>, // todo change to Option<&String>
    native_phrase: String,
    conlang_phrase: String,
    overwrite_warning: Option<String>
}

impl LexiconEditWindow {
    /// Create an instance of the edit window for modifying an existing entry.
    pub fn edit_entry(curr_native_phrase: &String, lexicon: &Lexicon) -> LexiconEditWindow {
        LexiconEditWindow {
            original_native_phrase: Some(curr_native_phrase.to_owned()),
            native_phrase: curr_native_phrase.to_owned(),
            conlang_phrase: lexicon.get(curr_native_phrase).unwrap().to_owned(),
            overwrite_warning: None
        }
    }

    /// Create an instance of the edit window for adding a new entry.
    pub fn new_entry() -> LexiconEditWindow {
        LexiconEditWindow {
            original_native_phrase: None,
            native_phrase: String::new(),
            conlang_phrase: String::new(),
            overwrite_warning: None
        }
    }

    /// Render the lexicon entry edit window.
    /// Return true if the window should be closed, or false otherwise.
    pub fn show(&mut self, ui: &mut Ui, conlang_name: &str, lexicon: &mut Lexicon) -> bool {
        let mut not_manual_close = true; // negative semantics required to pass to Window::open()
        let mut auto_close = false;
        egui::Window::new("Edit Lexicon")
            .collapsible(false)
            .resizable(false)
            .open(&mut not_manual_close)
            .default_width(100.0)
            .show(ui.ctx(), |ui| {
                Grid::new("edit lexicon")
                    .min_row_height(25.0)
                    .min_col_width(100.0)
                    .show(ui, self.draw_edit_fields(conlang_name, lexicon));
                ui.separator();
                ui.horizontal(|ui| {
                    match &self.original_native_phrase {
                        Some(original) => {
                            auto_close |= draw_delete_btn(ui, lexicon, original);
                            auto_close |= draw_apply_btn(ui, lexicon, original, &self.native_phrase, &self.conlang_phrase, self.can_edit_lexicon());
                        },
                        None => {
                            auto_close |= draw_new_btn(ui, lexicon, &self.native_phrase, &self.conlang_phrase, self.can_edit_lexicon());
                        }
                    }
                });
            });
        !not_manual_close || auto_close
    }

    /// Return a function that can be passed to Grid::show() to draw the lexicon editing text fields.
    fn draw_edit_fields<'a>(&'a mut self, conlang_name: &'a str, lexicon: &'a mut Lexicon) -> impl FnOnce(&mut Ui) + 'a {
        move |ui| {
            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                ui.label(format!("{}:", conlang_name));
            });
            ui.text_edit_singleline(&mut self.conlang_phrase);
            ui.end_row();
    
            ui.with_layout(egui::Layout::right_to_left(), |ui| {
                ui.label("English:");
            });
            let native_input = ui.text_edit_singleline(&mut self.native_phrase);
            ui.end_row();
    
            if native_input.changed() {
                self.overwrite_warning = lexicon.get(&self.native_phrase)
                    .map(|curr_word| format!("Already mapped to <{}>", curr_word));
                if self.overwrite_warning.is_none() {
                    ui.memory().close_popup();
                }
            }
            if let Some(warning) = &self.overwrite_warning {
                let warning_id = ui.make_persistent_id("lexicon warning");
                ui.memory().open_popup(warning_id);
                popup::popup_below_widget(ui, warning_id, &native_input, |ui| {
                    ui.set_min_width(100.0);
                    ui.label(warning);
                });
            }
        }
    }

    /// Return whether the contents of the edit window can be safely committed to the lexicon.
    fn can_edit_lexicon(&self) -> bool {
        self.overwrite_warning.is_none() && !self.native_phrase.is_empty()
    }
}

/// Draw a button that deletes the active lexicon entry.
fn draw_delete_btn(ui: &mut Ui, lexicon: &mut Lexicon, orig_native_phrase: &String) -> bool {
    let clicked = ui.button("Delete Entry").clicked();
    if clicked {
        lexicon.remove(orig_native_phrase);
    }
    clicked
}

/// Draw a button that updates the active lexicon entry.
fn draw_apply_btn(ui: &mut Ui, lexicon: &mut Lexicon, orig_native_phrase: &String, native_phrase: &String, conlang_phrase: &String, can_edit: bool) -> bool {
    let button = Button::new("Apply Changes").enabled(can_edit);
    let clicked = ui.add(button).clicked();
    if clicked {
        lexicon.insert(native_phrase.clone(), conlang_phrase.clone());
        if orig_native_phrase != native_phrase {
            lexicon.remove(orig_native_phrase);
        }
    }
    clicked
}

/// Draw a button that adds the active entry to the lexicon.
fn draw_new_btn(ui: &mut Ui, lexicon: &mut Lexicon, native_phrase: &String, conlang_phrase: &String, can_edit: bool) -> bool {
    let button = Button::new("Add Entry").enabled(can_edit);
    let clicked = ui.add(button).clicked();
    if clicked {
        lexicon.insert(native_phrase.clone(), conlang_phrase.clone());
    }
    clicked
}