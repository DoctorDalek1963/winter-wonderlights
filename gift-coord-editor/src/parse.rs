//! This module handles parsing user input.

use crate::Command;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_until1, take_while1},
    character::complete::{self, multispace0, multispace1},
    number::complete::float,
    IResult, Parser,
};
use std::ops::RangeInclusive;
use ww_gift_coords::PointF;

/// Parse a command from the user's input.
#[allow(clippy::redundant_closure, reason = "style preference")]
pub fn parse_command(input: &str) -> IResult<&str, Command> {
    alt((
        parse_help.map(|()| Command::Help),
        parse_show.map(|range| Command::Show(range)),
        parse_set.map(|(idx, point)| Command::Set(idx, point)),
        #[cfg(feature = "_driver")]
        parse_light.map(|idx| Command::Light(idx)),
        (|input| parse_save_or_saveraw(input, "saveraw"))
            .map(|filename| Command::Save(filename, true)),
        (|input| parse_save_or_saveraw(input, "save"))
            .map(|filename| Command::Save(filename, false)),
    ))(input)
}

/// Parse the `help` or `?` command. See [`Command::Help`].
fn parse_help(input: &str) -> IResult<&str, ()> {
    let (input, _) = alt((tag("help"), tag("?")))(input)?;
    Ok((input, ()))
}

/// Parse a `show` command. See [`Command::Show`].
fn parse_show(input: &str) -> IResult<&str, Option<RangeInclusive<usize>>> {
    let (input, _) = tag("show")(input)?;
    let (input, _) = multispace0(input)?;

    match parse_show_args(input) {
        Ok((input, args)) => Ok((input, Some(args))),
        Err(_) => Ok((input, None)),
    }
}

/// Parse the arguments for a `show` command.
fn parse_show_args(input: &str) -> IResult<&str, RangeInclusive<usize>> {
    /// Parse an index for the lights.
    fn parse_one_idx(input: &str) -> IResult<&str, usize> {
        complete::u16.map(|idx| idx as usize).parse(input)
    }

    /// Parse a pair of indices for the lights.
    fn parse_pair_of_idx(input: &str) -> IResult<&str, (usize, usize)> {
        let (input, start) = parse_one_idx(input)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = tag(":")(input)?;
        let (input, _) = multispace0(input)?;
        let (input, end) = parse_one_idx(input)?;
        Ok((input, (start, end)))
    }

    alt((
        parse_pair_of_idx.map(|(start, end)| start..=end),
        parse_one_idx.map(|idx| idx..=idx),
    ))(input)
}

/// Parse a `set` command. See [`Command::Set`].
fn parse_set(input: &str) -> IResult<&str, (usize, PointF)> {
    let (input, _) = tag("set")(input)?;
    let (input, _) = multispace1(input)?;

    let (input, idx) = complete::u16(input)?;
    let (input, _) = multispace1(input)?;
    let (input, point) = parse_pointf(input)?;

    Ok((input, (idx.into(), point)))
}

/// Parse a [`PointF`].
fn parse_pointf(input: &str) -> IResult<&str, PointF> {
    let (input, _) = tag("(")(input)?;
    let (input, _) = multispace0(input)?;

    let (input, x) = float(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = tag(",")(input)?;
    let (input, _) = multispace0(input)?;

    let (input, y) = float(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = tag(",")(input)?;
    let (input, _) = multispace0(input)?;

    let (input, z) = float(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = tag(")")(input)?;

    Ok((input, (x, y, z)))
}

/// Parse a `light` command. See [`Command::Light`].
#[cfg(feature = "_driver")]
fn parse_light(input: &str) -> IResult<&str, usize> {
    let (input, _) = tag("light")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, idx) = complete::u16(input)?;
    Ok((input, idx as usize))
}

/// Parse a `save` or `saveraw` command. See [`Command::Save`].
fn parse_save_or_saveraw<'input>(
    input: &'input str,
    command: &'static str,
) -> IResult<&'input str, Option<&'input str>> {
    /// Parse a filename. This could be a simple filename with no escapes, or a filename in double
    /// or single quotes.
    fn parse_filename(input: &str) -> IResult<&str, &str> {
        let (input, _) = multispace1(input)?;
        alt((
            tag("\"")
                .and(take_until1("\""))
                .and(tag("\""))
                .map(|((_, filename), _)| filename),
            tag("'")
                .and(take_until1("'"))
                .and(tag("'"))
                .map(|((_, filename), _)| filename),
            take_while1(|c: char| !c.is_whitespace()),
        ))(input)
    }

    let (input, _) = tag(command)(input)?;
    match parse_filename(input) {
        Ok((input, filename)) => Ok((input, Some(filename))),
        Err(_) => Ok((input, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_test() {
        assert_eq!(parse_command("help"), Ok(("", Command::Help)));
        assert_eq!(parse_command("?"), Ok(("", Command::Help)));

        assert_eq!(parse_command("show"), Ok(("", Command::Show(None))));
        assert_eq!(
            parse_command("show 10"),
            Ok(("", Command::Show(Some(10..=10))))
        );
        assert_eq!(
            parse_command("show 8 : 25"),
            Ok(("", Command::Show(Some(8..=25))))
        );
        assert_eq!(
            parse_command("show 8:25"),
            Ok(("", Command::Show(Some(8..=25))))
        );
        assert_eq!(
            parse_command("show 8:   25"),
            Ok(("", Command::Show(Some(8..=25))))
        );

        assert_eq!(
            parse_command("set 10 (0.567, -0.345, 1.234)"),
            Ok(("", Command::Set(10, (0.567, -0.345, 1.234))))
        );
        assert_eq!(
            parse_command("set 0 (-0.567,-0.345,1.234)"),
            Ok(("", Command::Set(0, (-0.567, -0.345, 1.234))))
        );

        assert_eq!(parse_command("save"), Ok(("", Command::Save(None, false))));
        assert_eq!(
            parse_command("save \"file name\""),
            Ok(("", Command::Save(Some("file name"), false)))
        );
        assert_eq!(
            parse_command("save 'file name'"),
            Ok(("", Command::Save(Some("file name"), false)))
        );
        assert_eq!(
            parse_command("save filename"),
            Ok(("", Command::Save(Some("filename"), false)))
        );
        assert_eq!(
            parse_command("save /path/to/filename"),
            Ok(("", Command::Save(Some("/path/to/filename"), false)))
        );

        assert_eq!(
            parse_command("saveraw"),
            Ok(("", Command::Save(None, true)))
        );
        assert_eq!(
            parse_command("saveraw \"file name\""),
            Ok(("", Command::Save(Some("file name"), true)))
        );
        assert_eq!(
            parse_command("saveraw 'file name'"),
            Ok(("", Command::Save(Some("file name"), true)))
        );
        assert_eq!(
            parse_command("saveraw filename"),
            Ok(("", Command::Save(Some("filename"), true)))
        );
        assert_eq!(
            parse_command("saveraw /path/to/filename"),
            Ok(("", Command::Save(Some("/path/to/filename"), true)))
        );
    }
}
