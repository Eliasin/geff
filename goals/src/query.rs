use std::collections::HashSet;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    event::{EventId, TimeOfDay},
    goal::GoalId,
};

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct TimeOfDayConfiguration {
    midday_start: NaiveTime,
    evening_start: NaiveTime,
}

#[derive(Error, Debug, Clone, Copy)]
pub enum TimeOfDayCreationError {
    #[error(
        "supplied midday start ({midday_start:?}) is at or after evening start ({evening_start:?})"
    )]
    SuppliedMiddayIsAfterEvening {
        midday_start: NaiveTime,
        evening_start: NaiveTime,
    },
}

impl TimeOfDayConfiguration {
    pub fn from_start_of_midday_and_evening(
        midday_start: NaiveTime,
        evening_start: NaiveTime,
    ) -> Result<TimeOfDayConfiguration, TimeOfDayCreationError> {
        if midday_start < evening_start {
            Ok(TimeOfDayConfiguration {
                midday_start,
                evening_start,
            })
        } else {
            Err(TimeOfDayCreationError::SuppliedMiddayIsAfterEvening {
                midday_start,
                evening_start,
            })
        }
    }

    pub fn map_time(&self, time: NaiveTime) -> TimeOfDay {
        if time < self.midday_start {
            TimeOfDay::Morning
        } else if time < self.evening_start {
            TimeOfDay::Midday
        } else {
            TimeOfDay::Evening
        }
    }
}

impl Default for TimeOfDayConfiguration {
    fn default() -> Self {
        Self {
            midday_start: NaiveTime::from_hms_opt(12, 0, 0).expect("12h to be less than 24h"),
            evening_start: NaiveTime::from_hms_opt(18, 0, 0).expect("18h to be less than 24h"),
        }
    }
}

pub trait EventQueryEngine {
    fn currently_occuring_events(&self) -> HashSet<EventId>;

    fn past_events(&self) -> HashSet<EventId>;
    fn future_events(&self) -> HashSet<EventId>;
    fn event_ids(&self) -> HashSet<EventId>;
}

pub mod event_query_helpers {

    use chrono::{DateTime, Local, Utc};

    use crate::event::Event;

    use super::TimeOfDayConfiguration;

    pub fn event_not_started(
        time_of_day_config: &TimeOfDayConfiguration,
        reference: DateTime<Utc>,
        event: &Event,
    ) -> bool {
        match event {
            Event::BlockEvent(event) => {
                let local_event_start = event.start.with_timezone(&Local);

                reference < local_event_start
            }
            Event::InstantEvent(event) => {
                let local_event_time = event.time.with_timezone(&Local);

                reference < local_event_time
            }
            Event::FloatingEvent(event) => match reference.date_naive().cmp(&event.date) {
                std::cmp::Ordering::Equal => {
                    let reference_time_of_day = time_of_day_config.map_time(reference.time());

                    !reference_time_of_day.during_or_after(event.time_of_day)
                }
                std::cmp::Ordering::Less => true,
                std::cmp::Ordering::Greater => false,
            },
        }
    }

    pub fn event_occuring(
        time_of_day_config: &TimeOfDayConfiguration,
        reference: DateTime<Utc>,
        event: &Event,
    ) -> bool {
        match event {
            Event::BlockEvent(event) => {
                let event_end = event.start + event.duration;
                let local_event_start = event.start.with_timezone(&Local);
                let local_event_end = event_end.with_timezone(&Local);

                reference <= local_event_end && reference >= local_event_start
            }
            Event::InstantEvent(_) => false,
            Event::FloatingEvent(event) => match reference.date_naive().cmp(&event.date) {
                std::cmp::Ordering::Equal => {
                    let reference_time_of_day = time_of_day_config.map_time(reference.time());

                    reference_time_of_day == event.time_of_day
                }
                _ => false,
            },
        }
    }

    pub fn event_ended(
        time_of_day_config: &TimeOfDayConfiguration,
        reference: DateTime<Utc>,
        event: &Event,
    ) -> bool {
        match event {
            Event::BlockEvent(event) => {
                let event_end = event.start + event.duration;
                let local_event_end = event_end.with_timezone(&Local);

                reference > local_event_end
            }
            Event::InstantEvent(event) => {
                let local_event_time = event.time.with_timezone(&Local);

                reference >= local_event_time
            }
            Event::FloatingEvent(event) => match reference.date_naive().cmp(&event.date) {
                std::cmp::Ordering::Less => false,
                std::cmp::Ordering::Equal => {
                    let reference_time_of_day = time_of_day_config.map_time(reference.time());

                    reference_time_of_day.during_or_after(event.time_of_day)
                }
                std::cmp::Ordering::Greater => true,
            },
        }
    }
}

pub mod goal_query_helpers {
    use crate::{
        event::Event,
        goal::{GoalId, GoalRelationship},
    };

    pub fn goal_start_event<'a, E: Iterator<Item = &'a Event>>(
        goal_id: GoalId,
        events: E,
    ) -> Option<Event> {
        for event in events {
            for relationship in event.goal_relationships() {
                match relationship {
                    GoalRelationship::Starts(id) if *id == goal_id => return Some(event.clone()),
                    _ => {}
                }
            }
        }

        None
    }
    pub fn goal_end_event<'a, E: Iterator<Item = &'a Event>>(
        goal_id: GoalId,
        events: E,
    ) -> Option<Event> {
        for event in events {
            for relationship in event.goal_relationships() {
                match relationship {
                    GoalRelationship::Ends(id) if *id == goal_id => return Some(event.clone()),
                    _ => {}
                }
            }
        }

        None
    }

    pub fn goal_has_end<'a, E: Iterator<Item = &'a Event>>(goal_id: GoalId, events: E) -> bool {
        goal_end_event(goal_id, events).is_some()
    }

    pub fn goal_has_start<'a, E: Iterator<Item = &'a Event>>(goal_id: GoalId, events: E) -> bool {
        goal_start_event(goal_id, events).is_some()
    }
}

pub trait GoalQueryEngine {
    /// Active goals are the subset of goals that are
    /// - Unfinished
    /// - Past their start date or have no associated event with a start date
    /// - Are before their end date or have no associated event with an end date
    fn active_goals(&self) -> HashSet<GoalId> {
        let started_goals = self.started_goals();
        let unfinished_goals = self.unfinished_goals();
        let ended_goals = self.ended_goals();

        started_goals
            .into_iter()
            .filter(|g| unfinished_goals.contains(g) && ended_goals.contains(g))
            .collect()
    }

    fn inactive_goals(&self) -> HashSet<GoalId> {
        let active_goals = self.active_goals();
        self.goal_ids()
            .into_iter()
            .filter(|id| !active_goals.contains(id))
            .collect()
    }

    fn unfinished_goals(&self) -> HashSet<GoalId>;
    fn finished_goals(&self) -> HashSet<GoalId>;
    fn ended_goals(&self) -> HashSet<GoalId>;
    fn started_goals(&self) -> HashSet<GoalId>;

    fn goal_ids(&self) -> HashSet<GoalId>;
}
