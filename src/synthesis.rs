use eframe::egui::{Color32, DragValue, Grid, Ui};
use itertools::{EitherOrBoth::*, Itertools};
use crate::Language;
use crate::grapheme::*;

/// Render contents of the 'synthesis' tab.
pub fn draw_synthesis_tab(ui: &mut Ui, curr_lang: &mut Language) {
    draw_graphemic_inventory(ui, curr_lang);
    ui.add_space(10.0);
    draw_syllable_counter(ui, curr_lang);
}

fn draw_graphemic_inventory(ui: &mut Ui, curr_lang: &mut Language) {
    ui.heading("Graphemic Inventory");
    ui.label("The graphemic inventory is the set of recognized graphemes (unique letters or glyphs) in the \
        language. It can also contain multigraphs, like the English <ch> and <sh>.");
    ui.add_space(5.0);
    ui.add(GraphemeInputField::new(&mut curr_lang.graphemes, &mut curr_lang.new_grapheme, "new grapheme"));

    // show error if empty
    if curr_lang.graphemes.is_empty() {
        ui.add_space(5.0);
        ui.colored_label(Color32::RED, "The graphemic inventory must contain at least one grapheme");
    }
}

fn draw_syllable_counter(ui: &mut Ui, curr_lang: &mut Language) {
    ui.heading("Word Length");
    ui.label("Word length is measured in syllables. The settings below determine the probability \
        of generating a word with the given number of syllables. On average, function words \
        (conjunctions, determiners, etc.) often have fewer syllables than content words.");
    ui.add_space(5.0);
    ui.group(|ui| {
        Grid::new("syllable count").show(ui, |ui| {
            // header row
            ui.label("Word Type:");
            ui.label("Function");
            ui.label("Content");
            ui.end_row();

            // max syllable row
            ui.label("Max Syllables:");
            ui.add(int_field_1_to_100(&mut curr_lang.max_syllables.0));
            ui.add(int_field_1_to_100(&mut curr_lang.max_syllables.1));

            // resize weight lists based on above fields
            curr_lang.syllable_wgts.0.resize(curr_lang.max_syllables.0 as usize, 0);
            curr_lang.syllable_wgts.1.resize(curr_lang.max_syllables.1 as usize, 0);
            ui.end_row();

            // hardcoded first weight (so it doesn't say "1 Syllables")
            ui.label("1 Syllable:");
            ui.add(int_field_percent(&mut curr_lang.syllable_wgts.0[0]));
            ui.add(int_field_percent(&mut curr_lang.syllable_wgts.1[0]));
            ui.end_row();

            // all other weights
            for (row_num, wgts) in curr_lang.syllable_wgts.0.iter_mut().skip(1)
                .zip_longest(curr_lang.syllable_wgts.1.iter_mut().skip(1))
                .enumerate()
            {
                // itertools::zip_longest() stops once both columns are exhausted
                ui.label(format!("{} Syllables:", row_num + 2));
                match wgts {
                    Both(wgt1, wgt2) => {
                        ui.add(int_field_percent(wgt1));
                        ui.add(int_field_percent(wgt2));
                    }
                    Left(wgt) => {
                        ui.add(int_field_percent(wgt));
                    }
                    Right(wgt) => {
                        ui.scope(|_| {}); // empty cell
                        ui.add(int_field_percent(wgt));
                    }
                }
                ui.end_row();
            }
        });
    });
    
    // check each column sums to 100
    let func_total: u16 = curr_lang.syllable_wgts.0.iter().sum();
    let content_total: u16 = curr_lang.syllable_wgts.1.iter().sum();
    if func_total != 100 || content_total != 100 {
        ui.add_space(5.0);
        ui.colored_label(Color32::RED, "Each column should add up to 100%:");
        if func_total != 100 {
            ui.colored_label(Color32::RED, format!("  * The column \"Function Words\" adds up to {}%", func_total));
        }
        if content_total != 100 {
            ui.colored_label(Color32::RED, format!("  * The column \"Content Words\" adds up to {}%", content_total));
        }
    }
}

fn int_field_1_to_100(value: &mut u8) -> DragValue {
    DragValue::new(value).clamp_range(1..=100).speed(0.05)
}

fn int_field_percent(value: &mut u16) -> DragValue {
    DragValue::new(value).clamp_range(0..=100).suffix("%")
}