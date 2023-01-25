use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, char, multispace1, none_of, one_of, u32},
    combinator::{eof, map},
    multi::{count, many1},
    sequence::{delimited, tuple},
    IResult,
};

use crate::app::{CommandlineDisplayCommand, DisplayCommand};

#[derive(Debug, Clone)]
pub enum GoalCommand {
    Create {
        name: String,
        effort_to_complete: u32,
    },
    Delete,
    Refine {
        child_name: String,
        child_effort_to_complete: u32,
        parent_effort_removed: u32,
    },
    AddEffort {
        effort: u32,
    },
    RemoveEffort {
        effort: u32,
    },
}

fn quoted_string(input: &str) -> IResult<&str, String> {
    map(
        delimited(char('\"'), many1(none_of("\"")), char('\"')),
        |r| r.into_iter().collect(),
    )(input)
}

fn create_command(input: &str) -> IResult<&str, GoalCommand> {
    map(
        tuple((char('c'), multispace1, quoted_string, multispace1, u32, eof)),
        |(_, _, name, _, effort_to_complete, _)| GoalCommand::Create {
            name,
            effort_to_complete,
        },
    )(input)
}

fn delete_command(input: &str) -> IResult<&str, GoalCommand> {
    map(char('d'), |_| GoalCommand::Delete)(input)
}

fn add_effort_command(input: &str) -> IResult<&str, GoalCommand> {
    map(
        tuple((char('e'), multispace1, u32, eof)),
        |(_, _, effort, _)| GoalCommand::AddEffort { effort },
    )(input)
}

fn remove_effort_command(input: &str) -> IResult<&str, GoalCommand> {
    map(
        tuple((tag("re"), multispace1, u32, eof)),
        |(_, _, effort, _)| GoalCommand::RemoveEffort { effort },
    )(input)
}

fn refine_command(input: &str) -> IResult<&str, GoalCommand> {
    map(
        tuple((
            char('r'),
            multispace1,
            quoted_string,
            multispace1,
            u32,
            multispace1,
            u32,
        )),
        |(_, _, child_name, _, child_effort_to_complete, _, parent_effort_removed)| {
            GoalCommand::Refine {
                child_name,
                child_effort_to_complete,
                parent_effort_removed,
            }
        },
    )(input)
}

fn goal_command(input: &str) -> IResult<&str, GoalCommand> {
    alt((
        create_command,
        add_effort_command,
        remove_effort_command,
        delete_command,
        refine_command,
    ))(input)
}

fn change_font_size(input: &str) -> IResult<&str, DisplayCommand> {
    map(
        tuple((tag("dsf"), multispace1, u32, eof)),
        |(_, _, pixels, _)| CommandlineDisplayCommand::ChangeFontSize(pixels).into(),
    )(input)
}

fn hex_digit(input: &str) -> IResult<&str, char> {
    one_of("abcdefABCDEF1234567890")(input)
}

fn short_rgb(input: &str) -> IResult<&str, String> {
    map(
        tuple((char('#'), count(hex_digit, 3))),
        |(lead, mut c): (_, Vec<char>)| {
            c.insert(0, lead);
            c.into_iter().collect()
        },
    )(input)
}

fn long_rgb(input: &str) -> IResult<&str, String> {
    map(
        tuple((char('#'), count(hex_digit, 6))),
        |(lead, mut c): (_, Vec<char>)| {
            c.insert(0, lead);
            c.into_iter().collect()
        },
    )(input)
}

fn hex_code(input: &str) -> IResult<&str, String> {
    alt((short_rgb, long_rgb))(input)
}

fn color(input: &str) -> IResult<&str, String> {
    alt((hex_code, map(alphanumeric1, |s: &str| s.to_string())))(input)
}

fn change_background_color(input: &str) -> IResult<&str, DisplayCommand> {
    map(
        tuple((tag("dcb"), multispace1::<&str, _>, color)),
        |(_, _, color)| CommandlineDisplayCommand::ChangeBackgroundColor(color.to_string()).into(),
    )(input)
}

fn change_font_color(input: &str) -> IResult<&str, DisplayCommand> {
    map(
        tuple((tag("dcf"), multispace1::<&str, _>, color)),
        |(_, _, color)| CommandlineDisplayCommand::ChangeFontColor(color.to_string()).into(),
    )(input)
}

fn display_command(input: &str) -> IResult<&str, DisplayCommand> {
    alt((change_font_size, change_background_color, change_font_color))(input)
}

#[derive(Debug, Clone)]
pub enum ControlCommand {
    Quit,
}

fn quit_command(input: &str) -> IResult<&str, ControlCommand> {
    map(tuple((tag("q"), eof)), |_| ControlCommand::Quit)(input)
}

fn control_command(input: &str) -> IResult<&str, ControlCommand> {
    quit_command(input)
}

#[derive(Debug, Clone)]
pub enum Command {
    Display(DisplayCommand),
    Goal(GoalCommand),
    Control(ControlCommand),
}

pub fn command(input: &str) -> IResult<&str, Command> {
    map(
        tuple((
            char(':'),
            alt((
                map(display_command, |display_command| {
                    Command::Display(display_command)
                }),
                map(goal_command, |goal_command| Command::Goal(goal_command)),
                map(control_command, |control_command| {
                    Command::Control(control_command)
                }),
            )),
        )),
        |(_, command)| command,
    )(input)
}
