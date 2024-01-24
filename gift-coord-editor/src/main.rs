//! This crate provides a simple CLI to edit GIFT coordinates.

#![feature(lint_reasons)]
#![allow(
    clippy::expect_used,
    reason = "this crate doesn't use tracing, so we can't use expect_or_log"
)]

mod driver;
mod parse;

use clap::Parser;
use color_eyre::{eyre::Context, Result};
use driver::EditorDriver;
use parse::parse_command;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{fs, ops::RangeInclusive};
use termion::{color, style};
use ww_gift_coords::{GIFTCoords, PointF};

/// Generate the help text for the editor based on whether or not a driver has been enabled.
fn get_help_text() -> String {
    let mut text = String::from(
        r#"Use the following commands to edit the GIFT coordinates in the provided file:

    help  -  Show this help text
    ?     -  Same as help

    show        -  Show all the coordinates in the file
    show 10     -  Show the light at index 10
    show 10:20  -  Show the lights between index 10 and index 20 (inclusive)

    set 10 (0.234, -0.567, 2.345)  -  Set the light at index 10 to have the given coordinate

"#,
    );
    if cfg!(feature = "_driver") {
        text.push_str("    light 10  -  Light up the light at index 10\n\n");
    }
    text.push_str(
        r#"    save             -  Save the new coordinates back to the original file
    save "filename"  -  Save the new coordinates to a given file

    saveraw             -  Save the new coordinates back to the original file without renormalizing them
    saveraw "filename"  -  Save the new coordinates to a given file without renormalizing them

When saving the coordinates to a file, they get re-normalized to fit
within the GIFT coordinate format.

GIFT coordinates are (x, y, z) with z being vertical, going positively
upwards with 0 at the lowest light. x and y are both between -1 and 1
with (0, 0, z) being a point on the trunk of the tree. Positive x is in
the direction of "east" relative to the tree, and positive y is in the
direction of "north" relative to the tree.

Use Ctrl+D with an empty prompt to quit."#,
    );
    text
}

/// Edit GIFT coordinates with a simple CLI.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// The filename of the GIFT coordinates.
    filename: String,
}

/// A user-given command.
#[derive(Clone, Debug, PartialEq)]
enum Command<'save_filename> {
    /// Show available commands.
    Help,

    /// Show the coordinate of all lights, one light, or a range of lights.
    Show(Option<RangeInclusive<usize>>),

    /// Set the coordinate of one light.
    Set(usize, PointF),

    /// Light up the light at the specified index.
    #[cfg(feature = "_driver")]
    Light(usize),

    /// Save the coordinates back to the original file or a new one. The bool represents if the
    /// coords should be raw or not.
    Save(Option<&'save_filename str>, bool),
}

impl Command<'_> {
    /// Execute this command.
    #[cfg_attr(
        not(feature = "_driver"),
        allow(
            unused_variables,
            reason = "the `driver` argument is a ZST when we don't have a proper driver"
        )
    )]
    fn execute(self, coords: &mut [PointF], original_filename: &str, driver: &mut EditorDriver) {
        match self {
            Command::Help => println!("{}", get_help_text()),
            Command::Show(range) => match range {
                Some(range) => {
                    for idx in range {
                        let (x, y, z) = coords[idx];
                        println!("{idx}: ({x}, {y}, {z})");
                    }
                }
                None => {
                    for (idx, (x, y, z)) in coords.iter().enumerate() {
                        println!("{idx}: ({x}, {y}, {z})");
                    }
                }
            },
            Command::Set(idx, point) => {
                coords[idx] = point;
                println!("Set light {idx} to ({}, {}, {})", point.0, point.1, point.2);
            }
            #[cfg(feature = "_driver")]
            Command::Light(idx) => driver.enable_one_light(idx, coords.len()),
            Command::Save(new_filename, raw) => {
                let mut filename = new_filename.unwrap_or(original_filename).to_string();
                if raw {
                    if !filename.ends_with(".raw") {
                        filename.push_str(".raw");
                    }
                    let data =
                        bincode::serialize(coords).expect("Should be able to serialize raw coords");
                    fs::write(&filename, data).expect("Should be able to write raw coords to file");
                    println!("Saved raw coords to {filename:?}");
                } else {
                    if filename.ends_with(".raw") {
                        filename = filename
                            .strip_suffix(".raw")
                            .expect("We already checked that the filename ends with `.raw`")
                            .to_string();
                    }
                    GIFTCoords::from_unnormalized_coords(coords)
                        .expect("Should be able to build GIFT coords")
                        .save_to_file(&filename)
                        .expect("Should be able to save GIFT coords to file");
                    println!("Saved coords to {filename:?}");
                }
            }
        };
        println!();
    }
}

fn main() -> Result<()> {
    let filename = Args::parse().filename;
    let backup_filename = format!("{filename}.backup");
    let gift_coords = GIFTCoords::from_file(&filename)?;
    gift_coords
        .save_to_file(&backup_filename)
        .wrap_err("Failed to save backup coordinates")?;

    let mut coords = gift_coords.coords().clone();
    let mut prompt = DefaultEditor::new()?;

    // Safety: This method is only called once, so it's fine
    let mut driver = unsafe { self::driver::EditorDriver::init() };

    let prompt_string = format!(
        "{}{}==> {}",
        style::Bold,
        color::Fg(color::LightCyan),
        style::Reset
    );

    println!("{}\n", get_help_text());

    loop {
        match prompt.readline(&prompt_string) {
            Ok(input) => {
                prompt.add_history_entry(&input)?;
                match parse_command(&input) {
                    Ok(("", command)) => {
                        command.execute(&mut coords, &filename, &mut driver);
                    }
                    Ok((extra, _)) => eprintln!(
                        "{}{}ERROR:{} Trailing input: `{extra}`",
                        style::Bold,
                        color::Fg(color::Red),
                        style::Reset
                    ),
                    Err(error) => eprintln!(
                        "{}{}ERROR:{} Failed to parse input: `{error:?}`",
                        style::Bold,
                        color::Fg(color::Red),
                        style::Reset
                    ),
                };
            }
            Err(ReadlineError::Interrupted) => (),
            Err(ReadlineError::Eof) => return Ok(()),
            Err(ReadlineError::Io(e)) => return Err(e).wrap_err("IO error when reading prompt")?,
            Err(error) => panic!("Unknown error: `{error:?}`"),
        };
    }
}
