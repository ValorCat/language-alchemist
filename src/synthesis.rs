use eframe::egui::ScrollArea;
use eframe::egui::{Color32, DragValue, Grid, Ui};
use itertools::{EitherOrBoth::*, Itertools};
use crate::Language;
use crate::grapheme::*;

/// The four root rules of the syllable synthesis grammar.
#[derive(Default)]
pub struct RootSyllableRules {
    initial: SyllableRule,
    middle: SyllableRule,
    terminal: SyllableRule,
    single: SyllableRule
}

/// A node in the syllable synthesis rules tree.
pub enum SyllableRule {
    NotSet,
    Literal(Vec<Grapheme>, String),
    // RandomLiteral(Vec<Grapheme>, String),
    // Variable(String),
    // Optional(Box<SyllableRule>),
    // Union(Box<SyllableRule>, Box<SyllableRule>),
    // Intersection(Box<SyllableRule>, Box<SyllableRule>),  // must contain two RandomLiterals
    // Difference(Box<SyllableRule>, Box<SyllableRule>)
}

impl Default for SyllableRule {
    fn default() -> Self {
        Self::NotSet
    }
}

/// Render contents of the 'synthesis' tab.
pub fn draw_synthesis_tab(ui: &mut Ui, curr_lang: &mut Language) {
    ScrollArea::vertical().show(ui, |ui| {
        draw_graphemic_inventory(ui, curr_lang);
        ui.add_space(10.0);
        draw_syllable_rules(ui, curr_lang);
        ui.add_space(10.0);
        draw_syllable_counter(ui, curr_lang);
    });
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

fn draw_syllable_rules(ui: &mut Ui, curr_lang: &mut Language) {
    ui.heading("Syllable Synthesis");
    ui.label("Each word is formed from a sequence of syllables, which are themselves formed from sequences of \
        graphemes. There are four types of syllables: initial, middle, terminal, and single (for words with \
        only one syllable). Each syllable type is generated based on the rules you define in this section.");
    ui.add_space(5.0);
    ui.group(|ui| {
        ui.spacing_mut().interact_size.y += 6.0; // add extra row height
        let root = &mut curr_lang.syllable_rules;
        let mut order = 0;
        for (name, rule) in [
            ("InitialSyllable", &mut root.initial),
            ("MiddleSyllable", &mut root.middle),
            ("TerminalSyllable", &mut root.terminal),
            ("SingleSyllable", &mut root.single),
        ] {
            ui.horizontal(|ui| {
                ui.monospace(format!("{} =", name));
                ui.spacing_mut().interact_size.y -= 6.0; // revert height change for right side of '='
                draw_rule(ui, rule, &mut order);
            });
        }
    });
}

/// Recursively renders and updates a tree of syllable synthesis rules.
fn draw_rule(ui: &mut Ui, rule: &mut SyllableRule, order: &mut usize) {
    use self::SyllableRule::*;
    *order += 1; // increment for each node visited
    match rule {
        NotSet => {
            ui.menu_button("(click to set)", |ui| {
                if ui.button("Grapheme").clicked() {
                    *rule = Literal(Vec::new(), String::new());
                    ui.close_menu();
                }
                let _ = ui.button("Random grapheme");
                let _ = ui.button("Variable");
            });
        }
        Literal(graphemes, input) => {
            ui.add(GraphemeInputField::new(graphemes, input, *order).small());
        }
    }
}

fn int_field_1_to_100(value: &mut u8) -> DragValue {
    DragValue::new(value).clamp_range(1..=100).speed(0.05)
}

fn int_field_percent(value: &mut u16) -> DragValue {
    DragValue::new(value).clamp_range(0..=100).suffix("%")
}