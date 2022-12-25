#![feature(map_many_mut)]
pub mod event;
pub mod goal;
pub mod query;

pub mod profile {
    use std::collections::{HashMap, HashSet};

    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};

    use crate::{
        event::{Event, EventId},
        goal::{Goal, GoalEvent, GoalId, PopulatedGoal},
        query::{
            event_query_helpers::{event_ended, event_not_started, event_occuring},
            goal_query_helpers, EventQueryEngine, GoalQueryEngine, TimeOfDayConfiguration,
        },
    };

    pub struct ProfileAndDateTime<'a>(&'a mut Profile, DateTime<Utc>);

    struct GoalDeletionTree {
        ids_need_removing: HashSet<GoalId>,
        removed_goal_tree: PopulatedGoal,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct Profile {
        goal_id_count: u32,
        event_id_count: u32,
        goals: HashMap<GoalId, Goal>,
        events: HashMap<EventId, Event>,
        time_of_day_config: TimeOfDayConfiguration,
    }

    type GoalChildIndexPath = Vec<usize>;

    impl Profile {
        pub fn time_of_day_config(&self) -> &TimeOfDayConfiguration {
            &self.time_of_day_config
        }

        pub fn set_time_of_day_config(&mut self, config: TimeOfDayConfiguration) {
            self.time_of_day_config = config;
        }

        pub fn add_goal(&mut self, goal: Goal) -> GoalEvent {
            let goal_id = GoalId(self.goal_id_count);
            self.goal_id_count += 1;

            if self.goals.insert(goal_id, goal).is_some() {
                panic!("not to have a goal id conflict due to monotonic counter");
            }

            GoalEvent::Add { goal_id }
        }

        pub fn refine_goal(
            &mut self,
            child_goal: Goal,
            parent_goal_id: GoalId,
            parent_effort_removed: u32,
        ) -> Option<GoalEvent> {
            let Some(parent_goal) = self.goals.get_mut(&parent_goal_id) else {
                return None;
            };

            let child_goal_id = GoalId(self.goal_id_count);
            self.goal_id_count += 1;

            parent_goal.refine(child_goal_id, parent_effort_removed);

            if self.goals.insert(child_goal_id, child_goal).is_some() {
                panic!("not to have a goal id conflict due to monotonic counter");
            }

            Some(GoalEvent::Add {
                goal_id: child_goal_id,
            })
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

        pub fn get_goal_parent_id(&self, goal_id: GoalId) -> Option<GoalId> {
            self.goals
                .iter()
                .find(|(_, goal)| goal.children().contains(&goal_id))
                .map(|(id, _)| *id)
        }

        fn traverse_goal_children<'a>(
            root_goal: &'a mut PopulatedGoal,
            goal_child_index_path: &GoalChildIndexPath,
        ) -> Option<&'a mut PopulatedGoal> {
            let mut current = root_goal;

            for goal_child_index in goal_child_index_path {
                match current.children.get_mut(*goal_child_index) {
                    Some(child) => current = child,
                    None => return None,
                }
            }

            Some(current)
        }

        fn goal_template_for_deletion(
            goal_id: GoalId,
            goal: &Goal,
            parent_goal_id: Option<GoalId>,
        ) -> PopulatedGoal {
            PopulatedGoal {
                id: goal_id,
                parent_goal_id,
                name: goal.name().to_string(),
                effort_to_date: goal.effort_to_date(),
                effort_to_complete: goal.effort_to_complete(),
                children: vec![],
            }
        }

        fn create_goal_deletion_tree(&self, goal_id: GoalId) -> Option<GoalDeletionTree> {
            if let Some(goal) = self.goals.get(&goal_id) {
                let parent_goal_id = self.get_goal_parent_id(goal_id);

                let mut deleted_goal_data =
                    Profile::goal_template_for_deletion(goal_id, goal, parent_goal_id);

                let mut needs_removing = HashSet::new();
                let mut needs_visiting: Vec<(GoalId, GoalChildIndexPath)> = vec![(goal_id, vec![])];

                while let Some((current_goal_id, goal_child_index_path)) = needs_visiting.pop() {
                    let children = self
                        .get_goal(current_goal_id)
                        .expect("current goal to be in profile")
                        .children();

                    needs_removing.extend(children);

                    for child_id in children {
                        let child = self
                            .get_goal(*child_id)
                            .expect("child goal to be in the profile");

                        let child_goal_data = Profile::goal_template_for_deletion(
                            *child_id,
                            child,
                            Some(current_goal_id),
                        );

                        let current_goal_populated_data = Profile::traverse_goal_children(
                            &mut deleted_goal_data,
                            &goal_child_index_path,
                        )
                        .expect("goal child index path to be valid");

                        let mut child_goal_child_index_path = goal_child_index_path.clone();
                        child_goal_child_index_path
                            .push(current_goal_populated_data.children.len());

                        current_goal_populated_data.children.push(child_goal_data);

                        needs_visiting.push((*child_id, child_goal_child_index_path));
                    }
                }
                Some(GoalDeletionTree {
                    ids_need_removing: needs_removing,
                    removed_goal_tree: deleted_goal_data,
                })
            } else {
                None
            }
        }

        pub fn remove_goal(&mut self, goal_id: GoalId) -> Option<GoalEvent> {
            if let Some(GoalDeletionTree {
                ids_need_removing,
                removed_goal_tree,
            }) = self.create_goal_deletion_tree(goal_id)
            {
                for goal_id in ids_need_removing.iter() {
                    self.goals.remove(goal_id);
                }

                self.remove_goals_from_event_relationships(&ids_need_removing);

                Some(GoalEvent::Delete {
                    deleted_goal_data: removed_goal_tree,
                })
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
    }

    impl<'a> EventQueryEngine for ProfileAndDateTime<'a> {
        fn currently_occuring_events(&self) -> HashSet<EventId> {
            self.0
                .events
                .iter()
                .filter(|(_, event)| event_occuring(&self.0.time_of_day_config, self.1, event))
                .map(|(id, _)| *id)
                .collect()
        }

        fn past_events(&self) -> HashSet<EventId> {
            self.0
                .events
                .iter()
                .filter(|(_, event)| event_ended(&self.0.time_of_day_config, self.1, event))
                .map(|(id, _)| *id)
                .collect()
        }

        fn future_events(&self) -> HashSet<EventId> {
            self.0
                .events
                .iter()
                .filter(|(_, event)| event_not_started(&self.0.time_of_day_config, self.1, event))
                .map(|(id, _)| *id)
                .collect()
        }

        fn event_ids(&self) -> HashSet<EventId> {
            self.0.events.iter().map(|(&id, _)| id).collect()
        }
    }

    impl<'a> GoalQueryEngine for ProfileAndDateTime<'a> {
        fn unfinished_goals(&self) -> HashSet<GoalId> {
            self.0
                .goals
                .iter()
                .filter(|(_, goal)| goal.unfinished())
                .map(|(&id, _)| id)
                .collect()
        }

        fn finished_goals(&self) -> HashSet<GoalId> {
            self.0
                .goals
                .iter()
                .filter(|(_, goal)| goal.finished())
                .map(|(&id, _)| id)
                .collect()
        }

        fn ended_goals(&self) -> HashSet<GoalId> {
            self.0
                .goals
                .iter()
                .filter(|(&id, _)| {
                    match goal_query_helpers::goal_end_event(id, self.0.events.values()) {
                        Some(goal_end_event) => {
                            event_ended(&self.0.time_of_day_config, self.1, &goal_end_event)
                        }
                        None => false,
                    }
                })
                .map(|(&id, _)| id)
                .collect()
        }

        fn started_goals(&self) -> HashSet<GoalId> {
            self.0
                .goals
                .iter()
                .filter(|(&id, _)| {
                    match goal_query_helpers::goal_start_event(id, self.0.events.values()) {
                        Some(goal_end_event) => {
                            !event_not_started(&self.0.time_of_day_config, self.1, &goal_end_event)
                        }
                        None => false,
                    }
                })
                .map(|(&id, _)| id)
                .collect()
        }

        fn goal_ids(&self) -> HashSet<GoalId> {
            self.0.goals.iter().map(|(&id, _)| id).collect()
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

            use crate::{
                goal::{Goal, GoalEvent},
                profile::Profile,
                query::GoalQueryEngine,
            };

            #[test]
            fn goal_finish_status() {
                let mut profile = Profile::default();

                let datetime = Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();
                let profile = profile.with_datetime(datetime);

                let GoalEvent::Add { goal_id } = profile.0.add_goal(Goal::new("test goal", 2)) else {
                    panic!("unexpected goal event contents");
                };

                assert!(!profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
                assert_eq!(profile.finished_goals(), HashSet::from([]));

                profile.0.get_goal_mut(goal_id).unwrap().add_effort(1);
                assert!(!profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
                assert_eq!(profile.finished_goals(), HashSet::from([]));

                profile.0.get_goal_mut(goal_id).unwrap().add_effort(1);
                assert!(profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([]));
                assert_eq!(profile.finished_goals(), HashSet::from([goal_id]));

                profile.0.get_goal_mut(goal_id).unwrap().rescope(4);
                assert!(!profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
                assert_eq!(profile.finished_goals(), HashSet::from([]));

                profile.0.get_goal_mut(goal_id).unwrap().add_effort(1);
                assert!(!profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([goal_id]));
                assert_eq!(profile.finished_goals(), HashSet::from([]));

                profile.0.get_goal_mut(goal_id).unwrap().add_effort(1);
                assert!(profile.0.get_goal(goal_id).unwrap().finished());
                assert_eq!(profile.unfinished_goals(), HashSet::from([]));
                assert_eq!(profile.finished_goals(), HashSet::from([goal_id]));
            }

            #[test]
            fn goal_deletion() {
                let mut profile = Profile::default();

                let datetime = Utc.with_ymd_and_hms(2022, 1, 1, 1, 0, 0).unwrap();
            }
        }
    }
}
