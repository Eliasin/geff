use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    event::{Event, EventId},
    goal::{Goal, GoalId, PopulatedGoal},
    query::TimeOfDayConfiguration,
};

pub struct ProfileAndDateTime<'a>(pub &'a mut Profile, pub DateTime<Utc>);

pub mod goal_traversal;
use goal_traversal::{get_root_goals, populate_goal_tree, visit_tree_with_predicate};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Profile {
    goal_id_count: u32,
    event_id_count: u32,
    focused_goals: HashSet<GoalId>,
    pub(crate) goals: HashMap<GoalId, Goal>,
    pub(crate) events: HashMap<EventId, Event>,
    pub(crate) time_of_day_config: TimeOfDayConfiguration,
}

impl Profile {
    pub fn time_of_day_config(&self) -> &TimeOfDayConfiguration {
        &self.time_of_day_config
    }

    pub fn set_time_of_day_config(&mut self, config: TimeOfDayConfiguration) {
        self.time_of_day_config = config;
    }

    pub fn focus_single_goal(&mut self, id: GoalId) -> bool {
        if self.goals.contains_key(&id) {
            self.focused_goals.insert(id);
            true
        } else {
            false
        }
    }

    pub fn unfocus_single_goal(&mut self, id: GoalId) -> bool {
        if self.focused_goals.contains(&id) {
            self.focused_goals.remove(&id);
            true
        } else {
            false
        }
    }

    pub fn add_goal(&mut self, goal: Goal) -> GoalId {
        let goal_id = GoalId(self.goal_id_count);
        self.goal_id_count += 1;

        if self.goals.insert(goal_id, goal).is_some() {
            panic!("not to have a goal id conflict due to monotonic counter");
        }

        goal_id
    }

    pub fn focus_goal(&mut self, goal_id: GoalId) -> Option<HashSet<GoalId>> {
        visit_tree_with_predicate(&self.goals, goal_id, &mut |child_id, _| -> bool {
            !self.focused_goals.contains(&child_id)
        })
        .map(|mut child_ids_need_focusing| {
            self.focused_goals.insert(goal_id);
            for child_id in child_ids_need_focusing.iter() {
                self.focused_goals.insert(*child_id);
            }

            child_ids_need_focusing.insert(goal_id);
            child_ids_need_focusing
        })
    }

    pub fn unfocus_goal(&mut self, goal_id: GoalId) -> Option<HashSet<GoalId>> {
        if self.focused_goals.contains(&goal_id) {
            visit_tree_with_predicate(&self.goals, goal_id, &mut |child_id, _| -> bool {
                self.focused_goals.contains(&child_id)
            })
            .map(|mut child_ids_need_unfocusing| {
                self.focused_goals.remove(&goal_id);
                for child_id in child_ids_need_unfocusing.iter() {
                    self.focused_goals.remove(child_id);
                }

                child_ids_need_unfocusing.insert(goal_id);
                child_ids_need_unfocusing
            })
        } else {
            None
        }
    }

    pub fn rescope_goal(&mut self, goal_id: GoalId, new_effort_to_complete: u32) -> Option<u32> {
        if let Some(goal) = self.goals.get_mut(&goal_id) {
            let original_effort_to_complete = goal.effort_to_complete();
            goal.rescope(new_effort_to_complete);
            Some(original_effort_to_complete)
        } else {
            None
        }
    }

    pub fn rename_goal<S: Into<String>>(&mut self, goal_id: GoalId, new_name: S) -> Option<String> {
        self.goals
            .get_mut(&goal_id)
            .map(|goal| goal.rename(new_name))
    }

    pub fn refine_goal(
        &mut self,
        child_goal: Goal,
        parent_goal_id: GoalId,
        parent_effort_removed: u32,
    ) -> Option<GoalId> {
        let Some(parent_goal) = self.goals.get_mut(&parent_goal_id) else {
                return None;
            };

        let child_goal_id = GoalId(self.goal_id_count);
        self.goal_id_count += 1;

        parent_goal.refine(child_goal_id, parent_effort_removed);

        if self.goals.insert(child_goal_id, child_goal).is_some() {
            panic!("not to have a goal id conflict due to monotonic counter");
        }

        Some(child_goal_id)
    }

    fn remove_goals_from_event_relationships(&mut self, goal_ids: &HashSet<GoalId>) {
        for event in &mut self.events.values_mut() {
            event.goal_relationships_mut().retain(|goal| match goal {
                crate::goal::GoalRelationship::Requires(id) => goal_ids.contains(id),
                crate::goal::GoalRelationship::Ends(id) => goal_ids.contains(id),
                crate::goal::GoalRelationship::WorksOn(id) => goal_ids.contains(id),
                crate::goal::GoalRelationship::Starts(id) => goal_ids.contains(id),
            })
        }
    }

    pub fn remove_goal(&mut self, goal_id: GoalId) -> Option<PopulatedGoal> {
        if let Some((populated_goal, child_ids_needing_removal)) =
            populate_goal_tree(&self.goals, goal_id)
        {
            self.goals.remove(&goal_id);
            self.focused_goals.remove(&goal_id);

            for goal_id in child_ids_needing_removal.iter() {
                self.goals.remove(goal_id);
                self.focused_goals.remove(goal_id);
            }

            self.remove_goals_from_event_relationships(&child_ids_needing_removal);
            if let Some(parent_goal_id) = populated_goal.parent_goal_id {
                if let Some(parent_goal) = self.goals.get_mut(&parent_goal_id) {
                    parent_goal.remove_child(goal_id);
                }
            }

            Some(populated_goal)
        } else {
            None
        }
    }

    pub fn add_event(&mut self, event: Event) -> EventId {
        let event_id = EventId(self.event_id_count);
        self.event_id_count += 1;

        if self.events.insert(event_id, event).is_some() {
            panic!("not to have an event id conflict due to monotonic counter");
        }

        event_id
    }

    pub fn remove_event(&mut self, event_id: EventId) -> Option<Event> {
        self.events.remove(&event_id)
    }

    pub fn populate_goals(&self) -> Vec<PopulatedGoal> {
        let root_goal_ids = get_root_goals(&self.goals);

        root_goal_ids
            .map(|root_goal_id| populate_goal_tree(&self.goals, root_goal_id).unwrap().0)
            .collect()
    }

    pub fn with_datetime(&mut self, datetime: DateTime<Utc>) -> ProfileAndDateTime {
        ProfileAndDateTime(self, datetime)
    }

    pub fn get_event(&self, id: EventId) -> Option<&Event> {
        self.events.get(&id)
    }

    pub fn get_event_mut(&mut self, id: EventId) -> Option<&mut Event> {
        self.events.get_mut(&id)
    }

    pub fn get_goal(&self, id: GoalId) -> Option<&Goal> {
        self.goals.get(&id)
    }

    pub fn get_goal_mut(&mut self, id: GoalId) -> Option<&mut Goal> {
        self.goals.get_mut(&id)
    }

    pub fn focused_goals(&self) -> &HashSet<GoalId> {
        &self.focused_goals
    }

    pub fn unfocused_goals(&self) -> HashSet<GoalId> {
        self.goal_ids()
            .difference(&self.focused_goals)
            .copied()
            .collect()
    }

    pub fn goal_ids(&self) -> HashSet<GoalId> {
        self.goals.iter().map(|(&id, _)| id).collect()
    }
}

impl<'a> ProfileAndDateTime<'a> {
    pub fn get_event(&self, id: EventId) -> Option<&Event> {
        self.0.events.get(&id)
    }

    pub fn get_event_mut(&mut self, id: EventId) -> Option<&mut Event> {
        self.0.events.get_mut(&id)
    }

    pub fn get_goal(&self, id: GoalId) -> Option<&Goal> {
        self.0.goals.get(&id)
    }

    pub fn get_goal_mut(&mut self, id: GoalId) -> Option<&mut Goal> {
        self.0.goals.get_mut(&id)
    }
}

#[cfg(test)]
mod tests {
    mod goal_query {
        use std::collections::HashSet;

        use chrono::{TimeZone, Utc};

        use crate::{goal::Goal, profile::Profile, query::GoalQueryEngine};

        #[test]
        fn goal_finish_status() {
            let mut profile = Profile::default();

            let datetime = Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();
            let mut profile = profile.with_datetime(datetime);

            let goal_id = profile.0.add_goal(Goal::new("test goal", 2));

            assert!(!profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
            assert_eq!(profile.finished_goals(), HashSet::from([]));

            profile.get_goal_mut(goal_id).unwrap().add_effort(1);
            assert!(!profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
            assert_eq!(profile.finished_goals(), HashSet::from([]));

            profile.get_goal_mut(goal_id).unwrap().add_effort(1);
            assert!(profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([]));
            assert_eq!(profile.finished_goals(), HashSet::from([goal_id]));

            profile.get_goal_mut(goal_id).unwrap().rescope(4);
            assert!(!profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
            assert_eq!(profile.finished_goals(), HashSet::from([]));

            profile.get_goal_mut(goal_id).unwrap().add_effort(1);
            assert!(!profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
            assert_eq!(profile.finished_goals(), HashSet::from([]));

            profile.get_goal_mut(goal_id).unwrap().add_effort(1);
            assert!(profile.get_goal(goal_id).unwrap().finished());
            assert_eq!(profile.unfinished_goals(), HashSet::from([]));
            assert_eq!(profile.finished_goals(), HashSet::from([goal_id]));
        }

        #[test]
        fn goal_deletion() {
            let mut profile = Profile::default();

            let datetime = Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();

            let profile = profile.with_datetime(datetime);

            let root_goal = Goal::new("root", 0);
            let first_child_goal = Goal::new("first_child", 0);
            let second_child_goal = Goal::new("second_child", 0);
            let first_grandchild_goal = Goal::new("first_grandchild_child", 0);
            let second_grandchild_goal = Goal::new("second_grandchild_child", 0);

            let root_id = profile.0.add_goal(root_goal.clone());
            let first_child_id = profile
                .0
                .refine_goal(first_child_goal.clone(), root_id, 0)
                .unwrap();
            let second_child_id = profile
                .0
                .refine_goal(second_child_goal.clone(), root_id, 0)
                .unwrap();
            let first_grandchild_id = profile
                .0
                .refine_goal(first_grandchild_goal.clone(), second_child_id, 0)
                .unwrap();
            let second_grandchild_id = profile
                .0
                .refine_goal(second_grandchild_goal.clone(), second_child_id, 0)
                .unwrap();

            let all_goals = {
                let mut all_goals = HashSet::new();

                all_goals.insert(root_id);
                all_goals.insert(first_child_id);
                all_goals.insert(second_child_id);
                all_goals.insert(first_grandchild_id);
                all_goals.insert(second_grandchild_id);

                all_goals
            };

            assert_eq!(all_goals, profile.goal_ids());

            assert!(profile.0.remove_goal(second_child_id).is_some());

            let goals_after_deletion = {
                let mut goals_after_deletion = HashSet::new();

                goals_after_deletion.insert(root_id);
                goals_after_deletion.insert(first_child_id);

                goals_after_deletion
            };

            assert_eq!(goals_after_deletion, profile.goal_ids());
        }
    }
}
