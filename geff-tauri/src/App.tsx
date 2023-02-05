import { useEffect } from "react";
import {
  formatCommandline,
  useAppDispatch,
  useCommandline,
  useGoalState,
  PopulatedGoal,
  useCommandlineDisplayState,
} from "./Store";

import "./App.css";
import { keyboardEvent, loadCommand } from "./Event";

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
    return <div className="goal-status">{focusedToken}</div>;
  } else if (isSelected) {
    return <div className="goal-status">{focusedToken}</div>;
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
}: {
  goal: PopulatedGoal;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
  key: number;
}): JSX.Element {
  const isSelected = selectedGoalId !== null && selectedGoalId === goal.id;
  const hasChildren = goal.children.length > 0;

  const progressText =
    "(" + goal.effortToDate + "/" + goal.effortToComplete + ")";

  return (
    <div className={isSelected ? "goal selected-goal" : "goal"} key={key}>
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
      />
    </div>
  );
}

function Goals({
  goals,
  selectedGoalId,
  focusedGoals,
}: {
  goals: Array<PopulatedGoal>;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
}): JSX.Element {
  return (
    <div className="goals">
      {goals.map((goal) =>
        Goal({ goal, focusedGoals, selectedGoalId, key: goal.id })
      )}
    </div>
  );
}

function RootGoals(): JSX.Element {
  const goals = useGoalState();

  if (goals.type === "loaded") {
    const { populatedGoals, focusedGoals, selectedGoalId } = goals;
    return (
      <div className="root-goals">
        {populatedGoals.map((goal) =>
          Goal({ goal, focusedGoals, selectedGoalId, key: goal.id })
        )}
      </div>
    );
  } else {
    return <div>UNLOADED</div>;
  }
}

function Commandline(): JSX.Element {
  const commandline = useCommandline();
  const commandlineDisplay = useCommandlineDisplayState();

  const { backgroundColor, fontSizePixels, fontColor } = commandlineDisplay;

  return (
    <div
      style={{
        backgroundColor,
        fontSize: fontSizePixels + "px",
        color: fontColor,
      }}
      className="commandline"
    >
      {formatCommandline(commandline)}
    </div>
  );
}

function App() {
  const dispatch = useAppDispatch();

  function dispatchKeyPress(event: KeyboardEvent) {
    dispatch(keyboardEvent(event));
  }

  useEffect(() => {
    dispatch(loadCommand());
  }, []);

  useEffect(() => {
    window.addEventListener("keypress", dispatchKeyPress);

    return () => {
      window.removeEventListener("keypress", dispatchKeyPress);
    };
  }, [dispatchKeyPress, dispatch]);

  return (
    <div className="app">
      <div className="main">
        <RootGoals />
      </div>
      <Commandline />
    </div>
  );
}

export default App;
