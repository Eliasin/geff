use serde::{Deserialize, Serialize};

use geff_core::goal::{GoalId, PopulatedGoal};
use geff_core::profile::goal_traversal::{traverse_populated_goal_children, GoalChildIndexPath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedGoal {
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
pub enum CursorError {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cursor {
    SelectedGoal(Option<SelectedGoal>),
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor::SelectedGoal(None)
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash, Debug)]
pub enum CursorAction {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "out")]
    Out,
}

pub fn selected_goal_siblings<'a>(
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

pub fn get_selected_goal<'a>(
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

pub fn get_selected_goal_id(
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
                            if selected_goal.pop_child().is_none() {
                                *selected_goal_index_path = None;
                            }
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
