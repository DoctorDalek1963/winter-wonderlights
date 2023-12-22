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

pub fn parse_command(input: &str) -> IResult<&str, Command> {
    alt((
        parse_help.map(|()| Command::Help),
        parse_show.map(|range| Command::Show(range)),
        parse_set.map(|(idx, point)| Command::Set(idx, point)),
        parse_save.map(|filename| Command::Save(filename)),
    ))(input)
}

fn parse_help(input: &str) -> IResult<&str, ()> {
    let (input, _) = alt((tag("help"), tag("?")))(input)?;
    Ok((input, ()))
}

fn parse_show(input: &str) -> IResult<&str, Option<RangeInclusive<usize>>> {
    let (input, _) = tag("show")(input)?;
    let (input, _) = multispace0(input)?;

    match parse_show_args(input) {
        Ok((input, args)) => Ok((input, Some(args))),
        Err(_) => Ok((input, None)),
    }
}

fn parse_show_args(input: &str) -> IResult<&str, RangeInclusive<usize>> {
    fn parse_one_idx(input: &str) -> IResult<&str, usize> {
        complete::u16.map(|idx| idx as usize).parse(input)
    }

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

fn parse_set(input: &str) -> IResult<&str, (usize, PointF)> {
    let (input, _) = tag("set")(input)?;
    let (input, _) = multispace1(input)?;

    let (input, idx) = complete::u16(input)?;
    let (input, _) = multispace1(input)?;
    let (input, point) = parse_pointf(input)?;

    Ok((input, (idx.into(), point)))
}

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

fn parse_save(input: &str) -> IResult<&str, Option<&str>> {
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

    let (input, _) = tag("save")(input)?;
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

        assert_eq!(parse_command("save"), Ok(("", Command::Save(None))));
        assert_eq!(
            parse_command("save \"file name\""),
            Ok(("", Command::Save(Some("file name"))))
        );
        assert_eq!(
            parse_command("save 'file name'"),
            Ok(("", Command::Save(Some("file name"))))
        );
        assert_eq!(
            parse_command("save filename"),
            Ok(("", Command::Save(Some("filename"))))
        );
        assert_eq!(
            parse_command("save /path/to/filename"),
            Ok(("", Command::Save(Some("/path/to/filename"))))
        );
    }
}
