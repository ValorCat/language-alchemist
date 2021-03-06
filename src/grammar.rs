use std::collections::HashMap;
use std::hash::Hash;
use eframe::egui::{Color32, Frame, RichText, ScrollArea, Ui, Vec2};
use serde::{Deserialize, Serialize};
use crate::Language;
use crate::util::EditMode;

/// A word in the input text.
#[derive(Deserialize, Serialize)]
pub struct Word(String, WordType); // todo add Vec<WordAttribute>

/// A word type, roughly analogous to a part of speech, but simplified to support arbitrary languages.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WordType {
    Adposition, Conjunction, Determiner, Noun, NounModifier, Pronoun, Verb, VerbModifier
}

impl WordType {
    fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Adposition, Self::Conjunction, Self::Determiner, Self::Noun, Self::NounModifier,
            Self::Pronoun, Self::Verb, Self::VerbModifier
        ].into_iter()
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Adposition => "Adposition",
            Self::Conjunction => "Conjunction",
            Self::Determiner => "Determiner",
            Self::Noun => "Noun",
            Self::NounModifier => "Noun Modifier",
            Self::Pronoun => "Pronoun",
            Self::Verb => "Verb",
            Self::VerbModifier => "Verb Modifier"
        }
    }

    fn short_name(&self) -> &'static str {
        match self {
            Self::Adposition => "Adp",
            Self::Conjunction => "Conj",
            Self::Determiner => "Det",
            Self::Noun => "Noun",
            Self::NounModifier => "NM",
            Self::Pronoun => "Pro",
            Self::Verb => "Verb",
            Self::VerbModifier => "VM"
        }
    }
}

// pub enum Constituent {
//     Phrase(PhraseType, Vec<Constituent>),
//     Word(Word)
// }

/// A phrase type, roughly analogous to a constituent type in linguistic syntax. A phrase is composed
/// of words and other phrases.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PhraseType {
    Action, Argument, Clause, /*Conjunction,*/ Relation
}

impl PhraseType {
    fn iter() -> impl Iterator<Item = Self> {
        [Self::Action, Self::Argument, Self::Clause, Self::Relation].into_iter()
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Action => "Action Phrase",
            Self::Argument => "Argument Phrase",
            Self::Clause => "Clause Phrase",
            Self::Relation => "Relation Phrase"
        }
    }

    fn short_name(&self) -> &'static str {
        match self {
            Self::Action => "Action",
            Self::Argument => "Arg",
            Self::Clause => "Clause",
            Self::Relation => "Rel"
        }
    }
}

/// A rule in a language's grammar, which maps a "find pattern" to a "replace pattern".
/// Analagous to a production in a context-sensitive grammar.
#[derive(Deserialize, Serialize)]
pub struct GrammarRule {
    find: Vec<FindPattern>,
    replace: Vec<ReplacePattern>
}

#[derive(Deserialize, Serialize)]
pub struct FindPattern {
	pattern: PatternType,
	multimatch: bool,           // also match all adjacent constituents of same type
	optional: bool,             // also match even if not present
	children: Vec<FindPattern>, // only match if these sub-constituents also match
    label: String,
    short_label_len: usize      // label size before any nested labels
}

// The unique portion of a FindPattern, used for equality checking and hashing.
type FindPatternId = (PatternType, bool, bool);

impl FindPattern {
    fn new(pattern: PatternType) -> Self {
        Self { pattern, multimatch: false, optional: false, children: vec![], label: String::new(), short_label_len: 0 }
    }

    /// Get the unique portion of this pattern.
    fn id(&self) -> FindPatternId {
        (self.pattern.clone(), self.multimatch, self.optional)
    }

    /// Get an iterator over all the "find" patterns that are part of this pattern, including itself
    /// and any deep match patterns.
    fn subtree(this: &FindPattern) -> Box<dyn Iterator<Item = &FindPattern> + '_> {
        Box::new(
            std::iter::once(this) // root node
            .chain(this.children.iter().flat_map(FindPattern::subtree)) // child nodes
        )
    }

    /// Compute and save this node's label. It can be accessed later through the `self.label` field.
    fn compute_label(&mut self, counter: &mut HashMap<FindPatternId, (u32, u32)>) {
        self.label.clear();
        
        // add abbreviated type name
        match &self.pattern {
            PatternType::Phrase(ty) => self.label.push_str(ty.short_name()),
            PatternType::Word(ty) => self.label.push_str(ty.short_name()),
            PatternType::Literal(word) => {
                self.label.push('"');
                self.label.push_str(word);
                self.label.push('"');
            }
        }

        // add type modifiers (*, +, ?)
        match (self.multimatch, self.optional) {
            (true, true) => self.label.push('*'),
            (true, false) => self.label.push('+'),
            (false, true) => self.label.push('?'),
            (false, false) => {}
        }

        // add numeric identifier if there are multiple uses of this type
        if let Some((count, max)) = counter.get_mut(&self.id()) {
            if *max > 1 && count < max {
                *count += 1;
                self.label.push(' ');
                self.label.push_str(&count.to_string());
            }
        }

        // short label ends here
        self.short_label_len = self.label.len();

        // add nested patterns in braces
        if !self.children.is_empty() {
            self.label.push_str(" { ");
            for sub_pattern in &mut self.children {
                sub_pattern.compute_label(counter);
                self.label.push_str(&sub_pattern.label);
            }
            self.label.push_str(" }");
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum ReplacePattern {
	Capture(usize), // e.g. "Pronoun #4"
	Literal(String)
}

/// The type of one element in a find pattern or a replace pattern.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PatternType {
	Phrase(PhraseType),
	Word(WordType),
	Literal(String)
}

/// Render contents of the 'grammar' tab.
pub fn draw_grammar_tab(ui: &mut Ui, curr_lang: &mut Language) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Rules");
        ui.add_space(5.0);
        EditMode::draw_mode_picker(ui, &mut curr_lang.grammar_edit_mode);
        let mode = &curr_lang.grammar_edit_mode;
        ui.add_space(5.0);
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            for (i, rule) in curr_lang.grammar_rules.iter_mut().enumerate() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{}.", i + 1));
                    draw_rule(rule, ui, mode);
                });
                ui.add_space(3.0);
            }
            if mode.is_edit() {
                if !curr_lang.grammar_rules.is_empty() {
                    ui.add_space(7.0);
                }
                if ui.button("Add Rule").clicked() {
                    curr_lang.grammar_rules.push(None);
                }
            }
        });
    });
}

/// Render the find and replace patterns for a grammar rule.
fn draw_rule(rule: &mut Option<GrammarRule>, ui: &mut Ui, mode: &EditMode) {
    match rule {
        None => {
            // no find pattern has been set yet
            draw_find_node_selector(ui, mode, |new| {
                let mut new_rule = GrammarRule { find: vec![new], replace: vec![] };
                recompute_pattern_labels(&mut new_rule);
                *rule = Some(new_rule);
            });
        }
        Some(rule) => {
            // we have a find pattern
            if draw_find_patterns(&mut rule.find, ui, mode) {
                recompute_pattern_labels(rule);
            }
            ui.label("->");
            let find = &mut rule.find;
            if !rule.replace.is_empty() {
                draw_replace_patterns(find, &mut rule.replace, ui, mode);
            } else if mode.is_edit() {
                draw_replace_node_selector(ui, mode, find, |new| rule.replace.push(new));
            } else {
                ui.colored_label(Color32::RED, "(not set)");
            }
        }
    }
}

/// Render the "find" portion of a grammar rule. Return true if any nodes were changed.
fn draw_find_patterns(patterns: &mut Vec<FindPattern>, ui: &mut Ui, mode: &EditMode) -> bool {
    let mut changed = false;
    if !mode.is_edit() {
        // view and delete modes
        for node in patterns.iter_mut() {
            changed |= draw_find_node(node, ui, mode);
        }
    } else {
        // edit mode
        for i in 0..patterns.len() {
            changed |= draw_find_pattern_menu(ui, "+", |new| patterns.insert(i, new));
            changed |= draw_find_node(&mut patterns[i], ui, mode);
        }
        changed |= draw_find_pattern_menu(ui, "+", |new| patterns.push(new));
    }
    changed
}

/// Render the "replace" portion of a rule.
fn draw_replace_patterns(_find: &[FindPattern], _replace: &mut Vec<ReplacePattern>, _ui: &mut Ui, _mode: &EditMode) {
    
}

/// Render one element in a "find" pattern. Return true if any part of the node was changed.
fn draw_find_node(node: &mut FindPattern, ui: &mut Ui, mode: &EditMode) -> bool {
    let text = RichText::new(&node.label).monospace();
    match mode {
        EditMode::View => {
            let _ = ui.button(text);
            false // nothing was changed
        }
        EditMode::Edit => {
            let mut changed = false;
            ui.menu_button(text, |ui| {
                Frame::none().margin(Vec2::splat(6.0)).show(ui, |ui| {
                    match &mut node.pattern {
                        PatternType::Phrase(ty) => {
                            ui.label(ty.name());
                        }
                        PatternType::Word(ty) => {
                            ui.label(ty.name());
                        }
                        PatternType::Literal(word) => {
                            ui.horizontal(|ui| {
                                ui.label("Exact Word: ");
                                changed |= ui.text_edit_singleline(word).changed();
                            });
                        }
                    }
                    ui.separator();
                    changed |= ui.checkbox(&mut node.multimatch, "Group Matching")
                        .on_hover_text("Capture all adjacent elements of this type")
                        .changed();
                    changed |= ui.checkbox(&mut node.optional, "Optional Matching")
                        .on_hover_text("Match this rule even if this element is not present")
                        .changed();
                    if !matches!(node.pattern, PatternType::Literal(_)) {
                        ui.separator();
                        for child_node in &mut node.children {
                            changed |= draw_find_node(child_node, ui, mode);
                        }
                        changed |= draw_find_pattern_menu(ui, "Add Deep Match...", |new| node.children.push(new));
                    }
                });
            });
            changed
        }
        EditMode::Delete => todo!()
    }
}

/// Render the "find" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `FindPattern` as the argument and then true is returned.
fn draw_find_node_selector(ui: &mut Ui, mode: &EditMode, on_select: impl FnOnce(FindPattern)) -> bool {
    match mode {
        EditMode::View => {
            ui.colored_label(Color32::RED, "(not set)");
            false
        }
        EditMode::Edit => draw_find_pattern_menu(ui, "(click to set)", on_select),
        EditMode::Delete => todo!()
    }
}

/// Render the "replace" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `ReplacePattern` as the argument.
fn draw_replace_node_selector(ui: &mut Ui, mode: &EditMode, find_patterns: &[FindPattern],
    on_select: impl FnOnce(ReplacePattern))
{
    match mode {
        EditMode::View => {
            ui.colored_label(Color32::RED, "(not set)");
        }
        EditMode::Edit => draw_replace_pattern_menu(ui, "(click to set)", find_patterns, on_select),
        EditMode::Delete => todo!()
    }
}

/// Render a "find" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `FindPattern` as the argument and then true is returned.
fn draw_find_pattern_menu(ui: &mut Ui, text: &str, action: impl FnOnce(FindPattern)) -> bool {
    let response = ui.menu_button(text, |ui| {
        for choice in PhraseType::iter() {
            if ui.button(choice.name()).clicked() {
                ui.close_menu();
                return Some(PatternType::Phrase(choice));
            }
        }
        ui.separator();
        for choice in WordType::iter() {
            if ui.button(choice.name()).clicked() {
                ui.close_menu();
                return Some(PatternType::Word(choice));
            }
        }
        ui.separator();
        if ui.button("Exact Word").clicked() {
            ui.close_menu();
            return Some(PatternType::Literal("word".to_owned()));
        }
        None
    });
    response.inner.flatten()
        .map(|new| action(FindPattern::new(new)))
        .is_some()
}

/// Render a "replace" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `ReplacePattern` as the argument.
fn draw_replace_pattern_menu(ui: &mut Ui, text: &str, choices: &[FindPattern],
    action: impl FnOnce(ReplacePattern))
{
    let response = ui.menu_button(text, |ui| {
        for node in choices.iter().flat_map(FindPattern::subtree) {
            if ui.button(&node.label[..node.short_label_len]).clicked() {
                ui.close_menu();
                return Some(ReplacePattern::Capture(0));
            }
        }
        ui.separator();
        if ui.button("Exact Word").clicked() {
            ui.close_menu();
            return Some(ReplacePattern::Literal("word".to_owned()));
        }
        None
    });
    if let Some(new) = response.inner.flatten() {
        action(new);
    }
}

/// Recompute the text labels for all the pattern nodes in this rule. This should be
/// called whenever the order of the nodes changes, or when some part of a node changes
/// that is reflected in its label.
fn recompute_pattern_labels(rule: &mut GrammarRule) {
    let find_patterns = &mut rule.find;
    let mut counter = HashMap::with_capacity(find_patterns.len());
    for node in find_patterns.iter().flat_map(FindPattern::subtree) {
        counter.entry(node.id())
            .and_modify(|(_, max)| *max += 1)
            .or_insert((0u32, 1u32));
    }
    for node in find_patterns.iter_mut() {
        node.compute_label(&mut counter);
    }
}