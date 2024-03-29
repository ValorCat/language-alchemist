use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{lexicon, synthesis};

#[derive(Default, Deserialize, Serialize)]
pub struct TranslateTab {
    pub input_text: String,
    pub output_text: String,
}

/// Render contents of the 'translate' tab.
pub fn draw_translate_tab(
    ui: &mut egui::Ui,
    curr_lang: &mut crate::Language,
    editing_name: &mut bool,
) {
    let crate::Language {
        name,
        translate_tab,
        lexicon_tab,
        synthesis_tab,
        ..
    } = curr_lang;

    // draw name and 'rename' button
    ui.horizontal(|ui| {
        if *editing_name {
            let text_field = egui::TextEdit::singleline(name).font(egui::TextStyle::Heading);
            let response = ui.add(text_field);
            response.request_focus();
            if response.lost_focus()
                || response.clicked_elsewhere()
                || ui.ctx().input(|i| i.key_pressed(egui::Key::Enter))
            {
                *editing_name = false;
            }
        } else {
            ui.heading(name);
            if ui.small_button("Rename").clicked() {
                *editing_name = true;
            }
        }
    });

    // draw input box
    ui.add_space(10.0);
    ui.add(
        egui::TextEdit::multiline(&mut translate_tab.input_text)
            .hint_text("Enter text to translate...")
            .desired_width(ui.available_width() * 0.8)
    );

    // draw translate button
    ui.add_space(10.0);
    let button = ui
        .add_enabled(
            synthesis::is_config_valid(synthesis_tab),
            egui::Button::new("Translate"),
        )
        .on_disabled_hover_text("This language's configuration contains errors.");

    // parse input, ignoring punctuation, and translate the rest
    if button.clicked() {
        translate_tab.output_text.clear();
        let mut word_start = None;
        for (i, chr) in translate_tab.input_text.char_indices() {
            if chr.is_alphanumeric() {
                // mark this as the start of the word if no start already exists
                word_start.get_or_insert(i);
            } else {
                if let Some(start) = word_start.take() {
                    translate_tab.output_text.push_str(translate_word(
                        &translate_tab.input_text[start..i],
                        &mut lexicon_tab.lexicon,
                        &synthesis_tab.syllable_vars,
                        &synthesis_tab.syllable_wgts,
                    ));
                }
                translate_tab.output_text.push(chr);
            }
        }
        if let Some(start) = word_start {
            // translate and add trailing word if input doesn't end with a full stop
            translate_tab.output_text.push_str(translate_word(
                &translate_tab.input_text[start..],
                &mut lexicon_tab.lexicon,
                &synthesis_tab.syllable_vars,
                &synthesis_tab.syllable_wgts,
            ));
        }
    }

    // draw output box
    ui.add_space(10.0);
    ui.group(|ui| {
        ui.set_width(ui.available_width() * 0.8);
        ui.label(&translate_tab.output_text);
    });
}

/// Given an input word, translates it and updates the lexicon if the word
/// hasn't been translated before.
fn translate_word<'a>(
    word: &str,
    lexicon: &'a mut lexicon::Lexicon,
    vars: &synthesis::SyllableVars,
    weights: &(Vec<u16>, Vec<u16>),
) -> &'a str {
    let generate_new = || synthesis::synthesize_morpheme(vars, &weights.1); // todo distinguish content and function weights
    lexicon
        .entry(word.to_lowercase())
        .or_insert_with(generate_new)
}
