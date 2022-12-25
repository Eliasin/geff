use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone)]
pub enum GoalRelationship {
    Requires(GoalId),
    Ends(GoalId),
    WorksOn(GoalId),
    Starts(GoalId),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum GoalEvent {
    Effort {
        goal_id: GoalId,
        effort_done: u32,
    },
    Activated(GoalId),
    Deactivated(GoalId),
    RescopeByFinish {
        goal_id: GoalId,
        effort_done: u32,
    },
    Rescope {
        goal_id: GoalId,
        new_effort: u32,
    },
    Add {
        goal_id: GoalId,
    },
    Refine {
        parent_goal_id: GoalId,
        parent_effort_removed: u32,
        new_child_goal: GoalId,
    },
    Delete {
        deleted_goal_data: PopulatedGoal,
    },
    Rename {
        goal_id: GoalId,
        new_name: String,
        old_name: String,
    },
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct GoalId(pub(crate) u32);

#[derive(Serialize, Deserialize, Clone)]
pub struct Goal {
    name: String,
    effort_to_date: u32,
    effort_to_complete: u32,
    children: HashSet<GoalId>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct PopulatedGoal {
    pub id: GoalId,
    pub parent_goal_id: Option<GoalId>,
    pub name: String,
    pub effort_to_date: u32,
    pub effort_to_complete: u32,
    pub children: Vec<PopulatedGoal>,
}

impl Goal {
    pub fn new<S: Into<String>>(name: S, effort_to_complete: u32) -> Goal {
        Goal {
            name: name.into(),
            effort_to_date: 0,
            effort_to_complete,
            children: HashSet::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename<S: Into<String>>(&mut self, new_name: S) {
        self.name = new_name.into()
    }

    pub fn add_effort(&mut self, effort: u32) {
        self.effort_to_date += effort
    }

    pub fn remove_effort(&mut self, effort: u32) {
        self.effort_to_date -= effort
    }

    pub fn rescope(&mut self, new_effort: u32) {
        self.effort_to_complete = new_effort
    }

    pub fn rescope_by_finish(&mut self, effort_done: u32) {
        self.effort_to_date += effort_done;
        self.effort_to_complete = self.effort_to_date;
    }

    pub fn refine(&mut self, child: GoalId, effort_removed: u32) {
        self.effort_to_complete -= effort_removed;
        self.children.insert(child);
    }

    pub fn remove_child(&mut self, child: GoalId) -> bool {
        self.children.remove(&child)
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

    pub fn children(&self) -> &HashSet<GoalId> {
        &self.children
    }
}
