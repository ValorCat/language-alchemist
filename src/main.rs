use std::fmt::{self, Debug, Display};
use eframe::epi;
use egui::{self, CtxRef, Key, TextEdit, Ui};
use egui::containers::ScrollArea;
use crate::lexicon::*;
use crate::synthesis::*;

mod lexicon;
mod synthesis;

fn main() {
    let app = Application::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}

/// A constructed language.
#[derive(Default)]
pub struct Language {
    // translate tab
    name: String,
    input_text: String,
    output_text: String,

    // lexicon tab
    allow_homonyms: bool,
    num_homonyms: u32,
    lexicon_search: String,
    lexicon_search_mode: LexiconSearchMode,
    lexicon: Lexicon,

    // synthesis tab
    graphemes: Vec<Grapheme>,
    new_grapheme: String,
    max_syllables: (u8, u8),          // (function words, content words)
    syllable_wgts: (Vec<u8>, Vec<u8>) // (function words, content words)
}

impl Language {
    /// Create a new, blank language with the default attributes.
    fn new() -> Self {
        Self {
            name: "New Language".to_owned(),
            ..Default::default()
        }
    }
}

/// An instance of the application. Maintains the list of the languages as well as UI data.
#[derive(Default)]
struct Application {
    languages: Vec<Language>,
    curr_lang_idx: Option<usize>,
    curr_tab: Tab,
    editing_name: bool,
    lexicon_edit_win: Option<LexiconEditWindow>
}

/// One of the four UI tabs at the top of the window.
#[derive(Clone, Debug, PartialEq)]
enum Tab { Translate, Lexicon, Synthesis, Grammar }

impl Default for Tab {
    fn default() -> Self {
        Tab::Translate
    }
}

// implement to_string() so we don't have to repeat the tab names
impl Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl epi::App for Application {
    /// Get the name of the application and title of the main window.
    fn name(&self) -> &str {
        "Language Alchemist"
    }

    /// Called each frame to render the UI.
    ///
    /// # Arguments
    ///
    /// * `self` - The application instance, which stores the UI data.
    /// * `ctx` - The application context, which manages I/O.
    /// * `_frame` - The window and its surrounding context.
    fn update(&mut self, ctx: &CtxRef, _frame: &mut epi::Frame<'_>) {
        let Self {languages, curr_lang_idx, curr_tab, editing_name, lexicon_edit_win} = self;

        // draw left panel
        egui::SidePanel::left("language list").default_width(120.0).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0); // align with tab list to our right
                ui.heading("Languages");
            });
            ui.separator();

            // draw language list
            ScrollArea::vertical().show(ui, |ui| {
                if let Some(curr_lang_idx) = curr_lang_idx {
                    for (idx, lang) in languages.iter().enumerate() {
                        ui.selectable_value(curr_lang_idx, idx, &lang.name);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.label("(none)");
                    });
                }
            });

            ui.add_space(10.0);
            ui.separator();

            // draw 'new language' button
            ui.vertical_centered(|ui| {
                if ui.button("New Language").clicked() {
                    languages.push(Language::new());
                    *curr_lang_idx = Some(languages.len() - 1);
                    *curr_tab = Tab::Translate;
                }
            });

        });

        // draw main panel
        egui::CentralPanel::default().show(ctx, |ui| {
            let curr_lang = curr_lang_idx.map(|idx| &mut languages[idx]);
            if let Some(curr_lang) = curr_lang {

                // draw top tabs
                ui.horizontal(|ui| {
                    for tab in [Tab::Translate, Tab::Lexicon, Tab::Synthesis, Tab::Grammar] {
                        ui.selectable_value(curr_tab, tab.clone(), tab.to_string());
                        ui.separator();
                    }
                });

                ui.separator();
                ui.add_space(5.0);

                // draw contents of active tab
                match curr_tab {
                    Tab::Translate => draw_translate_tab(ui, ctx, curr_lang, editing_name),
                    Tab::Lexicon => draw_lexicon_tab(ui, curr_lang, lexicon_edit_win),
                    Tab::Synthesis => draw_synthesis_tab(ui, curr_lang),
                    Tab::Grammar => {},
                }
            } else {
                ui.add_space(10.0);
                ui.label("Select a language on the left, or create a new one.");
                egui::warn_if_debug_build(ui);
            }
        });
    }
}

/// Render contents of the 'translate' tab.
fn draw_translate_tab(ui: &mut Ui, ctx: &CtxRef, curr_lang: &mut Language, editing_name: &mut bool) {
    // draw name and 'rename' button
    ui.horizontal(|ui| {
        if *editing_name {
            let text_field = TextEdit::singleline(&mut curr_lang.name)
                .text_style(egui::TextStyle::Heading);
            let response = ui.add(text_field);
            response.request_focus();
            if response.lost_focus() || response.clicked_elsewhere() || ctx.input().key_pressed(Key::Enter) {
                *editing_name = false;
            }
        } else {
            ui.heading(&curr_lang.name);
            if ui.small_button("Rename").clicked() {
                *editing_name = true;
            }
        }
    });

    // draw input and output boxes
    let input_text = &mut curr_lang.input_text;
    let output_text = &mut curr_lang.output_text;

    ui.add_space(10.0);
    ui.add(TextEdit::multiline(input_text).hint_text("Enter text to translate..."));
    if ui.button("Translate").clicked() {
        // todo run translation engine
        *output_text = input_text.clone();
    }

    ui.add_space(10.0);
    ui.add_enabled(false, TextEdit::multiline(output_text));
}