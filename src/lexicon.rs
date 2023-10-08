use std::collections::HashMap;
use eframe::egui::{Align, Button, Checkbox, Grid, Layout, ScrollArea, TextEdit, Ui, Window, popup};
use crate::Language;

pub type Lexicon = HashMap<String, String>;

/// The popup window for updating the lexicon.
pub struct LexiconEditWindow {
    original_native_phrase: Option<String>, // todo change to Option<&String>
    native_phrase: String,
    conlang_phrase: String,
    overwrite_warning: Option<String>
}

/// The toggleable mode for the lexicon search field.
#[derive(Default, PartialEq)]
pub enum LexiconSearchMode {
    #[default] Native,
    Conlang
}

impl LexiconSearchMode {
    fn matches(&self, native: &str, conlang: &str, search: &str) -> bool {
        match self {
            LexiconSearchMode::Native => native.contains(search),
            LexiconSearchMode::Conlang => conlang.contains(search)
        }
    }
}

/// Render contents of the 'lexicon' tab.
pub fn draw_lexicon_tab(ui: &mut Ui, curr_lang: &mut Language, lexicon_edit_win: &mut Option<LexiconEditWindow>) {
    // add +10 pts vertical spacing between rows in this tab
    ui.spacing_mut().item_spacing += (0.0, 10.0).into();

    let label = format!("Allow homonyms ({} currently)", curr_lang.num_homonyms);
    let tooltip = "Homonyms are words with the same spelling or pronunciation, but different \
        meanings. Natural languages often have many homonyms, but constructed languages rarely do \
        to avoid confusion.";
    ui.add_enabled(false, Checkbox::new(&mut curr_lang.allow_homonyms, label))
        .on_hover_text(tooltip)
        .on_disabled_hover_text("Not yet implemented");
    
    ui.separator();

    // table search controls
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut curr_lang.lexicon_search)
            .hint_text("Search...")
            .desired_width(120.0));
        ui.label("Search by:");
        ui.selectable_value(&mut curr_lang.lexicon_search_mode, LexiconSearchMode::Native, "English");
        ui.selectable_value(&mut curr_lang.lexicon_search_mode, LexiconSearchMode::Conlang, &curr_lang.name);
    });

    // draw the lexicon table
    ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            // remove the extra 10 pts of spacing within the table
            ui.spacing_mut().item_spacing.y -= 10.0;
            
            // draw the table header
            ui.heading(format!("{} to {} Lexicon", &curr_lang.name, "English"));
            ui.separator();
    
            // draw the table body
            Grid::new("lexicon table")
                .striped(true)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    for (native, conlang) in curr_lang.lexicon.iter() {
                        if curr_lang.lexicon_search_mode.matches(native, conlang, &curr_lang.lexicon_search) {
                            let conlang_lbl = ui.selectable_label(false, conlang)
                                .on_hover_text("Click to modify");
                            let native_lbl = ui.selectable_label(false, native)
                                .on_hover_text("Click to modify");
                            if conlang_lbl.clicked() || native_lbl.clicked() {
                                *lexicon_edit_win = Some(LexiconEditWindow::edit_entry(native, &curr_lang.lexicon));
                            }
                            ui.end_row();
                        }
                    }
            });
        });
    });

    if ui.button("Add Manual Lexicon Entry").clicked() {
        *lexicon_edit_win = Some(LexiconEditWindow::new_entry());
    }

    // draw lexicon edit popup
    if let Some(edit_win) = lexicon_edit_win {
        let request_close = edit_win.show(ui, &curr_lang.name, &mut curr_lang.lexicon);
        if request_close {
            *lexicon_edit_win = None;
        }
    }
 }

impl LexiconEditWindow {
    /// Create an instance of the edit window for modifying an existing entry.
    pub fn edit_entry(curr_native_phrase: &str, lexicon: &Lexicon) -> LexiconEditWindow {
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
        Window::new("Edit Lexicon")
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
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format!("{}:", conlang_name));
            });
            ui.text_edit_singleline(&mut self.conlang_phrase);
            ui.end_row();
    
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label("English:");
            });
            let native_input = ui.text_edit_singleline(&mut self.native_phrase);
            ui.end_row();
    
            if native_input.changed() {
                self.overwrite_warning = lexicon.get(&self.native_phrase)
                    .map(|curr_word| format!("Already mapped to <{}>", curr_word));
                if self.overwrite_warning.is_none() {
                    ui.memory_mut(|mem| mem.close_popup());
                }
            }
            if let Some(warning) = &self.overwrite_warning {
                let warning_id = ui.make_persistent_id("lexicon warning");
                ui.memory_mut(|mem| mem.open_popup(warning_id));
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
fn draw_delete_btn(ui: &mut Ui, lexicon: &mut Lexicon, orig_native_phrase: &str) -> bool {
    let clicked = ui.button("Delete Entry").clicked();
    if clicked {
        lexicon.remove(orig_native_phrase);
    }
    clicked
}

/// Draw a button that updates the active lexicon entry.
fn draw_apply_btn(ui: &mut Ui, lexicon: &mut Lexicon, orig_native_phrase: &str, native_phrase: &str, conlang_phrase: &str, can_edit: bool) -> bool {
    let button = Button::new("Apply Changes");
    let clicked = ui.add_enabled(can_edit, button).clicked();
    if clicked {
        lexicon.insert(native_phrase.to_string(), conlang_phrase.to_string());
        if orig_native_phrase != native_phrase {
            lexicon.remove(orig_native_phrase);
        }
    }
    clicked
}

/// Draw a button that adds the active entry to the lexicon.
fn draw_new_btn(ui: &mut Ui, lexicon: &mut Lexicon, native_phrase: &str, conlang_phrase: &str, can_edit: bool) -> bool {
    let button = Button::new("Add Entry");
    let clicked = ui.add_enabled(can_edit, button).clicked();
    if clicked {
        lexicon.insert(native_phrase.to_string(), conlang_phrase.to_string());
    }
    clicked
}