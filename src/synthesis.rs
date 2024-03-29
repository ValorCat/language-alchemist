use crate::grapheme;
use crate::util::{self, EditMode, NonEmptyList};
use eframe::egui;
use itertools::{EitherOrBoth, Itertools};
use rand::{distributions::WeightedIndex, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

#[derive(Default, Deserialize, Serialize)]
pub struct SynthesisTab {
    pub graphemes: grapheme::MasterGraphemeStorage,
    pub syllable_vars: SyllableVars,
    pub max_syllables: (u8, u8), // (function words, content words)
    pub syllable_wgts: (Vec<u16>, Vec<u16>), // (function words, content words)
    #[serde(skip)]
    test_words: Vec<String>,
    #[serde(skip)]
    new_grapheme: String,
    #[serde(skip)]
    syllable_edit_mode: EditMode,
}

/// A mapping of syllable rule variable names to their values.
#[derive(Default, Deserialize, Serialize)]
pub struct SyllableVars {
    roots: SyllableRoots,
    vars: BTreeMap<String, OrRule>,
    reachable: HashSet<String>,
}

impl SyllableVars {
    /// Return the rule associated with a variable name if it exists, or otherwise None.
    fn get(&self, var: &str) -> Option<&OrRule> {
        match var {
            "InitialSyllable" => Some(&self.roots.initial),
            "MiddleSyllable" => Some(&self.roots.middle),
            "TerminalSyllable" => Some(&self.roots.terminal),
            "SingleSyllable" => Some(&self.roots.single),
            _ => self.vars.get(var),
        }
    }
}

/// The four root rules of the syllable synthesis grammar. Rules are stored in
/// sum-of-products form.
#[derive(Default, Deserialize, Serialize)]
struct SyllableRoots {
    initial: OrRule,
    middle: OrRule,
    terminal: OrRule,
    single: OrRule,
}

impl SyllableRoots {
    /// Return an iterator over the root rule names.
    fn names() -> impl Iterator<Item = &'static str> {
        [
            "InitialSyllable",
            "MiddleSyllable",
            "TerminalSyllable",
            "SingleSyllable",
        ]
        .into_iter()
    }

    /// Return an iterator over immutable references to the root rules.
    fn iter(&self) -> impl Iterator<Item = &OrRule> {
        [&self.initial, &self.middle, &self.terminal, &self.single].into_iter()
    }

    /// Return an iterator over mutable references to the root rules.
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut OrRule> {
        [
            &mut self.initial,
            &mut self.middle,
            &mut self.terminal,
            &mut self.single,
        ]
        .into_iter()
    }
}

/// An AND node in the syllable synthesis grammar.
type AndRule = NonEmptyList<LeafRule>;

/// An OR node in the syllable synthesis grammar.
type OrRule = NonEmptyList<AndRule>;

/// A leaf node in the syllable synthesis grammar.
#[derive(Deserialize, Serialize)]
enum LeafRule {
    Uninitialized,
    Sequence(Vec<grapheme::Grapheme>, String),
    Set(BTreeSet<grapheme::Grapheme>, String),
    Variable(String),
    Blank,
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
    fn menu(ui: &mut egui::Ui, text: &str, action: impl FnOnce(LeafRule)) -> egui::Response {
        ui.menu_button(text, |ui: &mut egui::Ui| {
            let clicked = LeafRule::choices().find(|(name, _)| ui.button(*name).clicked());
            if let Some((_, choice)) = clicked {
                action(choice());
                ui.close_menu();
            }
        })
        .response
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
pub fn draw_synthesis_tab(ui: &mut egui::Ui, data: &mut SynthesisTab) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        draw_test_generator(ui, data);
        ui.add_space(10.0);
        draw_graphemic_inventory(ui, data);
        ui.add_space(10.0);
        draw_syllable_rules(ui, data);
        ui.add_space(10.0);
        draw_syllable_counter(ui, data);
    });
}

fn draw_test_generator(ui: &mut egui::Ui, data: &mut SynthesisTab) {
    ui.heading("Sample Generation");
    ui.label("Use the buttons below to generate sample words using the current configuration.");
    ui.add_space(5.0);
    ui.horizontal(|ui| {
        let err_text = "The word length probabilities do not add up to 100%";
        let function_wgts = &data.syllable_wgts.0;
        let content_wgts = &data.syllable_wgts.1;
        let function_btn = ui
            .add_enabled(
                verify_weights(function_wgts),
                egui::Button::new("Function Words"),
            )
            .on_disabled_hover_text(err_text);
        let content_btn = ui
            .add_enabled(
                verify_weights(content_wgts),
                egui::Button::new("Content Words"),
            )
            .on_disabled_hover_text(err_text);
        if function_btn.clicked() || content_btn.clicked() {
            let weights = if function_btn.clicked() {
                function_wgts
            } else {
                content_wgts
            };
            let producer = || synthesize_morpheme(&data.syllable_vars, weights);
            data.test_words = std::iter::repeat_with(producer)
                .take(24) // 3 columns of 8
                .map(|word| {
                    if !word.is_empty() {
                        word
                    } else {
                        "(blank)".to_owned()
                    }
                })
                .collect();
            ui.close_menu();
        }
    });
    if !data.test_words.is_empty() {
        ui.add_space(5.0);
        ui.group(|ui| {
            ui.columns(3, |columns| {
                for (i, word) in data.test_words.iter().enumerate() {
                    columns[i % 3].label(word);
                }
            })
        });
    }
}

fn draw_graphemic_inventory(ui: &mut egui::Ui, data: &mut SynthesisTab) {
    ui.heading("Graphemic Inventory");
    ui.label("The graphemic inventory is the set of recognized graphemes (unique letters or glyphs) in the \
        language. It can also contain multigraphs, like the English <ch> and <sh>.");
    ui.add_space(5.0);
    ui.add(grapheme::GraphemeInputField::new(
        &mut data.graphemes,
        &mut data.new_grapheme,
        "new grapheme",
    ));

    // show error if empty
    if data.graphemes.is_empty() {
        ui.add_space(5.0);
        ui.colored_label(
            egui::Color32::RED,
            "The graphemic inventory must contain at least one grapheme",
        );
    }
}

fn draw_syllable_counter(ui: &mut egui::Ui, data: &mut SynthesisTab) {
    ui.heading("Word Length");
    ui.label(
        "Word length is measured in syllables. The settings below determine the probability \
        of generating a word with the given number of syllables. On average, function words \
        (conjunctions, determiners, etc.) often have fewer syllables than content words.",
    );
    ui.add_space(5.0);
    ui.group(|ui| {
        egui::Grid::new("syllable count").show(ui, |ui| {
            // header row
            ui.label("Word Type:");
            ui.label("Function");
            ui.label("Content");
            ui.end_row();

            // max syllable row
            ui.label("Max Syllables:");
            ui.add(int_field_1_to_100(&mut data.max_syllables.0));
            ui.add(int_field_1_to_100(&mut data.max_syllables.1));

            // resize weight lists based on above fields
            data.syllable_wgts
                .0
                .resize(data.max_syllables.0 as usize, 0);
            data.syllable_wgts
                .1
                .resize(data.max_syllables.1 as usize, 0);
            ui.end_row();

            // hardcoded first weight (so it doesn't say "1 Syllables")
            ui.label("1 Syllable:");
            ui.add(int_field_percent(&mut data.syllable_wgts.0[0]));
            ui.add(int_field_percent(&mut data.syllable_wgts.1[0]));
            ui.end_row();

            // all other weights
            for (row_num, wgts) in data
                .syllable_wgts
                .0
                .iter_mut()
                .skip(1)
                .zip_longest(data.syllable_wgts.1.iter_mut().skip(1))
                .enumerate()
            {
                // itertools::zip_longest() stops once both columns are exhausted
                ui.label(format!("{} Syllables:", row_num + 2));
                match wgts {
                    EitherOrBoth::Both(wgt1, wgt2) => {
                        ui.add(int_field_percent(wgt1));
                        ui.add(int_field_percent(wgt2));
                    }
                    EitherOrBoth::Left(wgt) => {
                        ui.add(int_field_percent(wgt));
                    }
                    EitherOrBoth::Right(wgt) => {
                        ui.scope(|_| {}); // empty cell
                        ui.add(int_field_percent(wgt));
                    }
                }
                ui.end_row();
            }
        });
    });

    // check each column sums to 100
    let func_total: u16 = data.syllable_wgts.0.iter().sum();
    let content_total: u16 = data.syllable_wgts.1.iter().sum();
    if func_total != 100 || content_total != 100 {
        ui.add_space(5.0);
        ui.colored_label(egui::Color32::RED, "Each column should add up to 100%:");
        if func_total != 100 {
            ui.colored_label(
                egui::Color32::RED,
                format!(
                    "  * The column \"Function Words\" adds up to {}%",
                    func_total
                ),
            );
        }
        if content_total != 100 {
            ui.colored_label(
                egui::Color32::RED,
                format!(
                    "  * The column \"Content Words\" adds up to {}%",
                    content_total
                ),
            );
        }
    }
}

fn draw_syllable_rules(ui: &mut egui::Ui, data: &mut SynthesisTab) {
    ui.heading("Syllable Synthesis");
    ui.label("Each word is formed from a sequence of syllables, which are themselves formed from sequences of \
        graphemes. There are four types of syllables: initial, middle, terminal, and single (for words with \
        only one syllable). Each syllable type is generated based on the rules you define in this section.");
    ui.add_space(5.0);
    EditMode::draw_mode_picker(ui, &mut data.syllable_edit_mode);
    ui.add_space(5.0);
    ui.group(|ui| {
        ui.set_width(ui.available_width()); // fill available width
        ui.spacing_mut().interact_size.y = 20.0; // fix row height

        // remove vars that are both unreachable and empty
        flag_reachable_vars(&mut data.syllable_vars);
        let SyllableVars {
            roots,
            vars,
            reachable,
        } = &mut data.syllable_vars;
        vars.retain(|var, rule| reachable.contains(var) || rule.head.head.initialized());

        // data updated by certain visited nodes
        let mut order = 0; // incremented for each leaf node visited
        let mut new_var = None; // set if a new variable is referenced

        // 4 root rules
        for (name, rule) in SyllableRoots::names().zip(roots.iter_mut()) {
            ui.horizontal_wrapped(|ui| {
                ui.monospace(format!("{} =", name));
                draw_or_node(
                    ui,
                    rule,
                    data.syllable_edit_mode,
                    &data.graphemes,
                    &mut order,
                    &mut new_var,
                );
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
                        let red_text = egui::RichText::new(var)
                            .monospace()
                            .color(egui::Color32::RED);
                        ui.label(red_text).on_hover_ui(|ui| {
                            ui.colored_label(
                                egui::Color32::RED,
                                "Not reachable from a start variable",
                            );
                        });
                        ui.monospace("=");
                    }
                    draw_or_node(
                        ui,
                        rule,
                        data.syllable_edit_mode,
                        &data.graphemes,
                        &mut order,
                        &mut new_var,
                    );
                });
                ui.add_space(3.0);
            }
        }

        // add new variable if an unrecognized name was used
        if let Some(new_var) = new_var {
            // we have to use all() instead of contains() because we're comparing &str to String
            if SyllableRoots::names().all(|s| *s != new_var) {
                vars.entry(new_var).or_insert_with(Default::default);
            }
        }
    });
}

fn draw_or_node(
    ui: &mut egui::Ui,
    rule: &mut OrRule,
    mode: EditMode,
    graphemes: &grapheme::MasterGraphemeStorage,
    order: &mut usize,
    new_var: &mut Option<String>,
) {
    // draw head node
    let should_delete = draw_and_node(ui, &mut rule.head, mode, graphemes, order, new_var);
    if should_delete {
        rule.head.head = LeafRule::Uninitialized;
    }

    // draw remaining nodes
    rule.tail.retain_mut(|and_rule| {
        ui.heading("OR");
        !draw_and_node(ui, and_rule, mode, graphemes, order, new_var)
    });

    // draw button to insert new OR clause
    if mode.is_edit() && rule.head.head.initialized() {
        ui.add_space(12.0);
        LeafRule::menu(ui, "OR...", |new_rule| {
            rule.tail.push(AndRule::new(new_rule))
        });
    }
}

/// Draw an AND rule node. Return true if it should be deleted.
fn draw_and_node(
    ui: &mut egui::Ui,
    rule: &mut AndRule,
    mode: EditMode,
    graphemes: &grapheme::MasterGraphemeStorage,
    order: &mut usize,
    new_var: &mut Option<String>,
) -> bool {
    // draw button to insert node at beginning
    if mode.is_edit() && rule.head.initialized() {
        LeafRule::menu(ui, "+", |new_rule| rule.prepend(new_rule));
    }

    // draw first node
    let should_delete = draw_leaf_node(ui, &mut rule.head, mode, graphemes, order, new_var);
    if should_delete {
        if rule.tail.is_empty() {
            return true; // this was the last node, so delete this whole AndRule
        }
        rule.head = rule.tail.remove(0);
    }

    // draw remaining nodes
    match mode {
        EditMode::View => {
            for rule in &mut rule.tail {
                ui.label("+");
                draw_leaf_node(ui, rule, mode, graphemes, order, new_var);
            }
        }
        EditMode::Edit => {
            for i in 0..rule.tail.len() {
                LeafRule::menu(ui, "+", |new_rule| rule.tail.insert(i, new_rule));
                draw_leaf_node(ui, &mut rule.tail[i], mode, graphemes, order, new_var);
            }
        }
        EditMode::Delete => {
            rule.tail.retain_mut(|rule| {
                ui.label("+");
                !draw_leaf_node(ui, rule, mode, graphemes, order, new_var)
            });
        }
    }

    // draw button to insert node at end
    if mode.is_edit() && rule.head.initialized() {
        LeafRule::menu(ui, "+", |new_rule| rule.tail.push(new_rule));
    }

    false // don't delete this AndRule
}

/// Draw a leaf rule node. Return true if it should be deleted.
fn draw_leaf_node(
    ui: &mut egui::Ui,
    rule: &mut LeafRule,
    mode: EditMode,
    graphemes: &grapheme::MasterGraphemeStorage,
    order: &mut usize,
    new_var: &mut Option<String>,
) -> bool {
    *order += 1; // increment for each leaf node visited
    let response = match rule {
        LeafRule::Uninitialized => {
            if mode.is_edit() {
                LeafRule::menu(ui, "(click to set)", |new_rule| *rule = new_rule);
            } else {
                ui.colored_label(egui::Color32::RED, "(not set)");
            }
            return false; // not deleteable
        }
        LeafRule::Sequence(string, input) => ui.add(
            grapheme::GraphemeInputField::new(string, input, *order)
                .link(graphemes)
                .small(true)
                .allow_editing(mode.is_edit())
                .interactable(!mode.is_delete()),
        ),
        LeafRule::Set(set, input) => {
            ui.scope(|ui| {
                ui.label("{");
                ui.add(
                    grapheme::GraphemeInputField::new(set, input, *order)
                        .link(graphemes)
                        .small(true)
                        .allow_editing(mode.is_edit())
                        .interactable(!mode.is_delete()),
                );
                ui.label("}");
            })
            .response
        }
        LeafRule::Variable(input) => {
            if mode.is_edit() {
                let response = ui.add(
                    egui::TextEdit::singleline(input)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("Type...")
                        .desired_width(80.0),
                );
                if response.changed() && !input.is_empty() {
                    input.retain(|c| !c.is_whitespace());
                    *new_var = Some(input.clone());
                }
                response
            } else {
                let text = if !input.is_empty() {
                    egui::RichText::new(&*input).monospace()
                } else {
                    egui::RichText::new("(no variable given)").color(egui::Color32::RED)
                };
                ui.add(
                    egui::Label::new(text)
                        .selectable(mode.is_view())
                        .sense(egui::Sense::click()),
                )
            }
        }
        LeafRule::Blank => ui.add(
            egui::Label::new("blank")
                .selectable(mode.is_view())
                .sense(egui::Sense::click()),
        ),
    };
    util::draw_deletion_overlay(mode, ui, &response)
}

/// Perform a DFS through the syllable rules, starting at each of the root variables.
/// Visited variables are stored in the set `vars.reachable`.
fn flag_reachable_vars(vars: &mut SyllableVars) {
    vars.reachable.clear();
    let mut stack: VecDeque<&OrRule> = vars.roots.iter().collect();
    while let Some(next) = stack.pop_back() {
        next.iter()
            .flat_map(NonEmptyList::iter)
            .filter_map(|leaf| match leaf {
                LeafRule::Variable(var) => Some(var),
                _ => None,
            })
            .filter(|&var| vars.reachable.insert(var.clone())) // skip already-visited variables
            .filter_map(|var| vars.vars.get(var)) // map name to rule and skip root variables
            .for_each(|rule| stack.push_back(rule))
    }
}

/// Return true if the synthesis configuration is in a valid state, otherwise false.
pub fn is_config_valid(data: &SynthesisTab) -> bool {
    verify_weights(&data.syllable_wgts.0) && verify_weights(&data.syllable_wgts.1)
}

/// Generate and return a new morpheme using the given settings.
pub fn synthesize_morpheme(vars: &SyllableVars, weights: &[u16]) -> String {
    let mut output = String::new();
    let mut rng = thread_rng();
    let num_syllables = 1 + WeightedIndex::new(weights)
        .unwrap() // weights already sanitized by front end (don't do this for secure stuff!)
        .sample(&mut rng);
    if num_syllables == 1 {
        synthesize_syllable(&vars.roots.single, vars, &mut output, &mut rng);
    } else {
        synthesize_syllable(&vars.roots.initial, vars, &mut output, &mut rng);
        for _ in 0..num_syllables - 2 {
            synthesize_syllable(&vars.roots.middle, vars, &mut output, &mut rng);
        }
        synthesize_syllable(&vars.roots.terminal, vars, &mut output, &mut rng);
    }
    output
}

/// Generate a syllable using the provided rule and append it to `output`.
fn synthesize_syllable(
    rule: &OrRule,
    vars: &SyllableVars,
    output: &mut String,
    rng: &mut impl Rng,
) {
    let or_clause = rule.iter().choose(rng).unwrap();
    for rule in or_clause.iter() {
        match rule {
            LeafRule::Sequence(list, _) => {
                for grapheme in list {
                    output.push_str(grapheme.as_str());
                }
            }
            LeafRule::Set(list, _) => {
                if let Some(grapheme) = list.iter().choose(rng) {
                    output.push_str(grapheme.as_str());
                }
            }
            LeafRule::Variable(var) => {
                if let Some(new_rule) = vars.get(var) {
                    synthesize_syllable(new_rule, vars, output, rng);
                }
            }
            LeafRule::Blank | LeafRule::Uninitialized => {}
        }
    }
}

/// Return true if the sum of a slice of weights equals 100, otherwise false.
fn verify_weights(weights: &[u16]) -> bool {
    weights.iter().sum::<u16>() == 100
}

fn int_field_1_to_100(value: &mut u8) -> egui::DragValue {
    egui::DragValue::new(value).clamp_range(1..=100).speed(0.05)
}

fn int_field_percent(value: &mut u16) -> egui::DragValue {
    egui::DragValue::new(value).clamp_range(0..=100).suffix("%")
}
