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
  let prefix = "";

  if (selectedGoalId !== null && selectedGoalId === goal.id) {
    prefix += "*";
  }
  if (focusedGoals.includes(goal.id)) {
    prefix += "F";
  }
  if (prefix !== "") {
    prefix += " ";
  }

  const progressText =
    "(" + goal.effortToDate + "/" + goal.effortToComplete + ")";

  return (
    <div>
      <div key={key}>{prefix + goal.name + " " + progressText}</div>
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
    <div>
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
      <Goals
        goals={populatedGoals}
        selectedGoalId={selectedGoalId}
        focusedGoals={focusedGoals}
      />
    );
  } else {
    return <div>UNLOADED</div>;
  }
}

function Commandline(): JSX.Element {
  const commandline = useCommandline();
  const commandlineDisplay = useCommandlineDisplayState();

  const { backgroundColor, fontSizePixels, fontColor } = commandlineDisplay;

  console.log(backgroundColor);

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
