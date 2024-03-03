use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use eframe::egui::{Color32, Frame, RichText, ScrollArea, Ui, Vec2};
use serde::{Deserialize, Serialize};
use crate::Language;
use crate::util::{draw_deletion_overlay, EditMode};

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

/// The type of one element in a find pattern or a replace pattern.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PatternType {
	Phrase(PhraseType),
	Word(WordType),
	Literal(String)
}

#[derive(Deserialize, Serialize)]
pub struct FindPattern {
	pattern: PatternType,
	multimatch: bool,           // also match all adjacent constituents of same type
	optional: bool,             // also match even if not present
	children: Vec<FindPatternRef>,
    label: String,
    short_label_len: usize      // label size before any nested labels
}

// A reference-counted FindPattern.
type FindPatternRef = Rc<RefCell<FindPattern>>;

// A reference to a FindPattern that automatically becomes invalid if the FindPattern is deleted.
type FindPatternWeakRef = Weak<RefCell<FindPattern>>;

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

    /// Get the "short" version of the label, without any sub-patterns.
    fn short_label(&self) -> &str {
        &self.label[..self.short_label_len]
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
            for sub_pattern in &self.children {
                sub_pattern.borrow_mut().compute_label(counter);
                self.label.push_str(&sub_pattern.borrow().label);
            }
            self.label.push_str(" }");
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum ReplacePattern {
	Capture {
        #[serde(skip)] capture: FindPatternWeakRef,
        serde_label: String
    },
	Literal(String)
}

impl ReplacePattern {
    fn is_valid(&self) -> bool {
        match self {
            ReplacePattern::Capture { capture: find_pattern, serde_label: _ } => find_pattern.upgrade().is_some(),
            ReplacePattern::Literal(_) => true
        }
    }

    fn as_dbg_text(&self) -> String {
        // todo replace this with a proper button
        match self {
            ReplacePattern::Capture { capture, .. } => match capture.upgrade() {
                Some(find_pattern) => find_pattern.borrow().short_label().to_owned(),
                None => String::new()
            },
            ReplacePattern::Literal(literal) => format!("\"{literal}\"")
        }
    }
}

/// A rule in a language's grammar, which maps a "find pattern" to a "replace pattern".
/// Analagous to a production in a context-sensitive grammar.
#[derive(Default, Deserialize, Serialize)]
pub struct GrammarRule {
    find_patterns: Vec<FindPatternRef>,
    replace_patterns: Vec<ReplacePattern>
}

/// Render contents of the 'grammar' tab.
pub fn draw_grammar_tab(ui: &mut Ui, curr_lang: &mut Language) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Rules");
        ui.add_space(5.0);
        EditMode::draw_mode_picker(ui, &mut curr_lang.grammar_edit_mode);
        let mode = curr_lang.grammar_edit_mode;
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
                    curr_lang.grammar_rules.push(Default::default());
                }
            }
        });
    });
}

/// Render the find and replace patterns for a grammar rule.
fn draw_rule(rule: &mut GrammarRule, ui: &mut Ui, mode: EditMode) {
    if rule.find_patterns.is_empty() {
        // no find pattern has been set yet
        draw_find_node_selector(ui, mode, |new| {
            rule.find_patterns.push(new);
            recompute_pattern_labels(rule);
        });
    } else {
        // we have a find pattern
        let is_modified = draw_find_patterns(&mut rule.find_patterns, ui, mode);
        if is_modified {
            recompute_pattern_labels(rule);
        }
        ui.label("->");
        if !rule.replace_patterns.is_empty() {
            draw_replace_patterns(rule, ui, mode);
        } else if mode.is_edit() {
            draw_replace_node_selector(ui, mode, &rule.find_patterns, |new| rule.replace_patterns.push(new));
        } else {
            ui.colored_label(Color32::RED, "(not set)");
        }
    }
}

/// Render the "find" portion of a grammar rule. Return true if any elements were modified or deleted.
fn draw_find_patterns(patterns: &mut Vec<FindPatternRef>, ui: &mut Ui, mode: EditMode) -> bool {
    let mut modified = false;
    match mode {
        EditMode::View => {
            for pattern in patterns {
                draw_find_node(&mut pattern.borrow_mut(), ui, mode);
            }
        },
        EditMode::Edit => {
            for i in 0..patterns.len() {
                modified |= draw_find_pattern_menu(ui, "+", |new| patterns.insert(i, new));
                modified |= draw_find_node(&mut patterns[i].borrow_mut(), ui, mode);
            }
            modified |= draw_find_pattern_menu(ui, "+", |new| patterns.push(new));
        },
        EditMode::Delete => {
            patterns.retain(|pattern| {
                let child_modified = draw_find_node(&mut pattern.borrow_mut(), ui, mode);
                modified |= child_modified;
                !child_modified
            });
        }
    }
    modified
}

/// Render the "replace" portion of a rule.
fn draw_replace_patterns(rule: &mut GrammarRule, ui: &mut Ui, mode: EditMode) {
    match mode {
        EditMode::View => {
            for pattern in &mut rule.replace_patterns {
                draw_replace_node(pattern, ui, mode);
            }
        },
        EditMode::Edit => {
            for i in 0..rule.replace_patterns.len() {
                draw_replace_pattern_menu(ui, "+", &rule.find_patterns, |new| rule.replace_patterns.insert(i, new));
                draw_replace_node(&mut rule.replace_patterns[i], ui, mode);
            }
            draw_replace_pattern_menu(ui, "+", &rule.find_patterns, |new: ReplacePattern| rule.replace_patterns.push(new));
        },
        EditMode::Delete => {
            rule.replace_patterns.retain_mut(|pattern| {
                let should_delete = draw_replace_node(pattern, ui, mode);
                !should_delete && pattern.is_valid()
            });
        }
    }
}

/// Render one element in a "find" pattern. Return true if the element was modified or deleted.
fn draw_find_node(node: &mut FindPattern, ui: &mut Ui, mode: EditMode) -> bool {
    let text = RichText::new(&node.label).monospace();
    if !mode.is_edit() {
        let node = ui.button(text);
        draw_deletion_overlay(mode, ui, &node)
    } else {
        let mut modified = false;
        ui.menu_button(text, |ui| {
            Frame::none().inner_margin(Vec2::splat(6.0)).show(ui, |ui| {
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
                            modified |= ui.text_edit_singleline(word).changed();
                        });
                    }
                }
                ui.separator();
                modified |= ui.checkbox(&mut node.multimatch, "Group Matching")
                    .on_hover_text("Capture all adjacent elements of this type")
                    .changed();
                modified |= ui.checkbox(&mut node.optional, "Optional Matching")
                    .on_hover_text("Match this rule even if this element is not present")
                    .changed();
                if !matches!(node.pattern, PatternType::Literal(_)) {
                    ui.separator();
                    for child_node in &mut node.children {
                        modified |= draw_find_node(&mut child_node.borrow_mut(), ui, mode);
                    }
                    modified |= draw_find_pattern_menu(ui, "Add Deep Match...", |new| node.children.push(new));
                }
            });
        });
        modified
    }
}

/// Render one element in a "replace" pattern. Return true if the element should be deleted.
fn draw_replace_node(node: &mut ReplacePattern, ui: &mut Ui, mode: EditMode) -> bool {
    let node = ui.button(node.as_dbg_text());
    draw_deletion_overlay(mode, ui, &node)
}

/// Render the "find" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `FindPatternRef` as the argument and then true is returned.
fn draw_find_node_selector(ui: &mut Ui, mode: EditMode, on_select: impl FnOnce(FindPatternRef)) -> bool {
    if mode.is_edit() {
        draw_find_pattern_menu(ui, "(click to set)", on_select)
    } else {
        ui.colored_label(Color32::RED, "(not set)");
        false
    }
}

/// Render the "replace" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `ReplacePatternR` as the argument.
fn draw_replace_node_selector(ui: &mut Ui, mode: EditMode, find_patterns: &[FindPatternRef],
    on_select: impl FnOnce(ReplacePattern))
{
    if mode.is_edit() {
        draw_replace_pattern_menu(ui, "(click to set)", find_patterns, on_select);
    } else {
        ui.colored_label(Color32::RED, "(not set)");
    }
}

/// Render a "find" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `FindPatternRef` as the argument and then true is returned.
fn draw_find_pattern_menu(ui: &mut Ui, text: &str, action: impl FnOnce(FindPatternRef)) -> bool {
    let new_pattern = ui.menu_button(text,
        |ui| {
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
        }).inner.flatten();
    if let Some(new_pattern) = new_pattern {
        action(Rc::new(RefCell::new(FindPattern::new(new_pattern))));
        true
    } else {
        false
    }
}

/// Render a "replace" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `ReplacePattern` as the argument.
fn draw_replace_pattern_menu(ui: &mut Ui, text: &str, choices: &[FindPatternRef],
    action: impl FnOnce(ReplacePattern))
{
    let response = ui.menu_button(text, |ui| {
        for choice in choices {
            let mut selected = None;
            for_each_in_subtree(choice, |node| {
                if ui.button(node.borrow().short_label()).clicked() {
                    ui.close_menu();
                    selected = Some(ReplacePattern::Capture {
                        capture: Rc::downgrade(node),
                        serde_label: String::new()
                    });
                }
            });
            if selected.is_some() {
                return selected;
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

/// Apply a function to each "find" pattern that is part of this pattern, including the root pattern
/// itself and any deep match patterns.
fn for_each_in_subtree(root: &FindPatternRef, mut function: impl FnMut(&FindPatternRef)) {
    function(root);
    for sub_pattern in &root.borrow().children {
        function(sub_pattern);
    }
}

/// Recompute the text labels for all the pattern nodes in this rule. This should be
/// called whenever the order of the nodes changes, or when some part of a node changes
/// that is reflected in its label.
fn recompute_pattern_labels(rule: &mut GrammarRule) {
    let mut counter = HashMap::with_capacity(rule.find_patterns.len());
    for pattern in &rule.find_patterns {
        for_each_in_subtree(pattern, |pattern| {
            counter.entry(pattern.borrow().id())
                .and_modify(|(_, max)| *max += 1)
                .or_insert((0u32, 1u32));
        });
    }
    for node in &mut rule.find_patterns {
        node.borrow_mut().compute_label(&mut counter);
    }
}

/// Because `ReplacePattern::Capture` contains a `Weak` reference to the captured `FindPattern`,
/// it can't be serialized directly. So we also serialize the `FindPattern`'s current label, and
/// during deserialization we use the label to associate with the correct `FindPattern`.
pub fn save_grammar_serde_metadata(rules: &mut Vec<GrammarRule>) {
    for rule in rules {
        for replace_pattern in &mut rule.replace_patterns {
            if let ReplacePattern::Capture { capture, serde_label } = replace_pattern {
                *serde_label = capture.upgrade()
                    .map(|find_pattern| find_pattern.borrow().short_label().to_owned())
                    .unwrap_or_default();
            }
        }
    }
}

/// See `save_grammar_serde_metadata()` for why this function exists.
pub fn load_grammar_serde_metadata(rules: &mut Vec<GrammarRule>) {
    for rule in rules {
        // map this rule's labels to their corresponding find patterns
        let find_pattern_labels: HashMap<String, FindPatternRef> = rule.find_patterns.iter()
            .map(|find_pattern| (find_pattern.borrow().short_label().to_owned(), Rc::clone(find_pattern)))
            .collect();

        // look up each replace pattern's deserialized label to get a reference to the captured find pattern
        for replace_pattern in &mut rule.replace_patterns {
            if let ReplacePattern::Capture { capture, serde_label } = replace_pattern {
                match find_pattern_labels.get(serde_label) {
                    Some(find_pattern) => *capture = Rc::downgrade(find_pattern),
                    None => *capture = Weak::new()
                }
            }
        }
    }
}