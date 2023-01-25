import { ThunkAction, ThunkDispatch } from "@reduxjs/toolkit";
import { invoke } from "@tauri-apps/api/tauri";
import { AnyAction } from "redux";
import {
  displayError,
  DisplayState,
  handleKeyPressEvent,
  load,
  PopulatedGoal,
  RootGetState,
  RootState,
  update as updateDisplay,
} from "./Store";

export type AppThunkDispatch = ThunkDispatch<RootState, unknown, AnyAction>;

async function fetchStateThunk(dispatch: AppThunkDispatch) {
  const frontendState: FrontendState | null = await invoke("fetch");

  if (frontendState !== null) {
    dispatch(
      load({
        type: "loaded",
        populatedGoals: frontendState.populatedGoals,
        selectedGoalId: frontendState.selectedGoalId,
        focusedGoals: frontendState.focusedGoals,
      })
    );

    dispatch(updateDisplay(frontendState.config.display));
  }
}

export type CursorAction = "up" | "down" | "in" | "out";

function cursorAction(action: CursorAction) {
  async function cursorActionThunk(dispatch: AppThunkDispatch) {
    await invoke("cursor_action", {
      cursorAction: action,
    });

    await fetchStateThunk(dispatch);
  }

  return cursorActionThunk;
}

function invokeCommand(command: string) {
  async function invokeCommandThunk(dispatch: AppThunkDispatch) {
    const error = await invoke("app_command", {
      command,
    })
      .then(() => null)
      .catch((e) => JSON.stringify(e));

    if (error !== null) {
      dispatch(displayError({ error }));
    } else {
      await fetchStateThunk(dispatch);
    }
  }

  return invokeCommandThunk;
}

type FrontendConfig = {
  display: DisplayState;
};

type FrontendState = {
  populatedGoals: Array<PopulatedGoal>;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
  config: FrontendConfig;
};

export async function loadCommandThunk(dispatch: AppThunkDispatch) {
  const error = await invoke("load")
    .then(() => null)
    .catch((e) => e);

  if (error !== null) {
    dispatch(displayError(error));
  }

  await fetchStateThunk(dispatch);
}

export function loadCommand(): ThunkAction<
  void,
  RootState,
  unknown,
  AnyAction
> {
  return loadCommandThunk;
}

export function keyboardEvent(event: KeyboardEvent) {
  async function keyboardEventThunk(
    dispatch: AppThunkDispatch,
    getState: RootGetState
  ) {
    const commandlineState = getState().commandline;
    if (event.key === "Enter") {
      if (commandlineState.state.type === "typing") {
        dispatch(invokeCommand(commandlineState.state.content));
      }
    }

    dispatch(handleKeyPressEvent(event.key));

    if (commandlineState.state.type === "typing") {
      switch (event.key) {
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
