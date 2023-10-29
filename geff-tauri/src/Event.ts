import { ThunkDispatch } from "@reduxjs/toolkit";
import { invoke } from "@tauri-apps/api/tauri";
import { AnyAction } from "redux";
import {
  ActiveActivity,
  displayError,
  DisplayState,
  handleKeyPressEvent,
  load,
  PopulatedGoal,
  RootGetState,
  RootState,
  setActiveActivity,
  update as updateDisplay,
} from "./Store";

export type RootThunkDispatch = ThunkDispatch<RootState, unknown, AnyAction>;

function wrapErrorHandler(
  target: (dispatch: RootThunkDispatch) => Promise<unknown>,
  options?: { fetchStateAfter: boolean }
): (dispatch: RootThunkDispatch) => Promise<void> {
  return async function (dispatch: RootThunkDispatch) {
    const error = await target(dispatch)
      .then(() => null)
      .catch((e) => JSON.stringify(e));

    if (error !== null) {
      dispatch(displayError({ error }));
    } else {
      const fetchStateAfterError = options?.fetchStateAfter ?? true;
      if (fetchStateAfterError) {
        dispatch(fetchState());
      }
    }
  };
}

function fetchState() {
  async function fetchStateThunk(dispatch: RootThunkDispatch) {
    const frontendState: FrontendState | null = await invoke("fetch");

    if (frontendState !== null) {
      const goalState = frontendState.goalState;
      dispatch(
        load({
          type: "loaded",
          populatedGoals: goalState.populatedGoals,
          selectedGoalId: goalState.selectedGoalId,
          focusedGoals: goalState.focusedGoals,
        })
      );

      console.debug("Fetched frontend state");
      console.debug(goalState);

      dispatch(updateDisplay(goalState.config.display));

      dispatch(setActiveActivity(frontendState.activeActivity));
    }
  }

  return wrapErrorHandler(fetchStateThunk, { fetchStateAfter: false });
}

export type CursorAction = "up" | "down" | "in" | "out";

function cursorAction(action: CursorAction) {
  async function cursorActionThunk(dispatch: RootThunkDispatch) {
    await invoke("cursor_action", {
      cursorAction: action,
    });

    dispatch(fetchState());
  }

  return wrapErrorHandler(cursorActionThunk);
}

function invokeSetActiveActivity(activeActivity: ActiveActivity) {
  async function invokeSetActiveActivityThunk() {
    await invoke("set_active_activity", {
      newActiveActivity: activeActivity,
    });
  }

  return wrapErrorHandler(invokeSetActiveActivityThunk);
}

function invokeAppCommand(command: string) {
  async function invokeAppCommandThunk() {
    const result = await invoke("app_command", {
      command,
    });
    console.debug(`Invoke '${command}' returned ${JSON.stringify(result)}`);

    return result;
  }

  return wrapErrorHandler(invokeAppCommandThunk);
}

type FrontendConfig = {
  display: DisplayState;
};

type FrontendState = {
  goalState: {
    populatedGoals: Array<PopulatedGoal>;
    selectedGoalId?: number;
    focusedGoals: Array<number>;
    config: FrontendConfig;
  };
  activeActivity: ActiveActivity;
};

async function loadCommandThunk(dispatch: RootThunkDispatch) {
  const error = await invoke("load")
    .then(() => null)
    .catch((e) => e);

  if (error !== null) {
    dispatch(displayError(error));
  }

  dispatch(fetchState());
}

export function loadCommand() {
  return loadCommandThunk;
}

export function keyboardEvent(event: KeyboardEvent) {
  async function keyboardEventThunk(
    dispatch: RootThunkDispatch,
    getState: RootGetState
  ) {
    const commandlineState = getState().commandline;
    if (event.key === "Enter") {
      if (commandlineState.state.type === "typing") {
        dispatch(invokeAppCommand(commandlineState.state.content));
      }
    }

    dispatch(handleKeyPressEvent(event.key));

    if (commandlineState.state.type !== "typing") {
      switch (event.key) {
        case "q": {
          const activeActivity = getState().activity.activeActivity;
          if (activeActivity === "Help") {
            dispatch(invokeSetActiveActivity("Goals"));
          }
        }
        case "h": {
          dispatch(cursorAction("out"));
          break;
        }
        case "j": {
          dispatch(cursorAction("down"));
          break;
        }
        case "k": {
          dispatch(cursorAction("up"));
          break;
        }
        case "l": {
          dispatch(cursorAction("in"));
          break;
        }
      }
    }
  }

  return keyboardEventThunk;
}
