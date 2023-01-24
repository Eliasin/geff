use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    goal::{Goal, GoalEvent, GoalId},
    profile::ProfileAndDateTime,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoalRequest {
    AddEffort {
        goal_id: GoalId,
        effort: u32,
    },
    RemoveEffort {
        goal_id: GoalId,
        effort: u32,
    },
    Focus(GoalId),
    Unfocus(GoalId),
    FocusSingle(GoalId),
    UnfocusSingle(GoalId),
    Rescope {
        goal_id: GoalId,
        new_effort_to_complete: u32,
    },
    ProcessDateTime {
        datetime: DateTime<Utc>,
    },
    Add {
        name: String,
        effort_to_complete: u32,
    },
    Refine {
        parent_goal_id: GoalId,
        parent_effort_removed: u32,
        child_name: String,
        child_effort_to_complete: u32,
    },
    Delete(GoalId),
    Rename {
        goal_id: GoalId,
        new_name: String,
    },
}

pub trait GoalRequestHandler {
    fn handle_request(&mut self, request: GoalRequest) -> Vec<GoalEvent>;
}

impl GoalRequestHandler for ProfileAndDateTime<'_> {
    fn handle_request(&mut self, request: GoalRequest) -> Vec<GoalEvent> {
        match request {
            GoalRequest::AddEffort { goal_id, effort } => {
                self.0.goals.get_mut(&goal_id).map_or(vec![], |goal| {
                    goal.add_effort(effort);

                    vec![GoalEvent::AddEffort { goal_id, effort }]
                })
            }
            GoalRequest::RemoveEffort { goal_id, effort } => {
                self.0.goals.get_mut(&goal_id).map_or(vec![], |goal| {
                    goal.remove_effort(effort);

                    vec![GoalEvent::RemoveEffort { goal_id, effort }]
                })
            }
            GoalRequest::Focus(goal_id) => {
                let focused_ids = self.0.focus_goal(goal_id);

                focused_ids.map_or(vec![], |mut focused_ids| {
                    focused_ids.remove(&goal_id);
                    vec![GoalEvent::Focus {
                        focus_root_id: goal_id,
                        focused_children: focused_ids,
                    }]
                })
            }
            GoalRequest::Unfocus(goal_id) => {
                let unfocused_ids = self.0.unfocus_goal(goal_id);

                unfocused_ids.map_or(vec![], |mut focused_ids| {
                    focused_ids.remove(&goal_id);
                    vec![GoalEvent::Unfocus {
                        unfocus_root_id: goal_id,
                        unfocused_children: focused_ids,
                    }]
                })
            }
            GoalRequest::FocusSingle(goal_id) => self
                .0
                .focus_single_goal(goal_id)
                .then_some(goal_id)
                .map_or(vec![], |goal_id| vec![GoalEvent::FocusSingle(goal_id)]),
            GoalRequest::UnfocusSingle(goal_id) => self
                .0
                .unfocus_single_goal(goal_id)
                .then_some(goal_id)
                .map_or(vec![], |goal_id| vec![GoalEvent::UnfocusSingle(goal_id)]),
            GoalRequest::Rescope {
                goal_id,
                new_effort_to_complete,
            } => self.0.rescope_goal(goal_id, new_effort_to_complete).map_or(
                vec![],
                |original_effort_to_complete| {
                    vec![GoalEvent::Rescope {
                        goal_id,
                        new_effort_to_complete,
                        original_effort_to_complete,
                    }]
                },
            ),
            GoalRequest::Add {
                name,
                effort_to_complete,
            } => vec![GoalEvent::Add {
                goal_id: self.0.add_goal(Goal::new(name, effort_to_complete)),
            }],
            GoalRequest::Refine {
                parent_goal_id,
                parent_effort_removed,
                child_name,
                child_effort_to_complete,
            } => self
                .0
                .refine_goal(
                    Goal::new(child_name, child_effort_to_complete),
                    parent_goal_id,
                    parent_effort_removed,
                )
                .map_or(vec![], |child_goal_id| {
                    vec![GoalEvent::Refine {
                        parent_goal_id,
                        parent_effort_removed,
                        new_child_goal_id: child_goal_id,
                    }]
                }),
            GoalRequest::Delete(goal_id) => self
                .0
                .remove_goal(goal_id)
                .map_or(vec![], |deleted_goal_tree| {
                    vec![GoalEvent::Delete { deleted_goal_tree }]
                }),
            GoalRequest::Rename { goal_id, new_name } => self
                .0
                .rename_goal(goal_id, &new_name)
                .map_or(vec![], |old_name| {
                    vec![GoalEvent::Rename { goal_id, old_name }]
                }),
            GoalRequest::ProcessDateTime { datetime: _ } => todo!(),
        }
    }
}
