use crate::effects::EffectList;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct VirtualTreeConfig {
    /// The amount of time to pause between loops of the effect.
    pub loop_pause_time: u64,

    /// The item in the effects enum that's currently being run.
    pub effect: Option<EffectList>,
}

impl VirtualTreeConfig {
    /// Create a default config for the virtual tree.
    pub const fn default() -> Self {
        Self {
            loop_pause_time: 1500,
            effect: None,
        }
    }

    /// Return the filename to store the config in.
    const fn config_filename() -> &'static str {
        "config/drivers/virtual_tree.ron"
    }

    /// Load the config from the file, using the default if the file is unavailable.
    pub fn from_file() -> Self {
        let _ = fs::DirBuilder::new()
            .recursive(true)
            .create("config/drivers");

        let write_and_return_default = || -> Self {
            let default = Self::default();
            default.save_to_file();
            default
        };

        let Ok(text) = fs::read_to_string(Self::config_filename()) else {
            return write_and_return_default();
        };

        ron::from_str(&text).unwrap_or_else(|_| write_and_return_default())
    }

    /// Save the config to the file.
    pub fn save_to_file(&self) {
        let _ = fs::write(
            Self::config_filename(),
            ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default().struct_names(true))
                .expect("The virtual tree config should be serializable"),
        );
    }
}
