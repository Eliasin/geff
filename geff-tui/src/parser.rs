use nom::{
    branch::alt,
    character::complete::{char, multispace1, none_of, u32},
    combinator::{eof, map},
    multi::many1,
    sequence::{delimited, tuple},
    IResult,
};

#[derive(Debug, Clone)]
pub enum Command {
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
}

fn goal_name(input: &str) -> IResult<&str, String> {
    map(
        delimited(char('\"'), many1(none_of("\"")), char('\"')),
        |r| r.into_iter().collect(),
    )(input)
}

fn create_command(input: &str) -> IResult<&str, Command> {
    map(
        tuple((char('c'), multispace1, goal_name, multispace1, u32, eof)),
        |(_, _, name, _, effort_to_complete, _)| Command::Create {
            name,
            effort_to_complete,
        },
    )(input)
}

fn delete_command(input: &str) -> IResult<&str, Command> {
    map(char('d'), |_| Command::Delete)(input)
}

fn refine_command(input: &str) -> IResult<&str, Command> {
    map(
        tuple((
            char('r'),
            multispace1,
            goal_name,
            multispace1,
            u32,
            multispace1,
            u32,
        )),
        |(_, _, child_name, _, child_effort_to_complete, _, parent_effort_removed)| {
            Command::Refine {
                child_name,
                child_effort_to_complete,
                parent_effort_removed,
            }
        },
    )(input)
}

pub fn command(input: &str) -> IResult<&str, Command> {
    alt((create_command, delete_command, refine_command))(input)
}
