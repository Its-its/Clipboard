use std::sync::{RwLock, Arc};

use eframe::{egui::{self, TextureId}, epi};
use log::error;

use crate::{Config, StorageContainer, ReturnedItem, item_time_ago, ReturnedItemType};


mod recent;
mod search;
mod settings;
mod starred;


pub trait Tab {
	fn on_open(&mut self, _frame: &epi::Frame, _store: &StorageContainer, _config: &mut Config) {}
	fn on_close(&mut self, _frame: &epi::Frame) {}

	fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame, store: &StorageContainer, config: &mut Config);
}



pub struct App {
	viewing_tab: usize,
	tabs: Vec<Box<dyn Tab>>,
	store: StorageContainer,
	config: Arc<RwLock<Config>>,
}

impl App {
	pub fn new(store: StorageContainer, config: Arc<RwLock<Config>>) -> Self {
		Self {
			config,
			store,
			viewing_tab: 0,
			tabs: vec![
				Box::new(recent::RecentTab::default()),
				Box::new(starred::StarredTab::default()),
				Box::new(search::SearchTab::default()),
				Box::new(settings::SettingsTab::default()),
			]
		}
	}
}

impl epi::App for App {
	fn setup(&mut self, _ctx: &egui::CtxRef, frame: &epi::Frame, _storage: Option<&dyn epi::Storage>) {
		self.tabs[self.viewing_tab].on_open(frame, &self.store, &mut *self.config.write().unwrap());
	}

	fn update(&mut self, ctx: &egui::CtxRef, frame: &epi::Frame) {
		if ctx.input().key_pressed(egui::Key::Escape) {
			frame.quit();
			return;
		}

		let config = &mut *self.config.write().unwrap();

		egui::TopBottomPanel::top("top_panel")
		.show(ctx, |ui| {
			egui::menu::bar(ui, |ui| {
				let buttons = ["Recent", "Starred", "Search", "Settings"];

				for (index, text) in buttons.into_iter().enumerate() {
					if ui.selectable_label(self.viewing_tab == index, text).clicked() {
						// If we're changing tabs.
						if self.viewing_tab != index {
							self.tabs[self.viewing_tab].on_close(frame);
							self.viewing_tab = index;
							self.tabs[self.viewing_tab].on_open(frame, &self.store, config);
						}
					}
				}

				egui::warn_if_debug_build(ui);
			});
		});

		self.tabs[self.viewing_tab].update(ctx, frame, &self.store, config);
	}

	fn name(&self) -> &str {
		"Clipboard"
	}

	// fn warm_up_enabled(&self) -> bool {
	// 	false
	// }

	// fn save(&mut self, _storage: &mut dyn epi::Storage) {}

	// fn on_exit(&mut self) {}

	// fn auto_save_interval(&self) -> std::time::Duration {
	// 	std::time::Duration::from_secs(30)
	// }

	// fn max_size_points(&self) -> egui::Vec2 {
	// 	// Some browsers get slow with huge WebGL canvases, so we limit the size:
	// 	egui::Vec2::new(1024.0, 2048.0)
	// }

	// fn clear_color(&self) -> egui::Rgba {
	// 	// NOTE: a bright gray makes the shadows of the windows look weird.
	// 	// We use a bit of transparency so that if the user switches on the
	// 	// `transparent()` option they get immediate results.
	// 	egui::Color32::from_rgba_unmultiplied(12, 12, 12, 180).into()
	// }

	// fn persist_native_window(&self) -> bool {
	// 	true
	// }

	// fn persist_egui_memory(&self) -> bool {
	// 	true
	// }
}



pub fn display_scroll_row(
	ui: &mut egui::Ui,
	desired_size: egui::Vec2,
	item: &mut ReturnedItem,
	config: &mut Config,
	store: &StorageContainer,
	removed_data_index: &mut Option<usize>,
	now: chrono::DateTime<chrono::Utc>
) {
	ui.allocate_ui_with_layout(desired_size, egui::Layout::right_to_left(), |ui| {
		ui.set_height(desired_size.y);

		ui.allocate_ui_with_layout(egui::vec2(20.0, ui.available_height()), egui::Layout::top_down(egui::Align::RIGHT), |ui| {
			if ui.add(egui::SelectableLabel::new(item.is_favorite, "⭐")).on_hover_text("Favorite").clicked() {
				item.is_favorite = !item.is_favorite;
				store.set_favorite(item.data_id, item.is_favorite).unwrap();
			}

			if ui.add(egui::Button::new("❌")).on_hover_text("Delete").clicked() {
				*removed_data_index = Some(item.data_id);
			}
		});

		match &item.value {
			ReturnedItemType::Text(text_data) => {
				ui.allocate_ui_with_layout(ui.available_size(), egui::Layout::top_down(egui::Align::LEFT), |ui| {
					ui.set_clip_rect(ui.available_rect_before_wrap());

					let clicked_label = ui.add(
						egui::Label::new(
							egui::RichText::new(text_data.replace('\n', " ").replace('\t', " "))
								.color(egui::Rgba::from_rgb(1.0, 1.0, 1.0))
						)
						.wrap(false)
						.sense(egui::Sense::click())
					).on_hover_text(text_data.as_str()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked();

					if clicked_label {
						ui.output().copied_text = text_data.clone();
					}

					ui.add(egui::Label::new(egui::RichText::new(item_time_ago(item.timestamp, now))).wrap(false))
						.on_hover_text(item.timestamp.with_timezone(&chrono::offset::Local).format(&config.app.timedate_format).to_string());
				});
			}

			&ReturnedItemType::ThumbTextureId(texture_id) => {
				ui.with_layout(egui::Layout::from_main_dir_and_cross_align(egui::Direction::LeftToRight, egui::Align::BOTTOM), |ui| {
					if ui.add(egui::ImageButton::new(TextureId::User(texture_id), [32.0, 32.0]).frame(false)).clicked() {
						if let Err(e) = clipboard_common::set_clipboard_image(item.data_id, store) {
							error!(target: "clipboard_gui", "Copy Image Error: {:?}", e);
						}
					}

					ui.add(egui::Label::new(egui::RichText::new(item_time_ago(item.timestamp, now))).wrap(false))
						.on_hover_text(item.timestamp.with_timezone(&chrono::offset::Local).format(&config.app.timedate_format).to_string());
				});
			}

			ReturnedItemType::Thumb(_) => {} // Should never be ran.
		}
	});
}


pub fn prepend_new_items_into_existing(existing_items: &mut Vec<ReturnedItem>, mut new_items: Vec<ReturnedItem>, frame: &epi::Frame) {
	new_items.iter_mut().for_each(|item| if let ReturnedItemType::Thumb(thumb_data) = &item.value {
		if let Ok(img) = image::load_from_memory(thumb_data.as_slice()) {
			let rgba = img.to_rgba8();

			let texture_id = frame.alloc_texture(epi::Image::from_rgba_unmultiplied(
				[rgba.width() as usize, rgba.height() as usize],
				rgba.as_raw()
			));

			drop(img);
			item.value = ReturnedItemType::ThumbTextureId(if let TextureId::User(v) = texture_id { v } else { unreachable!("prepend_new_items_into_existing") });
		}
	});

	// Add NEW items before old items.
	let mut old_items = std::mem::replace(existing_items, new_items);

	if !old_items.is_empty() {
		existing_items.append(&mut old_items);
	}
}