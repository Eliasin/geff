use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    event::{Event, EventId},
    goal::{Goal, GoalId, PopulatedGoal},
    query::TimeOfDayConfiguration,
};

use self::goal_traversal::{get_root_goals, populate_goal_tree, visit_tree_with_predicate};

pub struct ProfileAndDateTime<'a>(pub &'a mut Profile, pub DateTime<Utc>);

pub mod goal_traversal {
    use std::collections::{HashMap, HashSet};

    use crate::goal::{Goal, GoalId, PopulatedGoal};

    pub type GoalChildIndexPath = Vec<usize>;

    pub fn visit_goal_path_from<V: FnMut(&mut PopulatedGoal, &GoalChildIndexPath)>(
        root: &mut PopulatedGoal,
        path: &GoalChildIndexPath,
        v: &mut V,
    ) {
        let mut current_goal = root;
        let mut current_path = Vec::with_capacity(path.len());

        for index in path {
            v(current_goal, &current_path);

            current_goal = current_goal
                .children
                .get_mut(*index)
                .expect("paths constructed");

            current_path.push(*index);
        }

        v(current_goal, &current_path);
    }

    pub fn visit_populated_goal_children_mut<V, VF>(
        goal: &mut PopulatedGoal,
        v: &mut VF,
        root_visitor_data: V,
    ) where
        VF: FnMut(&GoalChildIndexPath, &V, &GoalChildIndexPath, &mut PopulatedGoal) -> V,
    {
        let mut needs_visiting: Vec<(GoalChildIndexPath, V)> = vec![(vec![], root_visitor_data)];

        while let Some((current_path, current_visitor_data)) = needs_visiting.pop() {
            let current_goal = traverse_populated_goal_children_mut(goal, &current_path)
                .expect("current path to always be valid");

            let children = &mut current_goal.children;
            for child_index in 0..children.len() {
                let child_index_path = {
                    let mut c = current_path.clone();
                    c.push(child_index);
                    c
                };

                let child_goal = current_goal
                    .children
                    .get_mut(child_index)
                    .expect("child index to be valid");

                let child_visitor_data = v(
                    &current_path,
                    &current_visitor_data,
                    &child_index_path,
                    child_goal,
                );

                needs_visiting.push((child_index_path, child_visitor_data));
            }
        }
    }

    pub fn visit_populated_goal_children<V, VF>(
        goal: &PopulatedGoal,
        v: &mut VF,
        root_visitor_data: V,
    ) where
        VF: FnMut(&GoalChildIndexPath, &V, &GoalChildIndexPath, &PopulatedGoal) -> V,
    {
        let mut needs_visiting: Vec<(GoalChildIndexPath, V)> = vec![(vec![], root_visitor_data)];

        while let Some((current_path, current_visitor_data)) = needs_visiting.pop() {
            let current_goal = traverse_populated_goal_children(goal, &current_path)
                .expect("current path to always be valid");

            let children = &current_goal.children;
            for child_index in 0..children.len() {
                let child_index_path = {
                    let mut c = current_path.clone();
                    c.push(child_index);
                    c
                };

                let child_goal = current_goal
                    .children
                    .get(child_index)
                    .expect("child index to be valid");

                let child_visitor_data = v(
                    &current_path,
                    &current_visitor_data,
                    &child_index_path,
                    child_goal,
                );

                needs_visiting.push((child_index_path, child_visitor_data));
            }
        }
    }

    pub fn traverse_populated_goal_children<'a>(
        root_goal: &'a PopulatedGoal,
        goal_child_index_path: &GoalChildIndexPath,
    ) -> Option<&'a PopulatedGoal> {
        let mut current = root_goal;

        for goal_child_index in goal_child_index_path {
            current = current.children.get(*goal_child_index)?;
        }

        Some(current)
    }

    pub fn traverse_populated_goal_children_mut<'a>(
        root_goal: &'a mut PopulatedGoal,
        goal_child_index_path: &GoalChildIndexPath,
    ) -> Option<&'a mut PopulatedGoal> {
        let mut current = root_goal;

        for goal_child_index in goal_child_index_path {
            current = current.children.get_mut(*goal_child_index)?;
        }

        Some(current)
    }

    pub fn get_goal_parent_id(goals: &HashMap<GoalId, Goal>, goal_id: GoalId) -> Option<GoalId> {
        goals
            .iter()
            .find(|(_, goal)| goal.children().contains(&goal_id))
            .map(|(id, _)| *id)
    }

    /// Visit goals in a goal child tree. This function is especially useful for building
    /// a parallel intrinsically connected tree from the flat, ID based internal
    /// representations of [Goal](Goal) in [Profile](super::Profile).
    ///
    /// To facilitate this use case each invocation of the visitor can create an associated
    /// chunk of data of type V that is created through the visitor function invocation and
    /// is passed to the children when they are visited.
    ///
    /// An example use case could be summing the required effort to fully complete goals between
    /// the root of a goal tree and children within the tree.
    ///
    /// ```
    /// # use std::collections::HashMap;
    /// # use goals::profile::goal_traversal::visit_goal_child_tree;
    /// # use goals::goal::{Goal, GoalId};
    /// let mut goals: HashMap<GoalId, Goal> = HashMap::new();
    ///
    /// const ROOT_EFFORT_TO_COMPLETE: u32 = 1;
    /// const LEFT_CHILD_EFFORT_TO_COMPLETE: u32 = 1;
    /// const RIGHT_CHILD_EFFORT_TO_COMPLETE: u32 = 4;
    /// const RIGHT_GRANDCHILD_EFFORT_TO_COMPLETE: u32 = 5;
    ///
    /// let (root_goal_id, mut root_goal) = (GoalId(1), Goal::new("root", ROOT_EFFORT_TO_COMPLETE));
    /// let (left_child_goal_id, mut left_child_goal) = (
    ///     GoalId(2),
    ///     Goal::new("left-child", LEFT_CHILD_EFFORT_TO_COMPLETE),
    /// );
    /// let (right_child_goal_id, mut right_child_goal) = (
    ///     GoalId(3),
    ///     Goal::new("right-child", RIGHT_CHILD_EFFORT_TO_COMPLETE),
    /// );
    /// let (right_grandchild_goal_id, mut right_grandchild_goal) = (
    ///     GoalId(4),
    ///     Goal::new("right-grandchild", RIGHT_GRANDCHILD_EFFORT_TO_COMPLETE),
    /// );
    ///
    /// root_goal.refine(left_child_goal_id, 0);
    /// root_goal.refine(right_child_goal_id, 0);
    /// right_child_goal.refine(right_grandchild_goal_id, 0);
    ///
    /// goals.insert(root_goal_id, root_goal);
    /// goals.insert(left_child_goal_id, left_child_goal);
    /// goals.insert(right_child_goal_id, right_child_goal);
    /// goals.insert(right_grandchild_goal_id, right_grandchild_goal);
    ///
    /// let mut goal_effort_totals: Vec<(GoalId, u32)> = vec![];
    ///
    /// let visited_ids = visit_goal_child_tree(
    ///     &goals,
    ///     root_goal_id,
    ///     &mut |_, parent_effort_total, child_id, child_goal| -> u32 {
    ///         let child_effort_total = parent_effort_total + child_goal.effort_to_complete();
    ///         goal_effort_totals.push((child_id, child_effort_total));
    ///
    ///         child_effort_total
    ///     },
    ///     ROOT_EFFORT_TO_COMPLETE,
    /// )
    /// .expect("root goal to exist");
    ///
    /// // Visitation will skip the root goal
    /// assert!(!visited_ids.contains(&root_goal_id));
    /// assert!(visited_ids.contains(&left_child_goal_id));
    /// assert!(visited_ids.contains(&right_child_goal_id));
    /// assert!(visited_ids.contains(&right_grandchild_goal_id));
    ///
    /// assert!(goal_effort_totals.contains(&(
    ///     left_child_goal_id,
    ///     ROOT_EFFORT_TO_COMPLETE + LEFT_CHILD_EFFORT_TO_COMPLETE
    /// )));
    /// assert!(goal_effort_totals.contains(&(
    ///     right_child_goal_id,
    ///     ROOT_EFFORT_TO_COMPLETE + RIGHT_CHILD_EFFORT_TO_COMPLETE
    /// )));
    /// assert!(goal_effort_totals.contains(&(
    ///     right_grandchild_goal_id,
    ///     ROOT_EFFORT_TO_COMPLETE
    ///         + RIGHT_CHILD_EFFORT_TO_COMPLETE
    ///         + RIGHT_GRANDCHILD_EFFORT_TO_COMPLETE
    /// )));
    /// ```
    pub fn visit_goal_child_tree<V, VF>(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
        goal_visitor: &mut VF,
        root_visitor_data: V,
    ) -> Option<HashSet<GoalId>>
    where
        VF: FnMut(GoalId, &V, GoalId, &Goal) -> V,
    {
        if goals.get(&goal_id).is_some() {
            let mut visited_ids = HashSet::new();

            let mut needs_visiting: Vec<(GoalId, V)> = vec![(goal_id, root_visitor_data)];

            while let Some((current_goal_id, current_visitor_data)) = needs_visiting.pop() {
                let children = goals
                    .get(&current_goal_id)
                    .expect("current goal to be in profile")
                    .children();

                visited_ids.extend(children);

                for child_id in children {
                    let child = goals
                        .get(child_id)
                        .expect("child goal to be in the profile");

                    let child_visitor_data =
                        goal_visitor(current_goal_id, &current_visitor_data, *child_id, child);

                    needs_visiting.push((*child_id, child_visitor_data));
                }
            }

            Some(visited_ids)
        } else {
            None
        }
    }

    pub fn get_root_goals(goals: &HashMap<GoalId, Goal>) -> impl Iterator<Item = GoalId> + '_ {
        let child_goals: HashSet<GoalId> = goals
            .values()
            .flat_map(|goal| goal.children())
            .copied()
            .collect();

        goals
            .keys()
            .filter_map(move |id| (!child_goals.contains(id)).then_some(*id))
    }

    pub fn populated_goal_traversal_template(
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
            max_child_depth: 0,
            max_child_layer_width: 0,
        }
    }

    /// Create a (PopulatedGoal)[PopulatedGoal] value by traversing the child
    /// tree of a goal. Returns an option containing the populated goal value
    /// and the set of child ids in the child tree. Returns None if no goals
    /// were found with the provided `goal_id`.
    pub fn populate_goal_tree(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
    ) -> Option<(PopulatedGoal, HashSet<GoalId>)> {
        if let Some(goal) = goals.get(&goal_id) {
            let parent_goal_id = get_goal_parent_id(goals, goal_id);

            let mut root_populated_goal =
                populated_goal_traversal_template(goal_id, goal, parent_goal_id);

            let mut widths: Vec<usize> = vec![];
            let mut add_node_to_width_vec = |node_depth: usize| {
                let insert_index = node_depth
                    .checked_sub(1)
                    .expect("add_node_to_width_vec to only be called for non root nodes");
                if let Some(width_counter) = widths.get_mut(insert_index) {
                    *width_counter += 1;
                } else {
                    widths.extend((0..(insert_index - widths.len())).map(|_| 0));
                    widths.push(1);
                }
            };

            let ids_visited = visit_goal_child_tree::<GoalChildIndexPath, _>(
                goals,
                goal_id,
                &mut |parent_goal_id: GoalId,
                      parent_index_path: &GoalChildIndexPath,
                      child_id: GoalId,
                      child_goal: &Goal|
                 -> GoalChildIndexPath {
                    let child_populated_goal_template = populated_goal_traversal_template(
                        child_id,
                        child_goal,
                        Some(parent_goal_id),
                    );

                    let current_goal_populated_template = traverse_populated_goal_children_mut(
                        &mut root_populated_goal,
                        parent_index_path,
                    )
                    .expect("goal child index path to be valid");

                    let child_index_path = {
                        let mut c = parent_index_path.clone();
                        c.push(current_goal_populated_template.children.len());
                        c
                    };

                    add_node_to_width_vec(child_index_path.len());

                    current_goal_populated_template
                        .children
                        .push(child_populated_goal_template);

                    if child_goal.children().is_empty() {
                        visit_goal_path_from(
                            &mut root_populated_goal,
                            parent_index_path,
                            &mut |populated_goal, goal_path| {
                                populated_goal.max_child_depth = usize::max(
                                    populated_goal.max_child_depth,
                                    child_index_path.len() - goal_path.len(),
                                );
                            },
                        );
                    }

                    child_index_path
                },
                vec![],
            )
            .expect("goal to be valid since it is checked before calling visit");

            let propagate_max_width_back = &mut || {
                let len = widths.len();
                let mut max = None;
                for width in widths[0..len].iter_mut().rev() {
                    if let Some(max) = max.as_mut() {
                        if *width > *max {
                            *max = *width;
                        } else {
                            *width = *max;
                        }
                    } else {
                        max = Some(*width);
                    }
                }
                // We need to set the width for the root since the visit does not touch the root
                root_populated_goal.max_child_layer_width = widths
                    .first()
                    .copied()
                    .unwrap_or(root_populated_goal.children.len());
            };

            propagate_max_width_back();

            visit_populated_goal_children_mut(
                &mut root_populated_goal,
                &mut |_, _, child_path, child_goal| {
                    child_goal.max_child_layer_width = widths
                        .get(child_path.len())
                        .copied()
                        .unwrap_or(child_goal.children.len());
                },
                (),
            );

            Some((root_populated_goal, ids_visited))
        } else {
            None
        }
    }

    pub struct PartitionedPopulatedTree {
        pub populated_tree: PopulatedGoal,
        pub satisfies_predicate: HashSet<(GoalChildIndexPath, GoalId)>,
        pub does_not_satisfy_predicate: HashSet<(GoalChildIndexPath, GoalId)>,
    }

    /// Create a (PopulatedGoal)[PopulatedGoal] value by traversing the child
    /// tree of a goal while also partitioning the tree using a predicate function.
    /// Returns an option containing the populated goal value, the set of child id and
    /// goal child index path pairs in the child tree that satisfies the predicate and
    /// the set of pairs in the child tree that do not. Returns None if no goals were
    /// found with the provided `goal_id`.
    pub fn populate_partitioned_goal_tree<P>(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
        predicate: &P,
    ) -> Option<PartitionedPopulatedTree>
    where
        P: Fn(GoalId, &Goal) -> bool,
    {
        if let Some(goal) = goals.get(&goal_id) {
            let parent_goal_id = get_goal_parent_id(goals, goal_id);

            let mut passing_children = HashSet::new();
            let mut failing_children = HashSet::new();

            let mut root_populated_goal =
                populated_goal_traversal_template(goal_id, goal, parent_goal_id);

            visit_goal_child_tree::<GoalChildIndexPath, _>(
                goals,
                goal_id,
                &mut |parent_goal_id: GoalId,
                      parent_index_path: &GoalChildIndexPath,
                      child_id: GoalId,
                      child_goal: &Goal|
                 -> GoalChildIndexPath {
                    let child_populated_goal_template = populated_goal_traversal_template(
                        child_id,
                        child_goal,
                        Some(parent_goal_id),
                    );

                    let current_goal_populated_template = traverse_populated_goal_children_mut(
                        &mut root_populated_goal,
                        parent_index_path,
                    )
                    .expect("goal child index path to be valid");

                    let mut child_index_path = parent_index_path.clone();
                    if predicate(child_id, child_goal) {
                        passing_children.insert((child_index_path.clone(), child_id));
                    } else {
                        failing_children.insert((child_index_path.clone(), child_id));
                    }

                    child_index_path.push(current_goal_populated_template.children.len());

                    current_goal_populated_template
                        .children
                        .push(child_populated_goal_template);

                    child_index_path
                },
                vec![],
            )
            .expect("goal to be valid since it is checked before calling visit");

            Some(PartitionedPopulatedTree {
                populated_tree: root_populated_goal,
                satisfies_predicate: passing_children,
                does_not_satisfy_predicate: failing_children,
            })
        } else {
            None
        }
    }

    /// Visit a goal child tree and collect the ids of child/parent goal tuples that satisfy
    /// the given predicate. The predicate is passed the parent's id, whether or not the parent
    /// satisfied the predicate, the child's id and the goal data.
    pub fn visit_tree_with_predicate_and_parent<P>(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
        predicate: &mut P,
        does_root_satisfy_predicate: bool,
    ) -> Option<HashSet<GoalId>>
    where
        P: FnMut(GoalId, bool, GoalId, &Goal) -> bool,
    {
        let mut passing_child_ids = HashSet::new();

        if visit_goal_child_tree(
            goals,
            goal_id,
            &mut |parent_goal_id, parent_satisfied_predicate, child_id, child_goal| -> bool {
                if predicate(
                    parent_goal_id,
                    *parent_satisfied_predicate,
                    child_id,
                    child_goal,
                ) {
                    passing_child_ids.insert(child_id);
                    true
                } else {
                    false
                }
            },
            does_root_satisfy_predicate,
        )
        .is_some()
        {
            Some(passing_child_ids)
        } else {
            None
        }
    }

    /// Visit a goal child tree and collect the ids of child goals that satisfy
    /// the given predicate. The predicate is passed the child's id and goal data.
    pub fn visit_tree_with_predicate<P>(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
        predicate: &mut P,
    ) -> Option<HashSet<GoalId>>
    where
        P: FnMut(GoalId, &Goal) -> bool,
    {
        let mut passing_child_ids = HashSet::new();

        if visit_goal_child_tree(
            goals,
            goal_id,
            &mut |_, _, child_id, child_goal| {
                if predicate(child_id, child_goal) {
                    passing_child_ids.insert(child_id);
                }
            },
            (),
        )
        .is_some()
        {
            Some(passing_child_ids)
        } else {
            None
        }
    }

    /// Partition a goal child tree into a set of child goal ids that satisfies the
    /// predicate and another set of ids where they do not. The predicate is passed
    /// the child goal's id and goal data. The return tuple is in the order
    /// `(satisfies, does not satisfy)`.
    pub fn partition_tree_with_predicate<P>(
        goals: &HashMap<GoalId, Goal>,
        goal_id: GoalId,
        predicate: &mut P,
    ) -> Option<(HashSet<GoalId>, HashSet<GoalId>)>
    where
        P: FnMut(GoalId, &Goal) -> bool,
    {
        let mut passing_child_ids = HashSet::new();
        let mut failing_child_ids = HashSet::new();

        if visit_goal_child_tree(
            goals,
            goal_id,
            &mut |_, _, child_id, child_goal| {
                if predicate(child_id, child_goal) {
                    passing_child_ids.insert(child_id);
                } else {
                    failing_child_ids.insert(child_id);
                }
            },
            (),
        )
        .is_some()
        {
            Some((passing_child_ids, failing_child_ids))
        } else {
            None
        }
    }
}

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
                !self.focused_goals.contains(&child_id)
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
