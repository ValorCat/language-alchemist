use egui::{Id, TextEdit, Ui};
use crate::Language;

pub struct Grapheme(String);

/// Render contents of the 'synthesis' tab.
pub fn draw_synthesis_tab(ui: &mut Ui, curr_lang: &mut Language) {
    ui.heading("Graphemic Inventory");
    ui.group(|ui| {
        ui.horizontal_wrapped(|ui| {
            // add extra space between graphemes
            ui.spacing_mut().item_spacing.x += 3.0;

            // draw graphemes, and remove them if clicked
            curr_lang.graphemes.retain(|grapheme| {
                !ui.button(&grapheme.0).on_hover_text("Click to remove").clicked()
            });

            // draw new grapheme text field at end
            let new_grapheme = ui.add(TextEdit::singleline(&mut curr_lang.new_grapheme)
                .frame(false)
                .hint_text("Add a grapheme...")
                .id(Id::new("new grapheme")));
            
            // add grapheme on space, enter, or focus loss
            if new_grapheme.changed() {
                while let Some(space_pos) = curr_lang.new_grapheme.find(char::is_whitespace) {
                    if space_pos > 0 {
                        curr_lang.graphemes.push(Grapheme(curr_lang.new_grapheme[..space_pos].to_owned()));
                    }
                    curr_lang.new_grapheme.replace_range(..=space_pos, "");
                }
            }
            if new_grapheme.lost_focus() && !curr_lang.new_grapheme.is_empty() {
                curr_lang.graphemes.push(Grapheme(curr_lang.new_grapheme.clone()));
                curr_lang.new_grapheme.clear();
            }
        });
    });
}
