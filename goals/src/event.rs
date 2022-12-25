use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::goal::GoalRelationship;

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Clone)]
pub struct BlockEvent {
    pub(crate) start: DateTime<Utc>,
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub(crate) duration: Duration,
    pub(crate) goal_relationships: Vec<GoalRelationship>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstantEvent {
    pub(crate) time: DateTime<Utc>,
    pub(crate) goal_relationships: Vec<GoalRelationship>,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum TimeOfDay {
    Morning,
    Midday,
    Evening,
}

impl TimeOfDay {
    pub fn during_or_after(&self, other: TimeOfDay) -> bool {
        match self {
            TimeOfDay::Morning => other == TimeOfDay::Morning,
            TimeOfDay::Midday => other == TimeOfDay::Morning || other == TimeOfDay::Midday,
            TimeOfDay::Evening => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FloatingEvent {
    pub(crate) date: NaiveDate,
    pub(crate) time_of_day: TimeOfDay,
    pub(crate) goal_relationships: Vec<GoalRelationship>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EventId(pub(crate) u32);

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    BlockEvent(BlockEvent),
    InstantEvent(InstantEvent),
    FloatingEvent(FloatingEvent),
}

impl Event {
    pub fn block_event(
        start: DateTime<Utc>,
        duration: Duration,
        goal_relationships: Vec<GoalRelationship>,
    ) -> Event {
        Event::BlockEvent(BlockEvent {
            start,
            duration,
            goal_relationships,
        })
    }

    pub fn instant_event(time: DateTime<Utc>, goal_relationships: Vec<GoalRelationship>) -> Event {
        Event::InstantEvent(InstantEvent {
            time,
            goal_relationships,
        })
    }

    pub fn floating_event(
        date: NaiveDate,
        time_of_day: TimeOfDay,
        goal_relationships: Vec<GoalRelationship>,
    ) -> Event {
        Event::FloatingEvent(FloatingEvent {
            date,
            time_of_day,
            goal_relationships,
        })
    }

    pub fn goal_relationships(&self) -> &Vec<GoalRelationship> {
        match self {
            Event::BlockEvent(event) => &event.goal_relationships,
            Event::InstantEvent(event) => &event.goal_relationships,
            Event::FloatingEvent(event) => &event.goal_relationships,
        }
    }

    pub fn goal_relationships_mut(&mut self) -> &mut Vec<GoalRelationship> {
        match self {
            Event::BlockEvent(event) => &mut event.goal_relationships,
            Event::InstantEvent(event) => &mut event.goal_relationships,
            Event::FloatingEvent(event) => &mut event.goal_relationships,
        }
    }
}
