use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use eframe::egui::{Response, RichText, TextEdit, TextStyle};
use eframe::egui::{Color32, DragValue, Grid, ScrollArea, Ui};
use itertools::{EitherOrBoth::*, Itertools};
use crate::Language;
use crate::grapheme::*;

/// The four root rules of the syllable synthesis grammar. Rules are stored in
/// sum-of-products form.
#[derive(Default)]
pub struct SyllableRoots {
    initial: OrRule,
    middle: OrRule,
    terminal: OrRule,
    single: OrRule
}

impl SyllableRoots {
    /// Return an iterator over the root rule names.
    fn names() -> impl Iterator<Item = &'static str> {
        ["InitialSyllable", "MiddleSyllable", "TerminalSyllable", "SingleSyllable"].into_iter()
    }

    /// Return an iterator over immutable references to the root rules.
    fn iter(&self) -> impl Iterator<Item = &OrRule> {
        [&self.initial, &self.middle, &self.terminal, &self.single].into_iter()
    }

    /// Return an iterator over mutable references to the root rules.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut OrRule> {
        [&mut self.initial, &mut self.middle, &mut self.terminal, &mut self.single].into_iter()
    }
}

/// A mapping of syllable rule variable names to their values.
#[derive(Default)]
pub struct SyllableVars {
    vars: BTreeMap<String, OrRule>,
    reachable: HashSet<String>
}

/// An AND node in the syllable synthesis grammar.
type AndRule = NonEmptyList<LeafRule>;

/// An OR node in the syllable synthesis grammar.
type OrRule = NonEmptyList<AndRule>;

/// A Vec that is guaranteed to have at least one element.
#[derive(Default)]
struct NonEmptyList<T> {
    head: T,
    tail: Vec<T>
}

impl<T> NonEmptyList<T> {
    /// Create a new NonEmptyList with `head` as the first element.
    fn new(head: T) -> Self {
        Self { head, tail: vec![] }
    }

    /// Return an iterator over the elements of this list.
    fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(&self.tail)
    }
}

/// A leaf node in the syllable synthesis grammar.
enum LeafRule {
    Uninitialized,
    Sequence(Vec<Grapheme>, String),
    Set(BTreeSet<Grapheme>, String),
    Variable(String),
    Blank
}

impl LeafRule {
    /// Return an iterator over a "menu" of leaf node types in a (name, constructor) format.
    fn choices() -> impl Iterator<Item = (&'static str, fn() -> Self)> {
        let names = ["String", "Random", "Variable", "Blank"];
        let funcs = [Self::sequence, Self::set, Self::variable, Self::blank];
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

    /// Return true if this node is not Self::Uninitialized, otherwise return false.
    fn initialized(&self) -> bool {
        !matches!(self, Self::Uninitialized)
    }

    /// Construct a default Sequence node.
    fn sequence() -> Self {
        Self::Sequence(Vec::new(), String::new())
    }

    /// Construct a default Set node.
    fn set() -> Self {
        Self::Set(BTreeSet::new(), String::new())
    }

    /// Construct a default Variable node.
    fn variable() -> Self {
        Self::Variable(String::new())
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
        
        // remove vars that are both unreachable and empty
        flag_reachable_vars(&curr_lang.syllable_roots, &mut curr_lang.syllable_vars);
        let SyllableVars {vars, reachable} = &mut curr_lang.syllable_vars;
        vars.retain(|var, rule| reachable.contains(var) || rule.head.head.initialized());

        // data updated by certain visited nodes
        let mut order = 0; // incremented for each leaf node visited
        let mut new_var = None; // set if a new variable is referenced

        // 4 root rules
        let roots = &mut curr_lang.syllable_roots;
        for (name, rule) in SyllableRoots::names().zip(roots.iter_mut()) {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{} =", name));
                draw_or_node(rule, ui, curr_lang.syllable_edit_mode, &curr_lang.graphemes,
                    &mut order, &mut new_var);
            });
            ui.add_space(3.0);
        }

        // all other variable rules
        if !vars.is_empty() {
            ui.separator();
            for (var, rule) in vars.iter_mut() {
                ui.horizontal_wrapped(|ui| {
                    if reachable.contains(var) {
                        ui.monospace(format!("{} =", var));
                    } else {
                        let red_text = RichText::new(var).monospace().color(Color32::RED);
                        ui.label(red_text).on_hover_ui(|ui| {
                            ui.colored_label(Color32::RED, "Not reachable from a start variable");
                        });
                        ui.monospace("=");
                    }
                    draw_or_node(rule, ui, curr_lang.syllable_edit_mode, &curr_lang.graphemes,
                        &mut order, &mut new_var);
                });
                ui.add_space(3.0);
            }
        }

        // add new variable if an unrecognized name was used
        if let Some(new_var) = new_var {
            // we have to use all() instead of contains() because we're comparing &str to String
            if SyllableRoots::names().all(|s| *s != new_var) {
                vars.entry(new_var.clone()).or_insert_with(Default::default);
            }
        }
    });
}

fn draw_or_node(
        rule: &mut OrRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage,
        order: &mut usize, new_var: &mut Option<String>
) {
    draw_and_node(&mut rule.head, ui, edit_mode, graphemes, order, new_var);
    for and_rule in &mut rule.tail {
        ui.heading("OR");
        draw_and_node(and_rule, ui, edit_mode, graphemes, order, new_var);
    }

    // draw button to insert new OR clause
    if edit_mode && rule.head.head.initialized() {
        ui.add_space(12.0);
        LeafRule::menu(ui, "OR...", |new_rule| rule.tail.push(AndRule::new(new_rule)));
    }
}

fn draw_and_node(
    rule: &mut AndRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage,
    order: &mut usize, new_var: &mut Option<String>
) {
    // draw button to insert node at beginning
    if edit_mode && rule.head.initialized() {
        LeafRule::menu(ui, "+", |new_rule| rule.tail.insert(0, std::mem::replace(&mut rule.head, new_rule)));
    }

    // draw first node
    draw_leaf_node(&mut rule.head, ui, edit_mode, graphemes, order, new_var);

    // draw remaining nodes
    // use indexed loop because we modify the list's length in the loop
    for i in 0..rule.tail.len() {
        if edit_mode {
            LeafRule::menu(ui, "+", |new_rule| rule.tail.insert(i, new_rule));
        } else {
            ui.label("+");
        }
        draw_leaf_node(&mut rule.tail[i], ui, edit_mode, graphemes, order, new_var);
    }

    // draw button to insert node at end
    if edit_mode && rule.head.initialized() {
        LeafRule::menu(ui, "+", |new_rule| rule.tail.push(new_rule));
    }
}

fn draw_leaf_node(
    rule: &mut LeafRule, ui: &mut Ui, edit_mode: bool, graphemes: &MasterGraphemeStorage,
    order: &mut usize, new_var: &mut Option<String>
) {
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
        LeafRule::Variable(input) => {
            if edit_mode {
                let response = ui.add(TextEdit::singleline(input)
                    .text_style(TextStyle::Monospace)
                    .hint_text("Type...")
                    .desired_width(90.0));
                if response.changed() && !input.is_empty() {
                    *new_var = Some(input.clone());
                }
                response
            } else {
                ui.monospace(&input[..])
            }
        }
        LeafRule::Blank => {
            ui.label("blank")
        }
    };
}

/// Perform a DFS through the syllable rules, starting at each of the root variables.
/// Visited variables are stored in the set `vars.reachable`.
fn flag_reachable_vars(roots: &SyllableRoots, vars: &mut SyllableVars) {
    vars.reachable.clear();
    let mut stack: VecDeque<&OrRule> = roots.iter().collect();
    while let Some(next) = stack.pop_back() {
        next.iter()
            .flat_map(NonEmptyList::iter)
            .filter_map(|leaf| match leaf {
                LeafRule::Variable(var) => Some(var),
                _ => None
            })
            .filter(|&var| vars.reachable.insert(var.clone())) // skip already-visited variables
            .filter_map(|var| vars.vars.get(var)) // map name to rule and skip root variables
            .for_each(|rule| stack.push_back(rule))
    }
}

fn int_field_1_to_100(value: &mut u8) -> DragValue {
    DragValue::new(value).clamp_range(1..=100).speed(0.05)
}

fn int_field_percent(value: &mut u16) -> DragValue {
    DragValue::new(value).clamp_range(0..=100).suffix("%")
}