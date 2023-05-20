#![feature(iter_intersperse)]

const DRIVER_NAMES: &'static [&'static str] = &["DEBUG"];

fn driver_name_to_feature_name(driver_name: &&str) -> String {
    format!("driver-{}", driver_name.to_lowercase().replace("_", "-"))
}

fn main() -> Result<(), String> {
    let names: Vec<_> = DRIVER_NAMES
        .into_iter()
        .filter(|&name| std::env::var(format!("CARGO_FEATURE_DRIVER_{name}")).is_ok())
        .collect();

    match names.len() {
        1 => Ok(()),
        _ => {
            let names: String = names
                .into_iter()
                .map(driver_name_to_feature_name)
                .intersperse(String::from(", "))
                .collect();

            let options: String = DRIVER_NAMES
                .into_iter()
                .map(driver_name_to_feature_name)
                .intersperse(String::from(", "))
                .collect();

            Err(format!(
                "ww-server must be built with EXACTLY ONE driver feature enabled. Got features: [{names}]. Options are: [{options}]",
            ))
        }
    }
}
