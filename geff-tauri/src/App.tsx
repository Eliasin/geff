import { useEffect } from "react";
import {
  formatCommandline,
  handleKeyPressEvent,
  invokeCommand,
  loadCommand,
  useAppDispatch,
  useCommandline,
  useGoalState,
  PopulatedGoal,
} from "./Store";

import "./App.css";

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
  return <div key={key}>{goal.name}</div>;
}

function Goals(): JSX.Element {
  const goals = useGoalState();
  console.log(goals);

  if (goals.type === "loaded") {
    const { populatedGoals, focusedGoals, selectedGoalId } = goals;

    return (
      <div>
        {populatedGoals.map((goal) =>
          Goal({ goal, focusedGoals, selectedGoalId, key: goal.id })
        )}
      </div>
    );
  } else {
    return <div>UNLOADED</div>;
  }
}

function App() {
  const commandline = useCommandline();
  const dispatch = useAppDispatch();

  function dispatchKeyPress(event: KeyboardEvent) {
    dispatch(handleKeyPressEvent(event.key));

    if (event.key === "Enter") {
      dispatch(invokeCommand());
    }
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
        <Goals />
      </div>
      <div className="commandline">{formatCommandline(commandline)}</div>
    </div>
  );
}

export default App;
