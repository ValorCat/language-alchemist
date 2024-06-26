use crate::util::{self, EditMode};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Default, Deserialize, Serialize)]
pub struct GrammarTab {
    pub grammar_rules: Vec<GrammarRule>,
    #[serde(skip)]
    grammar_edit_mode: EditMode,
}

/// A word in the input text.
#[derive(Deserialize, Serialize)]
pub struct Word(String, WordType); // todo add Vec<WordAttribute>

/// A word type, roughly analogous to a part of speech, but simplified to support arbitrary languages.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WordType {
    Adposition,
    Conjunction,
    Determiner,
    Noun,
    NounModifier,
    Pronoun,
    Verb,
    VerbModifier,
}

impl WordType {
    fn iter() -> impl Iterator<Item = Self> {
        [
            Self::Adposition,
            Self::Conjunction,
            Self::Determiner,
            Self::Noun,
            Self::NounModifier,
            Self::Pronoun,
            Self::Verb,
            Self::VerbModifier,
        ]
        .into_iter()
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
            Self::VerbModifier => "Verb Modifier",
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
            Self::VerbModifier => "VM",
        }
    }
}

/// A phrase type, roughly analogous to a constituent type in linguistic syntax. A phrase is composed
/// of words and other phrases.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PhraseType {
    Action,
    Argument,
    Clause,
    Relation,
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
            Self::Relation => "Relation Phrase",
        }
    }

    fn short_name(&self) -> &'static str {
        match self {
            Self::Action => "Action",
            Self::Argument => "Arg",
            Self::Clause => "Clause",
            Self::Relation => "Rel",
        }
    }
}

/// The type of one element in a find pattern or a replace pattern.
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PatternType {
    Phrase(PhraseType),
    Word(WordType),
    Literal(String),
}

#[derive(Deserialize, Serialize)]
pub struct FindPattern {
    pattern: PatternType,
    multimatch: bool, // also match all adjacent constituents of same type
    optional: bool,   // also match even if not present
    children: Vec<FindPatternRef>,
    label: String,
}

// A reference-counted FindPattern.
type FindPatternRef = Rc<RefCell<FindPattern>>;

// A reference to a FindPattern that automatically becomes invalid if the FindPattern is deleted.
type FindPatternWeakRef = Weak<RefCell<FindPattern>>;

// The unique portion of a FindPattern, used for equality checking and hashing.
type FindPatternId = (PatternType, bool, bool);

impl FindPattern {
    fn new(pattern: PatternType) -> Self {
        Self {
            pattern,
            multimatch: false,
            optional: false,
            children: vec![],
            label: String::new(),
        }
    }

    /// Get the unique portion of this pattern.
    fn id(&self) -> FindPatternId {
        (self.pattern.clone(), self.multimatch, self.optional)
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

        // recursively recompute labels of all children
        for sub_pattern in &self.children {
            sub_pattern.borrow_mut().compute_label(counter);
        }
    }
}

#[derive(Deserialize, Serialize)]
pub enum ReplacePattern {
    Capture {
        #[serde(skip)]
        capture: FindPatternWeakRef,
        serde_label: String,
    },
    Literal(String),
}

impl ReplacePattern {
    fn is_valid(&self) -> bool {
        match self {
            ReplacePattern::Capture {
                capture: find_pattern,
                ..
            } => find_pattern.upgrade().is_some(),
            ReplacePattern::Literal(_) => true,
        }
    }

    fn as_dbg_text(&self) -> String {
        // todo replace this with a proper button
        match self {
            ReplacePattern::Capture { capture, .. } => capture
                .upgrade()
                .map(|find_pattern| find_pattern.borrow().label.clone())
                .unwrap_or_default(),
            ReplacePattern::Literal(literal) => format!("\"{literal}\""),
        }
    }
}

/// A rule in a language's grammar, which maps a "find pattern" to a "replace pattern".
/// Analagous to a production in a context-sensitive grammar.
#[derive(Default, Deserialize, Serialize)]
pub struct GrammarRule {
    find_patterns: Vec<FindPatternRef>,
    replace_patterns: Vec<ReplacePattern>,
}

/// Render contents of the 'grammar' tab.
pub fn draw_grammar_tab(ui: &mut egui::Ui, data: &mut GrammarTab) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("Rules");
        ui.add_space(5.0);
        EditMode::draw_mode_picker(ui, &mut data.grammar_edit_mode);
        let mode = data.grammar_edit_mode;
        ui.add_space(5.0);
        ui.group(|ui| {
            ui.spacing_mut().item_spacing.y += 3.0;
            ui.add_space(ui.spacing().item_spacing.y); // match the extra space at the bottom
            ui.set_width(ui.available_width());

            let mut moved_rule = None;
            for (index, rule) in data.grammar_rules.iter_mut().enumerate() {
                let rule_id = egui::Id::new(format!("rule {index}"));
                let should_delete =
                    util::draw_reorderable(mode, ui, rule_id, index, &mut moved_rule, |ui| {
                        draw_rule(ui, rule, index, mode)
                    });
                if should_delete {
                    data.grammar_rules.remove(index);
                    break;
                }
                ui.add_space(3.0);
            }

            if mode.is_edit() {
                if !data.grammar_rules.is_empty() {
                    // draw space before 'add rule' button, which doubles as the drop zone for dragging a rule to the end
                    // we can't just call ui.add_space() because we need to check the space for hovers
                    let response = ui.allocate_rect(
                        egui::Rect::from_min_size(
                            ui.cursor().left_top(),
                            egui::Vec2::new(ui.available_width(), 10.0),
                        ),
                        egui::Sense::hover(),
                    );
                    util::draw_reorder_drop_area(
                        ui,
                        data.grammar_rules.len(),
                        &mut moved_rule,
                        &response,
                    );

                    // if any rules were dragged and released, move them now
                    if let Some(reordering) = moved_rule {
                        reordering.apply(&mut data.grammar_rules)
                    }
                }

                if ui.button("Add Rule").clicked() {
                    data.grammar_rules.push(Default::default());
                }
            }
        });
    });
}

/// Render the find and replace patterns for a grammar rule. Return the entire rule's Response, as well
/// as just the number label's Response (used for drag detection).
fn draw_rule(
    ui: &mut egui::Ui,
    rule: &mut GrammarRule,
    index: usize,
    mode: EditMode,
) -> (egui::Response, egui::Response) {
    let response = ui.horizontal_wrapped(|ui| {
        let label_sense = match mode {
            EditMode::View => egui::Sense::hover(),
            EditMode::Edit => egui::Sense::drag(),
            EditMode::Delete => egui::Sense::click(),
        };
        let number_label = egui::Label::new(format!("{}.", index + 1))
            .selectable(mode.is_view())
            .sense(label_sense);
        let label_response = ui.add(number_label);
        if rule.find_patterns.is_empty() {
            // no find pattern has been set yet
            draw_find_node_selector(ui, mode, |new| {
                rule.find_patterns.push(new);
                recompute_pattern_labels(rule);
            });
        } else {
            // we have a find pattern
            let mut was_modified = false;
            draw_find_patterns(ui, &mut rule.find_patterns, &mut was_modified, mode);
            if was_modified {
                recompute_pattern_labels(rule);
            }
            ui.label("->");
            if !rule.replace_patterns.is_empty() {
                draw_replace_patterns(ui, rule, mode);
            } else if mode.is_edit() {
                draw_replace_node_selector(ui, mode, &rule.find_patterns, |new| {
                    rule.replace_patterns.push(new)
                });
            } else {
                ui.colored_label(egui::Color32::RED, "(not set)");
            }
        }
        label_response
    });
    (response.response, response.inner)
}

/// Render the "find" portion of a grammar rule.
fn draw_find_patterns(
    ui: &mut egui::Ui,
    patterns: &mut Vec<FindPatternRef>,
    rule_modified: &mut bool,
    mode: EditMode,
) {
    match mode {
        EditMode::View => {
            for pattern in patterns {
                draw_find_node(ui, &mut pattern.borrow_mut(), rule_modified, mode);
            }
        }
        EditMode::Edit => {
            for i in 0..patterns.len() {
                *rule_modified |= draw_find_pattern_menu(ui, "+", |new| patterns.insert(i, new));
                draw_find_node(ui, &mut patterns[i].borrow_mut(), rule_modified, mode);
            }
            *rule_modified |= draw_find_pattern_menu(ui, "+", |new| patterns.push(new));
        }
        EditMode::Delete => {
            patterns.retain(|pattern| {
                let should_delete =
                    draw_find_node(ui, &mut pattern.borrow_mut(), rule_modified, mode);
                *rule_modified |= should_delete;
                !should_delete
            });
        }
    }
}

/// Render the "replace" portion of a rule.
fn draw_replace_patterns(ui: &mut egui::Ui, rule: &mut GrammarRule, mode: EditMode) {
    match mode {
        EditMode::View => {
            for pattern in &mut rule.replace_patterns {
                draw_replace_node(ui, pattern, mode);
            }
        }
        EditMode::Edit => {
            for i in 0..rule.replace_patterns.len() {
                draw_replace_pattern_menu(ui, "+", &rule.find_patterns, |new| {
                    rule.replace_patterns.insert(i, new)
                });
                draw_replace_node(ui, &mut rule.replace_patterns[i], mode);
            }
            draw_replace_pattern_menu(ui, "+", &rule.find_patterns, |new: ReplacePattern| {
                rule.replace_patterns.push(new)
            });
        }
        EditMode::Delete => {
            rule.replace_patterns.retain_mut(|pattern| {
                let should_delete = draw_replace_node(ui, pattern, mode);
                !should_delete && pattern.is_valid()
            });
        }
    }
}

/// Render one element in a "find" pattern. Return true if the element should be deleted.
fn draw_find_node(
    ui: &mut egui::Ui,
    node: &mut FindPattern,
    rule_modified: &mut bool,
    mode: EditMode,
) -> bool {
    let text = egui::RichText::new(&node.label).monospace();
    match mode {
        EditMode::View => {
            let _ = ui.button(text);
        }
        EditMode::Edit => {
            ui.menu_button(text, |ui| {
                egui::Frame::none()
                    .inner_margin(egui::Vec2::splat(6.0))
                    .show(ui, |ui| {
                        match &mut node.pattern {
                            PatternType::Phrase(ty) => ui.label(ty.name()),
                            PatternType::Word(ty) => ui.label(ty.name()),
                            PatternType::Literal(word) => {
                                ui.horizontal(|ui| {
                                    ui.label("Exact Word: ");
                                    *rule_modified |= ui.text_edit_singleline(word).changed();
                                })
                                .response
                            }
                        };
                        ui.separator();
                        *rule_modified |= ui
                            .checkbox(&mut node.multimatch, "Group Matching")
                            .on_hover_text("Capture all adjacent elements of this type")
                            .changed();
                        *rule_modified |= ui
                            .checkbox(&mut node.optional, "Optional Matching")
                            .on_hover_text("Match this rule even if this element is not present")
                            .changed();
                        if !matches!(node.pattern, PatternType::Literal(_)) {
                            ui.separator();
                            *rule_modified |=
                                draw_find_pattern_menu(ui, "Add Deep Match...", |new| {
                                    node.children.push(new)
                                });
                        }
                    });
            });
        }
        EditMode::Delete => {
            let node = ui.button(text);
            if util::draw_deletion_overlay(mode, ui, &node) {
                *rule_modified = true;
                return true;
            }
        }
    }
    if !node.children.is_empty() {
        ui.label("{");
        draw_find_patterns(ui, &mut node.children, rule_modified, mode);
        ui.label("}");
    }
    false
}

/// Render one element in a "replace" pattern. Return true if the element should be deleted.
fn draw_replace_node(ui: &mut egui::Ui, node: &mut ReplacePattern, mode: EditMode) -> bool {
    let text = egui::RichText::new(node.as_dbg_text()).monospace();
    let node = ui.button(text);
    util::draw_deletion_overlay(mode, ui, &node)
}

/// Render the "find" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `FindPatternRef` as the argument and then true is returned.
fn draw_find_node_selector(
    ui: &mut egui::Ui,
    mode: EditMode,
    on_select: impl FnOnce(FindPatternRef),
) -> bool {
    if mode.is_edit() {
        draw_find_pattern_menu(ui, "(click to set)", on_select)
    } else {
        ui.colored_label(egui::Color32::RED, "(not set)");
        false
    }
}

/// Render the "replace" pattern dropdown for a new rule. If an item is selected, the provided `on_select`
/// function is called with a new `ReplacePatternR` as the argument.
fn draw_replace_node_selector(
    ui: &mut egui::Ui,
    mode: EditMode,
    find_patterns: &[FindPatternRef],
    on_select: impl FnOnce(ReplacePattern),
) {
    if mode.is_edit() {
        draw_replace_pattern_menu(ui, "(click to set)", find_patterns, on_select);
    } else {
        ui.colored_label(egui::Color32::RED, "(not set)");
    }
}

/// Render a "find" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `FindPatternRef` as the argument and then true is returned.
fn draw_find_pattern_menu(
    ui: &mut egui::Ui,
    text: &str,
    action: impl FnOnce(FindPatternRef),
) -> bool {
    let new_pattern = ui
        .menu_button(text, |ui| {
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
        })
        .inner
        .flatten();
    if let Some(new_pattern) = new_pattern {
        action(Rc::new(RefCell::new(FindPattern::new(new_pattern))));
        true
    } else {
        false
    }
}

/// Render a "replace" pattern dropdown. If an item is selected, the provided `on_select` function is
/// called with a new `ReplacePattern` as the argument.
fn draw_replace_pattern_menu(
    ui: &mut egui::Ui,
    text: &str,
    choices: &[FindPatternRef],
    action: impl FnOnce(ReplacePattern),
) {
    let response = ui.menu_button(text, |ui| {
        for choice in choices {
            let mut selected = None;
            for_each_in_subtree(choice, |node| {
                if ui.button(&node.borrow().label).clicked() {
                    ui.close_menu();
                    selected = Some(ReplacePattern::Capture {
                        capture: Rc::downgrade(node),
                        serde_label: String::new(),
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
            counter
                .entry(pattern.borrow().id())
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
            if let ReplacePattern::Capture {
                capture,
                serde_label,
            } = replace_pattern
            {
                *serde_label = capture
                    .upgrade()
                    .map(|find_pattern| find_pattern.borrow().label.clone())
                    .unwrap_or_default();
            }
        }
    }
}

/// See `save_grammar_serde_metadata()` for why this function exists.
pub fn load_grammar_serde_metadata(rules: &mut Vec<GrammarRule>) {
    for rule in rules {
        // map this rule's labels to their corresponding find patterns
        let find_pattern_labels: HashMap<String, FindPatternRef> = rule
            .find_patterns
            .iter()
            .map(|find_pattern| (find_pattern.borrow().label.clone(), Rc::clone(find_pattern)))
            .collect();

        // look up each replace pattern's deserialized label to get a reference to the captured find pattern
        for replace_pattern in &mut rule.replace_patterns {
            if let ReplacePattern::Capture {
                capture,
                serde_label,
            } = replace_pattern
            {
                match find_pattern_labels.get(serde_label) {
                    Some(find_pattern) => *capture = Rc::downgrade(find_pattern),
                    None => *capture = Weak::new(),
                }
            }
        }
    }
}
