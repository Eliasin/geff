use std::collections::HashSet;

use geff_core::goal::{GoalEvent, GoalId, PopulatedGoal};
use geff_core::profile::Profile;
use geff_core::request::GoalRequest;
use geff_core::request::GoalRequestHandler;
use geff_core::{DateTime, Utc};
use geff_util::{get_selected_goal, get_selected_goal_id, Cursor, CursorAction, LoadError};
use iced::subscription::events_with;
use iced::widget::{column, container, row, scrollable, text};
use iced::{
    alignment, keyboard, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};
use pest::Parser;
use pest_derive::Parser;

#[derive(Debug, Default, Clone)]
struct PersistentState {
    profile: Profile,
    goal_event_history: Vec<GoalEvent>,
}

#[derive(Debug, Clone)]
struct AppState {
    persistent_state: PersistentState,
    current_datetime: DateTime<Utc>,
    populated_goals: Vec<PopulatedGoal>,
    cursor: Cursor,
    commandline: Option<String>,
    error_log: ErrorLog,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            persistent_state: Default::default(),
            current_datetime: Utc::now(),
            populated_goals: vec![],
            cursor: Default::default(),
            commandline: None,
            error_log: ErrorLog::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ErrorLog(Vec<String>);

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<PersistentState, LoadError>),
    TickTime(DateTime<Utc>),
    KeyboardEvent(keyboard::Event),
}

#[derive(Parser)]
#[grammar = "commandline_grammar.pest"]
struct CommandlineParser;

fn parse_commandline(
    commandline: &str,
    cursor: &Cursor,
    goals: &[PopulatedGoal],
) -> Result<Option<GoalRequest>, Box<dyn std::error::Error>> {
    let command = CommandlineParser::parse(Rule::command, commandline)?
        .next()
        .expect("unwrapping first rule to never fail");

    match command.as_rule() {
        Rule::create => {
            let mut pairs = command.into_inner();

            let goal_name = pairs.next().unwrap().as_span().as_str().to_string();
            let goal_effort = str::parse::<u32>(pairs.next().unwrap().as_span().as_str())
                .expect("pest grammar to guarantee this is a number");

            Ok(Some(GoalRequest::Add {
                name: goal_name,
                effort_to_complete: goal_effort,
            }))
        }
        Rule::delete => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => Some(GoalRequest::Focus(get_selected_goal_id(
                    selected_goal,
                    goals,
                )?)),
                None => None,
            }),
        },
        Rule::focus => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => Some(GoalRequest::Focus(get_selected_goal_id(
                    selected_goal,
                    goals,
                )?)),
                None => None,
            }),
        },
        Rule::unfocus => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => Some(GoalRequest::Unfocus(get_selected_goal_id(
                    selected_goal,
                    goals,
                )?)),
                None => None,
            }),
        },
        Rule::focus_single => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => Some(GoalRequest::FocusSingle(get_selected_goal_id(
                    selected_goal,
                    goals,
                )?)),
                None => None,
            }),
        },
        Rule::unfocus_single => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => Some(GoalRequest::UnfocusSingle(get_selected_goal_id(
                    selected_goal,
                    goals,
                )?)),
                None => None,
            }),
        },
        Rule::refine => match cursor {
            Cursor::SelectedGoal(selected_goal) => Ok(match selected_goal {
                Some(selected_goal) => {
                    let mut pairs = command.into_inner();

                    let child_name = pairs.next().unwrap().as_span().as_str().to_string();
                    let parent_effort_removed =
                        str::parse::<u32>(pairs.next().unwrap().as_span().as_str())
                            .expect("pest grammar to guarantee this is a number");
                    let child_effort_to_complete =
                        str::parse::<u32>(pairs.next().unwrap().as_span().as_str())
                            .expect("pest grammar to guarantee this is a number");

                    let selected_goal_data = get_selected_goal(selected_goal, goals)?;

                    Some(GoalRequest::Refine {
                        parent_goal_id: selected_goal_data.id,
                        parent_effort_removed,
                        child_name,
                        child_effort_to_complete,
                    })
                }
                None => None,
            }),
        },
        _ => unreachable!(),
    }
}

#[derive(Debug)]
enum App {
    Loading(ErrorLog),
    Loaded(AppState),
}

impl Application for App {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            App::Loading(ErrorLog(vec![])),
            Command::perform(
                async {
                    geff_util::PersistentState::<()>::load()
                        .await
                        .map(|loaded_state| {
                            let (profile, goal_event_history, _) = loaded_state.into();

                            PersistentState {
                                profile,
                                goal_event_history,
                            }
                        })
                },
                Message::Loaded,
            ),
        )
    }

    fn title(&self) -> String {
        "Geff".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match self {
            App::Loading(error_log) => match message {
                Message::Loaded(state) => match state {
                    Ok(state) => {
                        *self = App::Loaded(AppState {
                            persistent_state: state,
                            ..Default::default()
                        });
                        Command::none()
                    }
                    Err(load_error) => {
                        error_log.0.push(load_error.to_string());
                        error_log.0.push("Loading default state...".to_string());

                        *self = App::Loaded(AppState {
                            error_log: error_log.clone(),
                            ..Default::default()
                        });

                        Command::none()
                    }
                },
                _ => Command::none(),
            },
            App::Loaded(state) => match message {
                Message::TickTime(datetime) => {
                    state.current_datetime = datetime;

                    Command::none()
                }

                Message::KeyboardEvent(event) => {
                    use keyboard::KeyCode;

                    match event {
                        keyboard::Event::KeyPressed {
                            key_code,
                            modifiers: _,
                        } => {
                            if let Some(commandline) = state.commandline.as_mut() {
                                if key_code == KeyCode::Backspace {
                                    if commandline.len() == 1 {
                                        state.commandline = None;
                                    } else {
                                        commandline.pop();
                                    }
                                } else if key_code == KeyCode::Escape {
                                    state.commandline = None;
                                } else if key_code == KeyCode::Enter {
                                    match parse_commandline(
                                        commandline,
                                        &state.cursor,
                                        &state.populated_goals,
                                    ) {
                                        Ok(request) => {
                                            if let Some(request) = request {
                                                let events = state
                                                    .persistent_state
                                                    .profile
                                                    .with_datetime(state.current_datetime)
                                                    .handle_request(request);

                                                state
                                                    .persistent_state
                                                    .goal_event_history
                                                    .extend(events);

                                                state.populated_goals =
                                                    state.persistent_state.profile.populate_goals();
                                            }
                                        }
                                        Err(e) => state.error_log.0.push(e.to_string()),
                                    }

                                    state.commandline = None;
                                }
                            } else {
                                let cursor_action = match key_code {
                                    KeyCode::J => Some(CursorAction::Down),
                                    KeyCode::K => Some(CursorAction::Up),
                                    KeyCode::H => Some(CursorAction::Out),
                                    KeyCode::L => Some(CursorAction::In),
                                    _ => None,
                                };

                                if let Some(cursor_action) = cursor_action {
                                    if let Err(e) = state
                                        .cursor
                                        .handle_action(cursor_action, &state.populated_goals)
                                    {
                                        state.error_log.0.push(e.to_string());
                                    }
                                }
                            }
                        }
                        keyboard::Event::CharacterReceived(c) => {
                            if let Some(commandline) = &mut state.commandline {
                                if !c.is_ascii_control() {
                                    commandline.push(c);
                                }
                            } else if c == ':' {
                                state.commandline = Some("".to_string());
                            }
                        }
                        _ => {}
                    };

                    Command::none()
                }
                Message::Loaded(_) => Command::none(),
            },
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        match self {
            App::Loading(error_log) => column![loading_message(), error_log.view()].into(),
            App::Loaded(state) => column![main_ui(state), state.error_log.view()].into(),
        }
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        if let App::Loaded(_) = self {
            Subscription::batch(vec![
                events_with(|event, _| match event {
                    Event::Keyboard(e) => Some(Message::KeyboardEvent(e)),
                    _ => None,
                }),
                iced::time::every(std::time::Duration::from_millis(500))
                    .map(|_| Message::TickTime(Utc::now())),
            ])
        } else {
            Subscription::none()
        }
    }
}

fn loading_message<'a>() -> Element<'a, Message> {
    container(
        text("Loading...")
            .horizontal_alignment(alignment::Horizontal::Center)
            .size(50),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_y()
    .into()
}

fn commandline(commandline: &Option<String>) -> Element<'_, Message> {
    container(
        match commandline {
            Some(command) => text(format!(":{command}")),
            None => text(""),
        }
        .vertical_alignment(alignment::Vertical::Bottom),
    )
    .padding([5, 10])
    .height(Length::Fill)
    .align_y(alignment::Vertical::Bottom)
    .into()
}

fn goal<'a>(
    goal: &'a PopulatedGoal,
    selected_goal_id: Option<GoalId>,
    focused_goals: &HashSet<GoalId>,
) -> Element<'a, Message> {
    let mut prefix = String::new();

    if selected_goal_id
        .map(|selected_goal_id| selected_goal_id == goal.id)
        .unwrap_or(false)
    {
        prefix.push('*');
    }

    if focused_goals.contains(&goal.id) {
        prefix.push('F');
    }

    let goal_text = text(format!(
        "{} {} ({}/{})",
        prefix, goal.name, goal.effort_to_date, goal.effort_to_complete
    ));

    row![
        goal_text,
        goals(&goal.children, selected_goal_id, focused_goals)
    ]
    .into()
}

fn goals<'a>(
    goals: &'a [PopulatedGoal],
    selected_goal_id: Option<GoalId>,
    focused_goals: &HashSet<GoalId>,
) -> Element<'a, Message> {
    column(
        goals
            .iter()
            .map(|g| goal(g, selected_goal_id, focused_goals))
            .collect(),
    )
    .padding([30, 30])
    .into()
}

fn main_ui(state: &AppState) -> Element<'_, Message> {
    let title = text("Geff")
        .width(Length::Fill)
        .size(100)
        .style(Color::from([0.5, 0.5, 0.5]))
        .horizontal_alignment(alignment::Horizontal::Center)
        .vertical_alignment(alignment::Vertical::Top);

    let selected_cursor_id = if let Cursor::SelectedGoal(Some(selected_goal)) = &state.cursor {
        get_selected_goal_id(selected_goal, &state.populated_goals).ok()
    } else {
        None
    };

    let middle_element: Element<'_, Message> = if state.populated_goals.is_empty() {
        container(
            text(
                "help\n\
             : to start command\n\
             esc to cancel a command\n\
             h/j/k/l to move left/down/up/right\n\n\
             commands:\n\
             c \"<name>\" <effort_to_complete>    to create a new goal\n\
             d                                    to delete the selected goal\n\
             f                                    to focus the selected goal",
            )
            .vertical_alignment(alignment::Vertical::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    } else {
        goals(
            &state.populated_goals,
            selected_cursor_id,
            state.persistent_state.profile.focused_goals(),
        )
    };

    container(column![
        title,
        middle_element,
        commandline(&state.commandline)
    ])
    .width(Length::Fill)
    .height(Length::FillPortion(30))
    .into()
}

impl ErrorLog {
    pub fn view(&self) -> Element<'_, Message> {
        container(scrollable(
            column(self.0.iter().map(|message| text(message).into()).collect()).width(Length::Fill),
        ))
        .width(Length::Fill)
        .align_y(alignment::Vertical::Bottom)
        .padding([20, 20])
        .height(Length::FillPortion(10))
        .into()
    }
}

fn main() -> iced::Result {
    use iced::window;

    App::run(Settings {
        window: window::Settings {
            size: (800, 800),
            ..window::Settings::default()
        },
        ..Settings::default()
    })
}
