import { useEffect, useState } from "react";
import {
  formatCommandline,
  useAppDispatch,
  useCommandline,
  useCommandlineDisplayState,
} from "./Store";

import "./App.scss";
import { keyboardEvent, loadCommand } from "./Event";
import { RootGoals } from "./RootGoals";

function StatusBar(): JSX.Element {
  const [date, setDate] = useState(new Date());

  useEffect(() => {
    const intervalId = setInterval(() => {
      setDate(new Date());
    }, 1000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [setDate]);

  return <div className="status-bar">{date.toString()}</div>;
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
        <StatusBar />
        <RootGoals />
      </div>
      <Commandline />
    </div>
  );
}

export default App;
