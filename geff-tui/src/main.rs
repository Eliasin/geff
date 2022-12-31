use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use geff_core::{
    goal::{GoalEvent, GoalId, PopulatedGoal},
    profile::{
        goal_traversal::{
            traverse_populated_goal_children, visit_populated_goal_children, GoalChildIndexPath,
        },
        Profile,
    },
    request::{GoalRequest, GoalRequestHandler},
    DateTime, Utc,
};
use geff_util::{get_selected_goal_id, Cursor, LoadError};
use nom::Finish;
use parser::Command;
use std::{cell::RefCell, io, ops::Deref, time::Instant};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

mod parser;

struct ErrorLog {
    lines: RefCell<Vec<String>>,
    dismissed: RefCell<bool>,
}

#[allow(unused)]
impl ErrorLog {
    pub fn dismiss(&self) {
        *self.dismissed.borrow_mut() = true;
    }

    pub fn error_popup(&self) -> Option<String> {
        (!*self.dismissed.borrow())
            .then_some(
                self.lines
                    .borrow()
                    .last()
                    .map(|s| format!("{s}\nPress 'd' to dismiss")),
            )
            .flatten()
    }

    pub fn content(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }

    pub fn push_error<S: Into<String>>(&self, error_text: S) {
        self.lines.borrow_mut().push(error_text.into());
        *self.dismissed.borrow_mut() = false;
    }

    pub fn replace_error<S: Into<String>>(&self, error_text: S) {
        self.lines.borrow_mut().pop();
        self.push_error(error_text);
    }
}

impl Default for ErrorLog {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            dismissed: RefCell::new(true),
        }
    }
}

#[derive(Debug, Default, Clone)]
struct PersistentState {
    profile: Profile,
    goal_event_history: Vec<GoalEvent>,
}

const VERT_MARGIN: u16 = 1;
const HORI_MARGIN: u16 = 1;
struct App {
    persistent_state: PersistentState,
    current_datetime: DateTime<Utc>,
    cursor: Cursor,
    commandline: Option<String>,
    error_log: ErrorLog,
    should_quit: bool,
    populated_goals: Vec<PopulatedGoal>,
}

impl App {
    pub async fn new() -> Result<Self, LoadError> {
        let (profile, goal_event_history, _) = geff_util::PersistentState::<()>::load("geff-tui")
            .await?
            .into();

        let populated_goals = profile.populate_goals();

        Ok(App {
            persistent_state: PersistentState {
                profile,
                goal_event_history,
            },
            current_datetime: Utc::now(),
            cursor: Cursor::SelectedGoal(None),
            commandline: None,
            error_log: ErrorLog::default(),
            should_quit: false,
            populated_goals,
        })
    }

    fn command_slice_in_view(commandline: &str, commandline_chunk_width: u16) -> &str {
        let commandline_chunk_width: usize = commandline_chunk_width.into();
        if commandline_chunk_width == 0 {
            return "";
        }

        if commandline.len() <= commandline_chunk_width {
            commandline
        } else {
            &commandline[(commandline.len() - (commandline_chunk_width - 1))..]
        }
    }

    fn draw_error_log_and_get_sibling_chunks<B: Backend>(
        &self,
        frame: &mut Frame<B>,
    ) -> (Rect, Rect) {
        let error_popup = self.error_log.error_popup();

        if let Some(error_popup) = error_popup {
            let chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .vertical_margin(VERT_MARGIN)
                .horizontal_margin(HORI_MARGIN)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(frame.size());

            let main_and_error_chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(chunks[0]);

            let error_text = Paragraph::new(error_popup)
                .block(Block::default().title("Error").borders(Borders::ALL))
                .wrap(Wrap { trim: false });
            frame.render_widget(error_text, main_and_error_chunks[1]);

            (main_and_error_chunks[0], chunks[1])
        } else {
            let chunks = Layout::default()
                .direction(tui::layout::Direction::Vertical)
                .vertical_margin(VERT_MARGIN)
                .horizontal_margin(HORI_MARGIN)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(frame.size());

            (chunks[0], chunks[1])
        }
    }

    fn draw_commandline<B: Backend>(&self, frame: &mut Frame<B>, commandline_chunk: Rect) {
        frame.render_widget(
            Paragraph::new(
                self.commandline
                    .as_ref()
                    .map(|s| {
                        App::command_slice_in_view(
                            &format!(":{s}\u{2588}"),
                            commandline_chunk.width.saturating_sub(2),
                        )
                        .to_string()
                    })
                    .unwrap_or("".to_string()),
            )
            .block(Block::default())
            .wrap(Wrap { trim: false }),
            commandline_chunk,
        );
    }

    fn goal_text(&self, goal: &PopulatedGoal, selected_goal_id: &Option<GoalId>) -> String {
        let mut prefix = String::from("");

        if selected_goal_id
            .deref()
            .map(|selected_goal_id| selected_goal_id == goal.id)
            .unwrap_or(false)
        {
            prefix.push('*');
        }

        if self
            .persistent_state
            .profile
            .focused_goals()
            .contains(&goal.id)
        {
            prefix.push('F');
        }

        if !prefix.is_empty() {
            prefix.push(' ');
        }

        format!(
            "{prefix}{} ({}/{})",
            goal.name, goal.effort_to_date, goal.effort_to_complete
        )
    }

    fn render_root_goal(
        &self,
        root_goal: &PopulatedGoal,
        selected_goal_id: &Option<GoalId>,
    ) -> anyhow::Result<Vec<String>> {
        let mut layers: Vec<Vec<GoalChildIndexPath>> = vec![];
        visit_populated_goal_children(
            root_goal,
            &mut |_, _, child_path, _| {
                if let Some(layer) = layers.get_mut(child_path.len() - 1) {
                    layer.push(child_path.clone());
                } else {
                    layers.push(vec![child_path.clone()]);
                }
            },
            (),
        );

        let mut columns = vec![vec![]; layers.len() + 2];
        let root_goal_text = self.goal_text(root_goal, selected_goal_id);

        columns[0].push(root_goal_text.clone());
        for _ in 0..(usize::max(root_goal.max_child_layer_width, 1) - 1) {
            columns[0].push(" ".repeat(root_goal_text.len()))
        }

        for (index, layer) in layers.iter().enumerate() {
            let target_column_index = index + 1;

            for goal_path in layer {
                let goal = traverse_populated_goal_children(root_goal, goal_path)
                    .expect("goal path to be valid");

                let goal_text = self.goal_text(goal, selected_goal_id);
                columns[target_column_index].push(goal_text.clone());

                for _ in 0..(usize::max(goal.max_child_layer_width, 1) - 1) {
                    columns[target_column_index].push(" ".repeat(goal_text.len()));
                }

                if goal.children.is_empty() {
                    columns[target_column_index + 1].push(" ".to_string());
                }
            }
        }

        let mut final_rows = vec![String::new(); usize::max(root_goal.max_child_layer_width, 1)];
        for column in columns.iter() {
            for (row_num, row) in column.iter().enumerate() {
                final_rows[row_num].push(' ');
                final_rows[row_num].push_str(row);
            }
        }

        Ok(final_rows)
    }

    fn draw_main<B: Backend>(&self, frame: &mut Frame<B>, main_chunk: Rect) -> anyhow::Result<()> {
        let selected_goal_id = if let Cursor::SelectedGoal(Some(selected_goal)) = &self.cursor {
            get_selected_goal_id(selected_goal, &self.populated_goals).ok()
        } else {
            None
        };

        let goal_text_rows: Vec<String> = self
            .populated_goals
            .iter()
            .flat_map(|root_goal| {
                self.render_root_goal(root_goal, &selected_goal_id)
                    .unwrap_or(vec![format!(
                        "ERROR RENDERING ROOT GOAL {}",
                        root_goal.name
                    )])
            })
            .collect();

        let goal_text_widget = Paragraph::new(goal_text_rows.join("\n"))
            .block(Block::default().title("Goals").borders(Borders::ALL));

        frame.render_widget(goal_text_widget, main_chunk);

        Ok(())
    }

    pub fn draw<B: Backend>(&mut self, frame: &mut Frame<B>) {
        let (main_chunk, commandline_chunk) = self.draw_error_log_and_get_sibling_chunks(frame);

        if let Err(e) = self.draw_main(frame, main_chunk) {
            self.error_log.push_error(e.to_string());
        }

        self.draw_commandline(frame, commandline_chunk);
    }

    fn handle_character_key(&mut self, c: char) -> anyhow::Result<()> {
        if let Some(commandline) = &mut self.commandline {
            commandline.push(c);
        } else if c == ':' {
            self.commandline = Some(String::from(""));
        } else {
            let cursor_action = match c {
                'h' => Some(geff_util::CursorAction::Out),
                'j' => Some(geff_util::CursorAction::Down),
                'k' => Some(geff_util::CursorAction::Up),
                'l' => Some(geff_util::CursorAction::In),
                'd' => {
                    self.error_log.dismiss();
                    None
                }
                _ => None,
            };

            if let Some(cursor_action) = cursor_action {
                self.cursor
                    .handle_action(cursor_action, &self.populated_goals)?;
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, command: Command) -> anyhow::Result<()> {
        let mut profile = self
            .persistent_state
            .profile
            .with_datetime(self.current_datetime);

        let emitted_events = match command {
            Command::Create {
                name,
                effort_to_complete,
            } => profile.handle_request(GoalRequest::Add {
                name,
                effort_to_complete,
            }),
            Command::Delete => {
                if let Cursor::SelectedGoal(Some(selected_goal)) = &self.cursor {
                    let selected_goal_id =
                        get_selected_goal_id(selected_goal, &self.populated_goals)?;

                    self.cursor
                        .handle_action(geff_util::CursorAction::Out, &self.populated_goals)?;
                    profile.handle_request(GoalRequest::Delete(selected_goal_id))
                } else {
                    self.error_log.push_error("No goal selected");

                    vec![]
                }
            }
            Command::Refine {
                child_name,
                child_effort_to_complete,
                parent_effort_removed,
            } => {
                if let Cursor::SelectedGoal(Some(selected_goal)) = &self.cursor {
                    let selected_goal_id =
                        get_selected_goal_id(selected_goal, &self.populated_goals)?;

                    profile.handle_request(GoalRequest::Refine {
                        parent_goal_id: selected_goal_id,
                        parent_effort_removed,
                        child_name,
                        child_effort_to_complete,
                    })
                } else {
                    self.error_log.push_error("No goal selected");

                    vec![]
                }
            }
            Command::AddEffort { effort } => {
                if let Cursor::SelectedGoal(Some(selected_goal)) = &self.cursor {
                    let selected_goal_id =
                        get_selected_goal_id(selected_goal, &self.populated_goals)?;

                    profile.handle_request(GoalRequest::AddEffort {
                        goal_id: selected_goal_id,
                        effort,
                    })
                } else {
                    self.error_log.push_error("No goal selected");

                    vec![]
                }
            }
            Command::RemoveEffort { effort } => {
                if let Cursor::SelectedGoal(Some(selected_goal)) = &self.cursor {
                    let selected_goal_id =
                        get_selected_goal_id(selected_goal, &self.populated_goals)?;

                    profile.handle_request(GoalRequest::RemoveEffort {
                        goal_id: selected_goal_id,
                        effort,
                    })
                } else {
                    self.error_log.push_error("No goal selected");

                    vec![]
                }
            }
        };

        self.persistent_state
            .goal_event_history
            .extend(emitted_events);

        self.populated_goals = self.persistent_state.profile.populate_goals();

        Ok(())
    }

    pub async fn handle_key_event(&mut self, event: KeyEvent) -> anyhow::Result<()> {
        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        match event.code {
            KeyCode::Char(c) => self.handle_character_key(c)?,
            KeyCode::Backspace => {
                if let Some(commandline) = &mut self.commandline {
                    commandline.pop();

                    if commandline.is_empty() {
                        self.commandline = None;
                    }
                }
            }
            KeyCode::Esc => {
                self.commandline = None;
            }
            KeyCode::Enter => {
                if let Some(commandline) = &self.commandline {
                    let command = parser::command(commandline.as_str()).finish();

                    match command {
                        Ok((_, command)) => self.handle_command(command)?,
                        Err(e) => self.error_log.push_error(e.to_string()),
                    }

                    self.commandline = None;
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let tick_rate = std::time::Duration::from_millis(150);
    let last_tick = Instant::now();
    let mut app = App::new().await?;

    while !app.should_quit {
        terminal.draw(|f| {
            app.draw(f);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| std::time::Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(event) = crossterm::event::read()? {
                app.handle_key_event(event).await?;
            }
        }
    }

    terminal.show_cursor()?;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(())
}
