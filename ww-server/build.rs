//! This is the build script for `ww-server`. It checks that exactly one driver feature is enabled.

#![feature(iter_intersperse)]

/// Names of driver features with leading `driver-` removed, dashes replaced with underscores, and
/// everything in ALL CAPS.
const DRIVER_NAMES: &[&str] = &["DEBUG", "VIRTUAL_TREE"];

/// Convert the given driver name into the name of the corresponding feature.
fn driver_name_to_feature_name(driver_name: &&str) -> String {
    format!("driver-{}", driver_name.to_lowercase().replace('_', "-"))
}

fn main() -> Result<(), String> {
    let names: Vec<_> = DRIVER_NAMES
        .iter()
        .filter(|&name| std::env::var(format!("CARGO_FEATURE_DRIVER_{name}")).is_ok())
        .collect();

    if names.len() == 1 {
        Ok(())
    } else {
        let names: String = names
            .into_iter()
            .map(driver_name_to_feature_name)
            .intersperse(String::from(", "))
            .collect();

        let options: String = DRIVER_NAMES
            .iter()
            .map(driver_name_to_feature_name)
            .intersperse(String::from(", "))
            .collect();

        Err(format!(
            "ww-server must be built with EXACTLY ONE driver feature enabled. Got features: [{names}]. Options are: [{options}]",
        ))
    }
}
