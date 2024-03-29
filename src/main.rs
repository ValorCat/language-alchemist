use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

mod grammar;
mod grapheme;
mod lexicon;
mod synthesis;
mod translate;
mod util;

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Language Alchemist",
        Default::default(),
        Box::new(|cc| Box::new(Application::new(cc))),
    )
}

/// A constructed language.
#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Language {
    name: String,
    translate_tab: translate::TranslateTab,
    lexicon_tab: lexicon::LexiconTab,
    synthesis_tab: synthesis::SynthesisTab,
    grammar_tab: grammar::GrammarTab,
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
    #[serde(skip)]
    curr_tab: Tab,
    #[serde(skip)]
    editing_name: bool,
    #[serde(skip)]
    lexicon_edit_win: Option<lexicon::LexiconEditWindow>,
}

impl Application {
    fn new(cc: &eframe::CreationContext) -> Self {
        if let Some(storage) = cc.storage {
            let mut loaded_app: Self =
                eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            for language in &mut loaded_app.languages {
                grammar::load_grammar_serde_metadata(&mut language.grammar_tab.grammar_rules);
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
    #[default]
    Translate,
    Lexicon,
    Synthesis,
    Grammar,
}

// implement to_string() so we don't have to repeat the tab names
impl Display for Tab {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl eframe::App for Application {
    /// Called on exit to save any state not marked with `#[serde(skip)]`.
    /// Also automatically called every 30 seconds (as defined by `epi:App::auto_save_interval`).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        for language in &mut self.languages {
            grammar::save_grammar_serde_metadata(&mut language.grammar_tab.grammar_rules);
        }
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each frame to render the UI.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self {
            languages,
            curr_lang_idx,
            curr_tab,
            editing_name,
            lexicon_edit_win,
        } = self;

        // draw left panel
        egui::SidePanel::left("language list")
            .default_width(120.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0); // align with tab list to our right
                    ui.heading("Languages");
                });
                ui.separator();

                // draw language list
                egui::ScrollArea::vertical().show(ui, |ui| {
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

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::global_dark_light_mode_buttons(ui);
                    });
                });

                ui.separator();
                ui.add_space(5.0);

                // draw contents of active tab
                match curr_tab {
                    Tab::Translate => translate::draw_translate_tab(ui, curr_lang, editing_name),
                    Tab::Lexicon => lexicon::draw_lexicon_tab(
                        ui,
                        &mut curr_lang.lexicon_tab,
                        &curr_lang.name,
                        lexicon_edit_win,
                    ),
                    Tab::Synthesis => {
                        synthesis::draw_synthesis_tab(ui, &mut curr_lang.synthesis_tab)
                    }
                    Tab::Grammar => grammar::draw_grammar_tab(ui, &mut curr_lang.grammar_tab),
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
