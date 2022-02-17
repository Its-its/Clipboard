use chrono::Utc;
use eframe::{egui, epi};
use log::error;


use crate::{Tab, ReturnedItem, StorageContainer, StorageQuery, Config};


#[derive(Default)]
pub struct SearchTab {
	search: String,
	items: Vec<ReturnedItem>,
	fetching_items: bool
}

impl SearchTab {
	pub fn fetch(&mut self, frame: &epi::Frame, store: &StorageContainer) {
		if self.fetching_items {
			return;
		}

		self.items.clear();

		if self.search.is_empty() {
			return;
		}

		self.fetching_items = true;

		match store.query(StorageQuery::Search { value: self.search.clone() }) {
			Ok(new_items) => super::prepend_new_items_into_existing(&mut self.items, new_items, frame),
			Err(e) => error!(target: "clipboard_gui", "{:?}", e),
		}

		self.fetching_items = false;
	}
}

impl Tab for SearchTab {
	fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame, store: &StorageContainer, config: &mut Config) {
		egui::CentralPanel::default()
		.show(ctx, |ui| {
			ui.label("Type in your query below");

			let text_edit = ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(f32::INFINITY));

			if text_edit.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
				self.fetch(frame, store);
			}

			egui::ScrollArea::vertical()
			.show_rows(ui, 40.0, self.items.len(), |ui, viewing| {
				let mut removed_data_index: Option<usize> = None;

				let desired_size = egui::vec2(ui.available_width(), 40.0);

				let now = Utc::now();

				for item in &mut self.items[viewing] {
					super::display_scroll_row(ui, desired_size, item, config, store, &mut removed_data_index, now);

					ui.separator();
				}

				// If you clicked the Remove Button
				if let Some(data_id) = removed_data_index {
					// Remove all items in view list.
					for index in self.items.len() - 1..=0 {
						if self.items[index].data_id == data_id {
							self.items.remove(index);
						}
					}

					store.delete(data_id).unwrap();
				}

				if self.fetching_items {
					ui.allocate_ui_with_layout(desired_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
						ui.label(egui::RichText::new("Loading...").color(egui::Rgba::from_rgb(1.0, 1.0, 1.0)).strong().heading());
					});
				}
			});
		});
	}
}