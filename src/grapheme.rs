use std::hash::Hash;
use eframe::egui::{Id, Response, TextEdit, Ui, Widget};

/// A grapheme or multigraph.
pub struct Grapheme(String);

/// A TextField-like widget for storing graphemes.
pub struct GraphemeInputField<'data, 'buffer> {
    graphemes: &'data mut Vec<Grapheme>,
    input: &'buffer mut String,
    id: Id
}

impl<'data, 'buffer> GraphemeInputField<'data, 'buffer> {
    /// Create a new GraphemeInputField that stores its data in `graphemes` and uses
    /// `input` as an input buffer while the user is typing. A unique id is required to
    /// keep the input field focused after adding a new grapheme.
    pub fn new(graphemes: &'data mut Vec<Grapheme>, input: &'buffer mut String, id: impl Hash)
        -> GraphemeInputField<'data, 'buffer>
    {
        GraphemeInputField { graphemes, input, id: Id::new(id) }
    }
}

impl<'data, 'buffer> Widget for GraphemeInputField<'data, 'buffer> {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                // add extra space between graphemes
                ui.spacing_mut().item_spacing.x += 3.0;
    
                // draw graphemes, and remove them if clicked
                self.graphemes.retain(|grapheme| {
                    !ui.button(&grapheme.0).on_hover_text("Click to remove").clicked()
                });
    
                // draw new input field at end
                let new_grapheme = ui.add(TextEdit::singleline(self.input)
                    .frame(false)
                    .hint_text("Add a grapheme...")
                    .id(self.id));
            
                // add grapheme on space, enter, or focus loss
                if new_grapheme.changed() {
                    while let Some(space_pos) = self.input.find(char::is_whitespace) {
                        if space_pos > 0 {
                            self.graphemes.push(Grapheme(self.input[..space_pos].to_owned()));
                        }
                        self.input.replace_range(..=space_pos, "");
                    }
                }
                if new_grapheme.lost_focus() && !self.input.is_empty() {
                    self.graphemes.push(Grapheme(self.input.clone()));
                    self.input.clear();
                }
            });
        }).response
    }
}