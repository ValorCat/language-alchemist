use std::fmt::{self, Debug, Display};
use eframe::egui;
use egui::{Button, Context, Key, TextEdit, Ui};
use egui::containers::ScrollArea;
use serde::{Deserialize, Serialize};
use crate::grammar::{GrammarRule, draw_grammar_tab, load_grammar_serde_metadata, save_grammar_serde_metadata};
use crate::grapheme::MasterGraphemeStorage;
use crate::lexicon::{LexiconSearchMode, Lexicon, LexiconEditWindow, draw_lexicon_tab};
use crate::synthesis::{SyllableVars, draw_synthesis_tab, is_config_valid, synthesize_morpheme};
use crate::util::EditMode;

mod grammar;
mod grapheme;
mod lexicon;
mod synthesis;
mod util;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Language Alchemist",
        Default::default(),
        Box::new(|cc| Box::new(Application::new(cc)))
    )
}

/// A constructed language.
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Language {
    // translate tab
    name: String,
    input_text: String,
    output_text: String,

    // lexicon tab
    allow_homonyms: bool,
    num_homonyms: u32,
    #[serde(skip)] lexicon_search: String,
    #[serde(skip)] lexicon_search_mode: LexiconSearchMode,
    lexicon: Lexicon,

    // synthesis tab
    #[serde(skip)] test_words: Vec<String>,
    graphemes: MasterGraphemeStorage,
    #[serde(skip)] new_grapheme: String,
    max_syllables: (u8, u8),             // (function words, content words)
    syllable_wgts: (Vec<u16>, Vec<u16>), // (function words, content words)
    syllable_vars: SyllableVars,
    #[serde(skip)] syllable_edit_mode: EditMode,

    // grammar tab
    grammar_rules: Vec<GrammarRule>,
    #[serde(skip)] grammar_edit_mode: EditMode
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
#[derive(Default, Deserialize, Serialize)]
struct Application {
    curr_lang_idx: Option<usize>,
    languages: Vec<Language>,
    #[serde(skip)] curr_tab: Tab,
    #[serde(skip)] editing_name: bool,
    #[serde(skip)] lexicon_edit_win: Option<LexiconEditWindow>
}

impl Application {
    fn new(cc: &eframe::CreationContext) -> Self {
        if let Some(storage) = cc.storage {
            let mut loaded_app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            for language in &mut loaded_app.languages {
                load_grammar_serde_metadata(&mut language.grammar_rules);
            }
            loaded_app
        } else {
            Default::default()
        }
    }
}

/// One of the four UI tabs at the top of the window.
#[derive(Clone, Debug, Default, PartialEq)]
enum Tab {
    #[default] Translate,
    Lexicon,
    Synthesis,
    Grammar
}

// implement to_string() so we don't have to repeat the tab names
impl Display for Tab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl eframe::App for Application {
    /// Called on exit to save any state not marked with `#[serde(skip)]`.
    /// Also automatically called every 30 seconds (as defined by `epi:App::auto_save_interval`).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        for language in &mut self.languages {
            save_grammar_serde_metadata(&mut language.grammar_rules);
        }
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each frame to render the UI.
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
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
                    Tab::Grammar => draw_grammar_tab(ui, curr_lang)
                }
            } else {
                ui.add_space(10.0);
                ui.label("Select a language on the left, or create a new one.");
                egui::global_dark_light_mode_buttons(ui);
                egui::warn_if_debug_build(ui);
            }
        });
    }
}

/// Render contents of the 'translate' tab.
fn draw_translate_tab(ui: &mut Ui, ctx: &Context, curr_lang: &mut Language, editing_name: &mut bool) {
    // draw name and 'rename' button
    ui.horizontal(|ui| {
        if *editing_name {
            let text_field = TextEdit::singleline(&mut curr_lang.name)
                .font(egui::TextStyle::Heading);
            let response = ui.add(text_field);
            response.request_focus();
            if response.lost_focus() || response.clicked_elsewhere() || ctx.input(|i| i.key_pressed(Key::Enter)) {
                *editing_name = false;
            }
        } else {
            ui.heading(&curr_lang.name);
            if ui.small_button("Rename").clicked() {
                *editing_name = true;
            }
        }
    });

    // draw input box
    ui.add_space(10.0);
    ui.add(TextEdit::multiline(&mut curr_lang.input_text)
        .hint_text("Enter text to translate...")
        .desired_width(ui.available_width() * 0.8));
    
    // draw translate button
    ui.add_space(10.0);
    let button = ui.add_enabled(is_config_valid(curr_lang), Button::new("Translate"))
        .on_disabled_hover_text("This language's configuration contains errors.");
    
    // parse input, ignoring punctuation, and translate the rest
    if button.clicked() {
        curr_lang.output_text.clear();
        let mut word_start = None;
        for (i, chr) in curr_lang.input_text.char_indices() {
            if chr.is_alphanumeric() {
                // mark this as the start of the word if no start already exists
                word_start.get_or_insert(i);
            } else {
                if let Some(start) = word_start.take() {
                    curr_lang.output_text.push_str(translate_word(&curr_lang.input_text[start..i],
                        &mut curr_lang.lexicon, &curr_lang.syllable_vars, &curr_lang.syllable_wgts));
                }
                curr_lang.output_text.push(chr);
            }
        }
        if let Some(start) = word_start {
            // translate and add trailing word if input doesn't end with a full stop
            curr_lang.output_text.push_str(translate_word(&curr_lang.input_text[start..],
                &mut curr_lang.lexicon, &curr_lang.syllable_vars, &curr_lang.syllable_wgts));
        }
    }

    // draw output box
    ui.add_space(10.0);
    ui.group(|ui| {
        ui.set_width(ui.available_width() * 0.8);
        ui.label(&curr_lang.output_text);
    });
}

/// Given an input word, translates it and updates the lexicon if the word
/// hasn't been translated before.
fn translate_word<'a>(word: &str, lexicon: &'a mut Lexicon, vars: &SyllableVars,
    weights: &(Vec<u16>, Vec<u16>))
-> &'a str {
    let generate_new = || synthesize_morpheme(vars, &weights.1); // todo distinguish content and function weights
    lexicon.entry(word.to_lowercase()).or_insert_with(generate_new)
}