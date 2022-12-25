use crate::{
    goal::{GoalEvent, GoalId},
    profile::ProfileAndDateTime,
};

#[derive(Clone)]
pub enum GoalRequest {
    AddEffort {
        goal_id: GoalId,
        effort: u32,
    },
    Focus(GoalId),
    Unfocus(GoalId),
    FocusSingle(GoalId),
    UnfocusSingle(GoalId),
    RescopeByFinish {
        goal_id: GoalId,
    },
    Rescope {
        goal_id: GoalId,
        new_effort: u32,
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
    fn handle_request(&mut self, request: GoalRequest) -> Option<GoalEvent>;
}

#[allow(unused_variables)]
impl<'a> GoalRequestHandler for ProfileAndDateTime<'a> {
    fn handle_request(&mut self, request: GoalRequest) -> Option<GoalEvent> {
        match request {
            GoalRequest::AddEffort { goal_id, effort } => {
                self.0.goals.get_mut(&goal_id).map(|goal| {
                    goal.add_effort(effort);

                    GoalEvent::Add { goal_id }
                })
            }
            GoalRequest::Focus(goal_id) => self.0.focus_goal(goal_id),
            GoalRequest::Unfocus(_) => todo!(),
            GoalRequest::FocusSingle(_) => todo!(),
            GoalRequest::UnfocusSingle(_) => todo!(),
            GoalRequest::RescopeByFinish { goal_id } => todo!(),
            GoalRequest::Rescope {
                goal_id,
                new_effort,
            } => todo!(),
            GoalRequest::Add {
                name,
                effort_to_complete,
            } => todo!(),
            GoalRequest::Refine {
                parent_goal_id,
                parent_effort_removed,
                child_name,
                child_effort_to_complete,
            } => todo!(),
            GoalRequest::Delete(_) => todo!(),
            GoalRequest::Rename { goal_id, new_name } => todo!(),
        }
    }
}
