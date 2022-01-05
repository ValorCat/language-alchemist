use std::collections::HashMap;
use std::hash::Hash;
use eframe::egui::{Color32, Frame, ScrollArea, Ui, Vec2};
use serde::{Deserialize, Serialize};
use crate::Language;
use crate::util::{EditMode, NonEmptyList};

/// A word in the input text.
#[derive(Deserialize, Serialize)]
struct Word(String, WordType); // todo add Vec<WordAttribute>

/// A word type, roughly analogous to a part of speech, but simplified to support arbitrary languages.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum WordType {
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

// enum Constituent {
//     Phrase(PhraseType, Vec<Constituent>),
//     Word(Word)
// }

/// A phrase type, roughly analogous to a constituent type in linguistic syntax. A phrase is composed
/// of words and other phrases.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum PhraseType {
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
#[derive(Default, Deserialize, Serialize)]
pub struct GrammarRule {
	find: Option<NonEmptyList<FindPattern>>,
	replace: Vec<ReplacePattern>,
	description: String
}

#[derive(Deserialize, Serialize)]
struct FindPattern {
	pattern: PatternType,
	match_adjacent: bool, // also match all adjacent constituents of same type
	match_optional: bool, // also match even if not present
	match_contains: Vec<FindPattern>, // only match if these sub-constituents also match
    label: String
}

// The unique portion of a FindPattern, used for equality checking and hashing.
type FindPatternId = (PatternType, bool, bool);

impl FindPattern {
    fn new(pattern: PatternType) -> Self {
        Self { pattern, match_adjacent: false, match_optional: false, match_contains: vec![], label: String::new() }
    }

    /// Get the unique portion of this pattern.
    fn id(&self) -> FindPatternId {
        (self.pattern.clone(), self.match_adjacent, self.match_optional)
    }

    /// Compute and save this node's label. It can be accessed later through the `self.label` field.
    fn compute_label(&mut self, counter: &mut HashMap<FindPatternId, (u32, u32)>) {
        self.label.clear();
        match &self.pattern {
            PatternType::Phrase(ty) => self.label.push_str(ty.short_name()),
            PatternType::Word(ty) => self.label.push_str(ty.short_name()),
            PatternType::Literal(word) => {
                self.label.push('"');
                self.label.push_str(word);
                self.label.push('"');
            }
        }
        match (self.match_adjacent, self.match_optional) {
            (true, true) => self.label.push('*'),
            (true, false) => self.label.push('+'),
            (false, true) => self.label.push('?'),
            (false, false) => {}
        }
        if let Some((count, max)) = counter.get_mut(&self.id()) {
            if *max > 1 && count < max {
                *count += 1;
                self.label.push(' ');
                self.label.push_str(&count.to_string());
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
enum ReplacePattern {
	Capture(PatternType, usize), // e.g. "Pronoun #4"
	OtherWord(Word),
	Literal(String)
}

/// The type of one element in a find pattern or a replace pattern.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
enum PatternType {
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
                    draw_find_pattern(rule, ui, mode);
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

/// Render the "find" portion of a grammar rule.
fn draw_find_pattern(rule: &mut GrammarRule, ui: &mut Ui, mode: &EditMode) {
    let mut changed = false;
    if let Some(find_patterns) = &mut rule.find {
        if !mode.is_edit() {
            // view and delete modes
            changed |= draw_find_node(&mut find_patterns.head, ui, mode);
            for node in &mut find_patterns.tail {
                changed |= draw_find_node(node, ui, mode);
            }
        } else {
            // edit mode
            changed |= draw_find_pattern_menu(ui, "+", |new_pattern| find_patterns.prepend(new_pattern));
            changed |= draw_find_node(&mut find_patterns.head, ui, mode);
            for i in 0..find_patterns.tail.len() {
                changed |= draw_find_pattern_menu(ui, "+", |new_pattern| find_patterns.tail.insert(i, new_pattern));
                changed |= draw_find_node(&mut find_patterns.tail[i], ui, mode);
            }
            changed |= draw_find_pattern_menu(ui, "+", |new_pattern| find_patterns.tail.push(new_pattern));
        }
    } else {
        // if pattern isn't set yet, draw the pattern selector
        changed |= draw_find_node_selector(ui, mode, |new_type| rule.find = Some(NonEmptyList::new(new_type)));
    }
    if changed {
        // if any part of this rule was changed, recompute all the labels
        // this ensures the numbering remains correct even when nodes are rearranged
        recompute_pattern_labels(rule);
    }
}

/// Render one element in a "find" pattern. Return true if any part of the node was changed.
fn draw_find_node(node: &mut FindPattern, ui: &mut Ui, mode: &EditMode) -> bool {
    let text = &node.label;
    match mode {
        EditMode::View => {
            let _ = ui.button(text);
            false // nothing was changed
        }
        EditMode::Edit => {
            let mut changed = false;
            ui.menu_button(text, |ui| {
                Frame::none().margin(Vec2::splat(6.0)).show(ui, |ui| {
                    let full_name = match &node.pattern {
                        PatternType::Phrase(ty) => ty.name().to_owned(),
                        PatternType::Word(ty) => ty.name().to_owned(),
                        PatternType::Literal(word) => format!("Literal \"{}\"", word)
                    };
                    ui.label(full_name);
                    ui.separator();
                    let response1 = ui.checkbox(&mut node.match_adjacent, "Group Matching")
                        .on_hover_text("Capture all adjacent elements of this type");
                    let response2 = ui.checkbox(&mut node.match_optional, "Optional Matching")
                        .on_hover_text("Match this rule even if this element is not present");
                    changed = response1.union(response2).changed();
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
        EditMode::Edit => {
            draw_find_pattern_menu(ui, "(click to set)", on_select)
        }
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
        if ui.button("Literal Word").clicked() {
            ui.close_menu();
            return Some(PatternType::Literal(String::new()));
        }
        None
    });
    if let Some(pattern_type) = response.inner.flatten() {
        action(FindPattern::new(pattern_type));
        true
    } else {
        false
    }
}

/// Recompute the text labels for all the pattern nodes in this rule. This should be
/// called whenever the order of the nodes changes, or when some part of a node changes
/// that is reflected in its label.
fn recompute_pattern_labels(rule: &mut GrammarRule) {
    if let Some(patterns) = &mut rule.find {
        let mut counter = HashMap::with_capacity(patterns.len());
        for node in patterns.iter() {
            counter.entry(node.id())
                .and_modify(|(_, max)| *max += 1)
                .or_insert((0u32, 1u32));
        }
        for node in patterns.iter_mut() {
            node.compute_label(&mut counter);
        }
    }
}