import {
  useGoalState,
  PopulatedGoal,
  useCommandlineDisplayState,
} from "./Store";
import StarIcon from "@mui/icons-material/Star";

import "./App.css";

function colorFromGoalDepth(depth: number, finished: boolean): string {
  if (finished) {
    return "#77d991";
  }

  switch (depth) {
    case 0: {
      return "#0ba334";
    }
    case 1: {
      return "#048226";
    }
  }

  return "#02611b";
}

function GoalStatusIndicator({
  goal,
  selectedGoalId,
  focusedGoals,
}: {
  goal: PopulatedGoal;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
}): JSX.Element | null {
  const isSelected = selectedGoalId !== null && selectedGoalId === goal.id;
  const isFocused = focusedGoals.includes(goal.id);

  const focusedToken = String.fromCodePoint(0x2605);

  if (isSelected && isFocused) {
    return (
      <div className="goal-status">
        <StarIcon />
      </div>
    );
  } else if (isSelected) {
    return <div className="goal-status"></div>;
  } else if (isFocused) {
    return <div className="goal-status">{focusedToken}</div>;
  } else {
    return <div className="goal-status"></div>;
  }
}

function Goal({
  goal,
  selectedGoalId,
  focusedGoals,
  key,
  depth,
}: {
  goal: PopulatedGoal;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
  key: number;
  depth: number;
}): JSX.Element {
  const isSelected = selectedGoalId !== null && selectedGoalId === goal.id;
  const hasChildren = goal.children.length > 0;

  const progressText =
    "(" + goal.effortToDate + "/" + goal.effortToComplete + ")";

  return (
    <div
      className={"goal"}
      style={{
        backgroundColor: isSelected
          ? "#0262b0"
          : colorFromGoalDepth(
              depth,
              goal.effortToDate >= goal.effortToComplete
            ),
      }}
      key={key}
    >
      <div
        className="goal-info"
        style={{ marginRight: hasChildren ? "4px" : undefined }}
      >
        <GoalStatusIndicator
          goal={goal}
          selectedGoalId={selectedGoalId}
          focusedGoals={focusedGoals}
        />
        <div className="goal-name">{goal.name}</div>
        <div className="goal-progress">{progressText}</div>
      </div>
      <Goals
        goals={goal.children}
        selectedGoalId={selectedGoalId}
        focusedGoals={focusedGoals}
        depth={depth + 1}
      />
    </div>
  );
}

function Goals({
  goals,
  selectedGoalId,
  focusedGoals,
  depth,
}: {
  goals: Array<PopulatedGoal>;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
  depth: number;
}): JSX.Element {
  return (
    <div className="goals">
      {goals.map((goal) =>
        Goal({ goal, focusedGoals, selectedGoalId, key: goal.id, depth })
      )}
    </div>
  );
}

export function RootGoals(): JSX.Element {
  const commandlineDisplay = useCommandlineDisplayState();

  const goals = useGoalState();
  const { fontSizePixels } = commandlineDisplay;

  if (goals.type === "loaded") {
    const { populatedGoals, focusedGoals, selectedGoalId } = goals;
    return (
      <div
        className="root-goals"
        style={{ paddingBottom: fontSizePixels + "px" }}
      >
        {populatedGoals.map((goal) =>
          Goal({ goal, focusedGoals, selectedGoalId, key: goal.id, depth: 0 })
        )}
      </div>
    );
  } else {
    return <div>UNLOADED</div>;
  }
}
