use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::event::EventId;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum GoalRelationship {
    Requires(GoalId),
    Ends(GoalId),
    WorksOn(GoalId),
    Starts(GoalId),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GoalEvent {
    AddEffort {
        goal_id: GoalId,
        effort: u32,
    },
    RemoveEffort {
        goal_id: GoalId,
        effort: u32,
    },
    Focus {
        focus_root_id: GoalId,
        focused_children: HashSet<GoalId>,
    },
    Unfocus {
        unfocus_root_id: GoalId,
        unfocused_children: HashSet<GoalId>,
    },
    FocusSingle(GoalId),
    UnfocusSingle(GoalId),
    RescopeByFinish {
        goal_id: GoalId,
        finished_by: EventId,
        effort_done_at_time_of_finish: u32,
    },
    Rescope {
        goal_id: GoalId,
        new_effort_to_complete: u32,
        original_effort_to_complete: u32,
    },
    Add {
        goal_id: GoalId,
    },
    Refine {
        parent_goal_id: GoalId,
        parent_effort_removed: u32,
        new_child_goal_id: GoalId,
    },
    Delete {
        deleted_goal_tree: PopulatedGoal,
    },
    Rename {
        goal_id: GoalId,
        old_name: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GoalId(pub u32);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Goal {
    name: String,
    effort_to_date: u32,
    effort_to_complete: u32,
    children: Vec<GoalId>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PopulatedGoal {
    pub id: GoalId,
    #[serde(rename = "parentGoalId")]
    pub parent_goal_id: Option<GoalId>,
    pub name: String,
    #[serde(rename = "effortToDate")]
    pub effort_to_date: u32,
    #[serde(rename = "effortToComplete")]
    pub effort_to_complete: u32,
    #[serde(rename = "maxChildLayerWidth")]
    pub max_child_layer_width: usize,
    #[serde(rename = "maxChildLayerDepth")]
    pub max_child_depth: usize,
    pub children: Vec<PopulatedGoal>,
}

#[derive(thiserror::Error, Debug)]
pub enum GoalOperationError {
    #[error("adding goal `{1:?}` to `{0}` failed as it `{1:?}` is already a child of `{0}`")]
    CannotHaveDuplicateChildren(String, GoalId),
    #[error("no child with id `{1:?}` on goal `{0}`")]
    NoSuchChild(String, GoalId),
}

impl Goal {
    pub fn new<S: Into<String>>(name: S, effort_to_complete: u32) -> Goal {
        Goal {
            name: name.into(),
            effort_to_date: 0,
            effort_to_complete,
            children: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename<S: Into<String>>(&mut self, new_name: S) -> String {
        let old_name = self.name.clone();
        self.name = new_name.into();
        old_name
    }

    pub fn add_effort(&mut self, effort: u32) {
        self.effort_to_date += effort
    }

    pub fn remove_effort(&mut self, effort: u32) {
        self.effort_to_date = self.effort_to_date.saturating_sub(effort);
    }

    pub fn rescope(&mut self, new_effort: u32) {
        self.effort_to_complete = new_effort
    }

    pub fn rescope_by_finish(&mut self, effort_done: u32) {
        self.effort_to_date += effort_done;
        self.effort_to_complete = self.effort_to_date;
    }

    pub fn refine(&mut self, child: GoalId, effort_removed: u32) -> Result<(), GoalOperationError> {
        self.effort_to_complete = self.effort_to_complete.saturating_sub(effort_removed);
        if self.children.contains(&child) {
            return Err(GoalOperationError::CannotHaveDuplicateChildren(
                self.name.clone(),
                child,
            ));
        }
        self.children.push(child);

        Ok(())
    }

    pub fn remove_child(&mut self, child: GoalId) -> bool {
        if let Some(index) = self
            .children
            .iter()
            .position(|child_elem| *child_elem == child)
        {
            self.children.remove(index);
            true
        } else {
            false
        }
    }

    pub fn finished(&self) -> bool {
        self.effort_to_date >= self.effort_to_complete
    }

    pub fn unfinished(&self) -> bool {
        !self.finished()
    }

    pub fn effort_to_complete(&self) -> u32 {
        self.effort_to_complete
    }

    pub fn effort_to_date(&self) -> u32 {
        self.effort_to_date
    }

    pub fn children(&self) -> &Vec<GoalId> {
        &self.children
    }

    pub fn swap_children(
        &mut self,
        child_a: GoalId,
        child_b: GoalId,
    ) -> Result<(), GoalOperationError> {
        if let Some(child_a_position) = self.children.iter().position(|child| *child == child_a) {
            if let Some(child_b_position) = self.children.iter().position(|child| *child == child_b)
            {
                self.children.swap(child_a_position, child_b_position);
                Ok(())
            } else {
                Err(GoalOperationError::NoSuchChild(self.name.clone(), child_b))
            }
        } else {
            Err(GoalOperationError::NoSuchChild(self.name.clone(), child_a))
        }
    }
}
