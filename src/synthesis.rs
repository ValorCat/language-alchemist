use std::collections::BTreeSet;
use eframe::egui::Response;
use eframe::egui::{Color32, DragValue, Grid, ScrollArea, Ui};
use itertools::{EitherOrBoth::*, Itertools};
use crate::Language;
use crate::grapheme::*;

/// The four root rules of the syllable synthesis grammar. Rules are stored in
/// sum-of-products form.
#[derive(Default)]
pub struct SyllableRules {
    initial: OrRule,
    middle: OrRule,
    terminal: OrRule,
    single: OrRule,
}

impl SyllableRules {
    fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut OrRule)> {
        [
            ("InitialSyllable", &mut self.initial),
            ("MiddleSyllable", &mut self.middle),
            ("TerminalSyllable", &mut self.terminal),
            ("SingleSyllable", &mut self.single),
        ].into_iter()
    }
}

/// A leaf node in the syllable synthesis grammar.
enum LeafRule {
    Uninitialized,
    Sequence(Vec<Grapheme>, String),
    Set(BTreeSet<Grapheme>, String),
    // Variable(String),
    Blank
}

/// An AND node in the syllable synthesis grammar.
#[derive(Default)]
struct AndRule {
    head: LeafRule,
    tail: Vec<LeafRule>
}

/// An OR node in the syllable synthesis grammar.
#[derive(Default)]
struct OrRule {
    head: AndRule,
    tail: Vec<AndRule>
}

impl LeafRule {
    /// Return an iterator over a "menu" of leaf node types in a (name, constructor) format.
    fn choices() -> impl Iterator<Item = (&'static str, fn() -> Self)> {
        let names = ["String", "Random", "Blank"];
        let funcs = [Self::sequence, Self::set, Self::blank];
        names.into_iter().zip(funcs)
    }

    /// Show a menu button that offers the choices in `LeafRule::choices()`, and then calls
    /// `action` with the chosen option.
    fn menu(ui: &mut Ui, text: &str, action: impl FnOnce(LeafRule)) -> Response {
        ui.menu_button(text, |ui: &mut Ui| {
            LeafRule::choices()
                .find(|(name, _)| ui.button(*name).clicked())
                .map(|(_, choice)| {
                    action(choice());
                    ui.close_menu();
                });
        }).response
    }

    /// Construct a default Sequence node.
    fn sequence() -> Self {
        Self::Sequence(Vec::new(), String::new())
    }

    /// Construct a default Set node.
    fn set() -> Self {
        Self::Set(BTreeSet::new(), String::new())
    }

    /// Construct a default Blank node.
    fn blank() -> Self {
        Self::Blank
    }
}

impl Default for LeafRule {
    fn default() -> Self {
        Self::Uninitialized
    }
}

impl AndRule {
    fn new(head: LeafRule) -> Self {
        AndRule { head, tail: Vec::new() }
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
    ui.horizontal(|ui| {
        ui.selectable_value(&mut curr_lang.syllable_edit_mode, false, "View Mode");
        ui.selectable_value(&mut curr_lang.syllable_edit_mode, true, "Edit Mode");
    });
    ui.add_space(5.0);
    ui.group(|ui| {
        ui.set_width(ui.available_width());      // fill available width
        ui.spacing_mut().interact_size.y = 20.0; // fix row height
        let mut order = 0; // incremented for each leaf node visited
        for (name, rule) in curr_lang.syllable_rules.iter_mut() {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{} =", name));
                draw_or_node(rule, ui, curr_lang.syllable_edit_mode, &curr_lang.graphemes, &mut order);
            });
            ui.add_space(3.0);
        }
    });
}

fn draw_or_node(rule: &mut OrRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage, order: &mut usize) {
    draw_and_node(&mut rule.head, ui, edit_mode, graphemes, order);
    for and_rule in &mut rule.tail {
        ui.heading("OR");
        draw_and_node(and_rule, ui, edit_mode, graphemes, order);
    }

    // draw button to insert new OR clause
    if edit_mode {
        ui.add_space(12.0);
        LeafRule::menu(ui, "OR...", |new_rule| rule.tail.push(AndRule::new(new_rule)));
    }
}

fn draw_and_node(rule: &mut AndRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage, order: &mut usize) {
    // draw button to insert node at beginning
    if edit_mode {
        LeafRule::menu(ui, "+", |new_rule| rule.tail.insert(0, std::mem::replace(&mut rule.head, new_rule)));
    }

    // draw first node
    draw_leaf_node(&mut rule.head, ui, edit_mode, graphemes, order);

    // draw remaining nodes
    // use indexed loop because we modify the list's length in the loop
    for i in 0..rule.tail.len() {
        if edit_mode {
            LeafRule::menu(ui, "+", |new_rule| rule.tail.insert(i, new_rule));
        } else {
            ui.label("+");
        }
        draw_leaf_node(&mut rule.tail[i], ui, edit_mode, graphemes, order);
    }

    // draw button to insert node at end
    if edit_mode {
        LeafRule::menu(ui, "+", |new_rule| rule.tail.push(new_rule));
    }
}

fn draw_leaf_node(rule: &mut LeafRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage, order: &mut usize) {
    *order += 1; // increment for each leaf node visited
    match rule {
        LeafRule::Uninitialized => {
            if edit_mode {
                LeafRule::menu(ui, "(click to set)", |new_rule| *rule = new_rule)
            } else {
                ui.colored_label(Color32::RED, "(not set)")
            }
        }
        LeafRule::Sequence(string, input) => {
            ui.add(GraphemeInputField::new(string, input, *order)
                .link(graphemes)
                .small(true)
                .allow_editing(edit_mode))
        }
        LeafRule::Set(set, input) => {
            ui.scope(|ui| {
                ui.label("{");
                ui.add(GraphemeInputField::new(set, input, *order)
                    .link(graphemes)
                    .small(true)
                    .allow_editing(edit_mode));
                ui.label("}");
            }).response
        }
        LeafRule::Blank => {
            ui.label("blank")
        }
    };
}

fn int_field_1_to_100(value: &mut u8) -> DragValue {
    DragValue::new(value).clamp_range(1..=100).speed(0.05)
}

fn int_field_percent(value: &mut u16) -> DragValue {
    DragValue::new(value).clamp_range(0..=100).suffix("%")
}