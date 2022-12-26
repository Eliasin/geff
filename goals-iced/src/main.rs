use std::collections::HashSet;
use std::{env::VarError, path::PathBuf};

use goals::goal::{GoalId, PopulatedGoal};
use goals::profile::goal_traversal::{traverse_populated_goal_children, GoalChildIndexPath};
use goals::request::GoalRequestHandler;
use goals::{goal::GoalEvent, profile::Profile, request::GoalRequest};
use goals::{DateTime, Utc};
use iced::subscription::events_with;
use iced::widget::{column, container, row, scrollable, text};
use iced::{
    alignment, keyboard, Application, Color, Command, Element, Event, Length, Settings,
    Subscription, Theme,
};
use pest::Parser;
use pest_derive::Parser;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct PersistentState {
    profile: Profile,
    goal_event_history: Vec<GoalEvent>,
}

#[derive(Debug, Clone)]
struct SelectedGoal {
    root_goal_index: usize,
    child_index_path: GoalChildIndexPath,
}

impl SelectedGoal {
    pub fn selected_index(&mut self) -> &mut usize {
        match self.child_index_path.last_mut() {
            Some(last_child_index) => last_child_index,
            None => &mut self.root_goal_index,
        }
    }

    pub fn pop_child(&mut self) -> Option<usize> {
        self.child_index_path.pop()
    }

    pub fn push_child(&mut self, index: usize) {
        self.child_index_path.push(index);
    }
}

#[derive(Debug, thiserror::Error)]
enum CursorError {
    #[error("root index of selected goal does not exist: {0:?}")]
    InvalidRootIndex(SelectedGoal),
    #[error("attempted to visit nonexistent child index {child_index} in goal {goal:?}")]
    InvalidGoalChild {
        goal: PopulatedGoal,
        child_index: usize,
    },
    #[error("error attempting to traverse to selected goal at {0:?}")]
    TraversalError(SelectedGoal),
}

#[derive(Debug, Clone)]
enum Cursor {
    SelectedGoal(Option<SelectedGoal>),
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::SelectedGoal(None)
    }
}

#[derive(Clone, Copy)]
enum CursorAction {
    Up,
    Down,
    In,
    Out,
}

fn selected_goal_siblings<'a>(
    selected_goal: &SelectedGoal,
    goals: &'a Vec<PopulatedGoal>,
) -> Result<&'a Vec<PopulatedGoal>, CursorError> {
    if let Some((_last, before_last)) = selected_goal.child_index_path.split_last() {
        let mut current = goals
            .get(selected_goal.root_goal_index)
            .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

        for index in before_last {
            current = current
                .children
                .get(*index)
                .ok_or(CursorError::InvalidGoalChild {
                    goal: current.clone(),
                    child_index: *index,
                })?;
        }

        Ok(&current.children)
    } else {
        Ok(goals)
    }
}

fn get_selected_goal<'a>(
    selected_goal: &SelectedGoal,
    goals: &'a [PopulatedGoal],
) -> Result<&'a PopulatedGoal, CursorError> {
    let mut current = goals
        .get(selected_goal.root_goal_index)
        .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

    for index in &selected_goal.child_index_path {
        current = current
            .children
            .get(*index)
            .ok_or(CursorError::InvalidGoalChild {
                goal: current.clone(),
                child_index: *index,
            })?;
    }

    Ok(current)
}

fn get_selected_goal_id(
    selected_goal: &SelectedGoal,
    goals: &[PopulatedGoal],
) -> Result<GoalId, CursorError> {
    let selected_goal_data = get_selected_goal(selected_goal, goals)?;
    Ok(selected_goal_data.id)
}

impl Cursor {
    pub fn handle_action(
        &mut self,
        action: CursorAction,
        goals: &Vec<PopulatedGoal>,
    ) -> Result<(), CursorError> {
        use CursorAction::*;

        match self {
            Cursor::SelectedGoal(selected_goal_index_path) => {
                match selected_goal_index_path.as_mut() {
                    Some(selected_goal) => match action {
                        Down => {
                            let sibling_goals = selected_goal_siblings(selected_goal, goals)?;

                            let selected_goal_index = selected_goal.selected_index();
                            if sibling_goals.len() > (*selected_goal_index) + 1 {
                                *selected_goal_index += 1;
                            }

                            Ok(())
                        }
                        Up => {
                            let selected_goal_index = selected_goal.selected_index();
                            if *selected_goal_index > 0 {
                                *selected_goal_index -= 1;
                            }
                            Ok(())
                        }
                        In => {
                            let root_goal = goals
                                .get(selected_goal.root_goal_index)
                                .ok_or(CursorError::InvalidRootIndex(selected_goal.clone()))?;

                            let selected_goal_data = traverse_populated_goal_children(
                                root_goal,
                                &selected_goal.child_index_path,
                            )
                            .ok_or(CursorError::TraversalError(selected_goal.clone()))?;

                            if !selected_goal_data.children.is_empty() {
                                selected_goal.push_child(0);
                            }

                            Ok(())
                        }
                        Out => {
                            selected_goal.pop_child();
                            Ok(())
                        }
                    },
                    None => {
                        if !goals.is_empty() {
                            *self = Cursor::SelectedGoal(Some(SelectedGoal {
                                root_goal_index: 0,
                                child_index_path: vec![],
                            }));
                        }

                        Ok(())
                    }
                }
            }
        }
    }
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

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct ErrorLog(Vec<String>);

#[allow(unused)]
#[derive(thiserror::Error, Debug, Clone)]
enum LoadError {
    #[error("APP_DATA or $t HOME directory not found: {0}")]
    NoAppDataOrHomeDirectory(#[from] VarError),
    #[error("Failed to create profile data at {0}: {1}")]
    ProfileDataCreation(PathBuf, String),
    #[error("Failed to read profile data at {0}: {1}")]
    ProfileDataFileRead(PathBuf, String),
    #[error("Profile data at {0} is malformed: {1}")]
    MalformedProfileDataFile(PathBuf, String),
    #[error("Failed to write default data to new file at {0}: {1}")]
    FailureToWriteDefaultData(PathBuf, String),
}

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
            Command::perform(async { PersistentState::load().await }, Message::Loaded),
        )
    }

    fn title(&self) -> String {
        "Goals".to_string()
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

#[cfg(not(target_arch = "wasm32"))]
impl PersistentState {
    #[cfg(target_os = "windows")]
    fn default_data_path() -> Result<PathBuf, LoadError> {
        let appdata = PathBuf::from(std::env::var("APPDATA")?);

        Ok(app_data.join("Roaming/GoalsIced/data"))
    }

    #[cfg(target_os = "linux")]
    fn default_data_path() -> Result<PathBuf, LoadError> {
        let home = PathBuf::from(std::env::var("HOME")?);
        Ok(home.join(".goals-iced"))
    }

    #[cfg(target_os = "macos")]
    fn default_data_path() -> Result<PathBuf, LoadError> {
        Ok(PathBuf::from("~Library/Application Suppoer/GoalsIced/Data"))
    }

    pub async fn load() -> Result<Self, LoadError> {
        use tokio::fs;

        let profile_data_path = std::env::var("GOALS_ICED_DATA_PATH")
            .map(PathBuf::from)
            .unwrap_or(Self::default_data_path()?);

        if !profile_data_path.exists() {
            fs::create_dir_all(
                profile_data_path
                    .parent()
                    .expect("profile data path to have parent"),
            )
            .await
            .map_err(|e| {
                LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
            })?;

            let default_data = rmp_serde::encode::to_vec(&Self::default())
                .expect("default data type to be serializable");

            fs::File::create(&profile_data_path)
                .await
                .map_err(|e| {
                    LoadError::ProfileDataCreation(profile_data_path.clone(), e.to_string())
                })?
                .write_all(&default_data)
                .await
                .map_err(|e| {
                    LoadError::FailureToWriteDefaultData(profile_data_path.clone(), e.to_string())
                })?;
        }

        let mut data_file = fs::File::open(profile_data_path.clone())
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
            })?;

        let mut profile_bytes = vec![];
        data_file
            .read_to_end(&mut profile_bytes)
            .await
            .map_err(|e| {
                LoadError::ProfileDataFileRead(profile_data_path.clone(), e.to_string())
            })?;

        rmp_serde::decode::from_slice(&profile_bytes).map_err(|e| {
            LoadError::MalformedProfileDataFile(profile_data_path.clone(), e.to_string())
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl PersistentState {
    pub async fn load() -> Self {
        todo!()
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
    let title = text("Goals")
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
