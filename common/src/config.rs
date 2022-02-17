use anyhow::Result;
use serde::{Serialize, Deserialize};


static CONFIG_PATH: &str = "config.toml";

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
	pub app: ConfigApp,
	pub stores: Stores,
	// pub auth
}

impl Config {
	pub fn load() -> Result<Self> {
		if let Ok(value) = std::fs::read(CONFIG_PATH) {
			Ok(toml::from_slice(&value)?)
		} else {
			Ok(Self::default())
		}
	}

	pub fn save(&self) -> Result<()> {
		let value = toml::to_string_pretty(self)?;

		std::fs::write(CONFIG_PATH, value)?;

		Ok(())
	}

	pub fn reload(&mut self) -> Result<()> {
		*self = Self::load()?;
		Ok(())
	}
}


#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ConfigApp {
	pub logging: bool,
	pub query_return_limit: usize,
	pub timedate_format: String,
	pub hide_when_deleted: bool,
	pub always_on_top: bool
}

impl Default for ConfigApp {
    fn default() -> Self {
        Self {
			logging: true,
			query_return_limit: 25,
			timedate_format: String::from("%b %e %Y, %l:%M:%S %p"),
			hide_when_deleted: false,
			always_on_top: false
		}
    }
}


#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Stores {
	pub text: StoreTypeText,
	pub image: StoreTypeImage,
	pub file: StoreTypeFile,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct StoreTypeText {
	pub enabled: bool,
	pub max_size: usize
}

impl Default for StoreTypeText {
	fn default() -> Self {
		Self {
			enabled: true,
			max_size: 5120
		}
	}
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct StoreTypeImage {
	pub enabled: bool,
	pub max_size: usize
}

impl Default for StoreTypeImage {
	fn default() -> Self {
		Self {
			enabled: false,
			max_size: 5120
		}
	}
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct StoreTypeFile {
	pub enabled: bool,
	pub max_size: usize
}

impl Default for StoreTypeFile {
	fn default() -> Self {
		Self {
			enabled: false,
			max_size: 5120
		}
	}
}