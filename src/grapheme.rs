use std::collections::BTreeSet;
use std::hash::Hash;
use eframe::egui::{Button, Color32, Frame, Id, Response, RichText, TextEdit, Ui, Vec2, Widget};

/// A grapheme or multigraph.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Grapheme(String);

/// A container that can hold graphemes. The container can set its own policies on
/// ordering and duplicate permissability.
pub trait GraphemeStorage {
    /// Add a grapheme to the container.
    fn add(&mut self, grapheme: Grapheme);

    /// Return true if the container contains the given grapheme, otherwise false.
    fn contains(&self, grapheme: &Grapheme) -> bool;

    /// Return true if the container contains no graphemes, otherwise false.
    fn is_empty(&self) -> bool;

    /// Apply the given function to each grapheme, removing it if it returns false.
    fn update(&mut self, f: impl FnMut(&Grapheme) -> bool);
}

impl GraphemeStorage for Vec<Grapheme> {
    fn add(&mut self, grapheme: Grapheme) {
        self.push(grapheme);
    }

    fn contains(&self, grapheme: &Grapheme) -> bool {
        // deref to slice to avoid infinite recursion
        self[..].contains(grapheme)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn update(&mut self, f: impl FnMut(&Grapheme) -> bool) {
        self.retain(f);
    }
}

impl GraphemeStorage for BTreeSet<Grapheme> {
    fn add(&mut self, grapheme: Grapheme) {
        self.insert(grapheme);
    }

    fn contains(&self, grapheme: &Grapheme) -> bool {
        self.contains(grapheme)
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn update(&mut self, f: impl FnMut(&Grapheme) -> bool) {
        self.retain(f);
    }
}

/// The type of the master grapheme inventory, which other grapheme fields may be linked to.
pub type MasterGraphemeStorage = BTreeSet<Grapheme>;

/// A TextField-like widget for storing graphemes.
pub struct GraphemeInputField<'data, 'buffer, 'master, Storage: GraphemeStorage> {
    graphemes: &'data mut Storage,
    input: &'buffer mut String,
    master: Option<&'master MasterGraphemeStorage>,
    small: bool,
    id: Id
}

impl<'data, 'buffer, 'master, Storage: GraphemeStorage> GraphemeInputField<'data, 'buffer, 'master, Storage>
{
    /// Create a new GraphemeInputField that stores its data in `graphemes` and uses
    /// `input` as an input buffer while the user is typing. A unique id is required to
    /// keep the input field focused after adding a new grapheme.
    pub fn new(graphemes: &'data mut Storage, input: &'buffer mut String, id: impl Hash) -> Self
    {
        GraphemeInputField { graphemes, input, master: None, small: false, id: Id::new(id) }
    }

    /// Link this GraphemeInputField to a master list. Graphemes in this container
    /// will appear in red if they are not also in the master list.
    pub fn link(mut self, master: &'master MasterGraphemeStorage) -> Self {
        self.master = Some(master);
        self
    }

    /// Make the input field much lower profile. The frame border and hint text will
    /// disappear once some graphemes have been added.
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Draw the contents of the GraphemeInputField.
    fn show_contents(&mut self, ui: &mut Ui) -> Response {
        ui.horizontal_wrapped(|ui| {
            // add extra space between graphemes
            ui.spacing_mut().item_spacing.x += if self.small { -2.0 } else { 4.0 };
    
            // draw graphemes, and remove them if clicked
            self.graphemes.update(|grapheme| {
                // invalid if there is a master list and the grapheme isn't in it
                let invalid = self.master.map_or(false, |master| !master.contains(grapheme));
                let text = RichText::new(&grapheme.0);
                let text = if !invalid { text } else { text.color(Color32::RED) };
                let button = Button::new(text);
                let button = if !self.small { button } else { button.small() };
                let response = ui.add(button);
                let response = if !invalid { response } else {
                    response.on_hover_ui(|ui| {
                        ui.colored_label(Color32::RED, "Not in graphemic inventory");
                    })
                };
                !response.on_hover_text("Click to remove").clicked()
            });
    
            // hide input field on small instances when not moused over
            let visible_area = {
                let mut rect = ui.min_rect();
                *rect.right_mut() += 45.0;
                rect
            };
            if !self.small || self.graphemes.is_empty() || !self.input.is_empty() || ui.rect_contains_pointer(visible_area) {
                // draw input field at end
                let input_buffer = ui.add({
                    let text_edit = TextEdit::singleline(self.input)
                        .frame(false)
                        .id(self.id);
                    if self.small {
                        text_edit.hint_text("Add...").desired_width(35.0)
                    } else {
                        text_edit.hint_text("Add a grapheme...")
                    }
                });
                
                // add grapheme on space, enter, or focus loss
                if input_buffer.changed() {
                    while let Some(space_pos) = self.input.find(char::is_whitespace) {
                        if space_pos > 0 {
                            self.graphemes.add(Grapheme(self.input[..space_pos].to_owned()));
                        }
                        self.input.replace_range(..=space_pos, "");
                    }
                }
                if input_buffer.lost_focus() && !self.input.is_empty() {
                    self.graphemes.add(Grapheme(self.input.clone()));
                    self.input.clear();
                }
            }
        }).response
    }
}

impl<'data, 'buffer, 'master, Storage: GraphemeStorage> Widget
    for GraphemeInputField<'data, 'buffer, 'master, Storage>
{
    fn ui(mut self, ui: &mut Ui) -> Response {
        if self.small && !self.graphemes.is_empty() {
            // draw without a frame to save space
            self.show_contents(ui)
        } else {
            // fix height if small
            if self.small {
                ui.spacing_mut().interact_size.y = 16.0;
            }

            // draw within a frame
            Frame {
                margin: Vec2::splat(if self.small { 2.0 } else { 6.0 }),
                ..Frame::group(ui.style())
            }.show(ui, |ui| {
                self.show_contents(ui)
            }).response
        }
    }
}