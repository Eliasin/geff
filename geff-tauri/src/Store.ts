import {
  configureStore,
  createSlice,
  PayloadAction,
  ThunkAction,
  ThunkDispatch,
} from "@reduxjs/toolkit";
import { invoke } from "@tauri-apps/api/tauri";
import { useDispatch, useSelector } from "react-redux";
import { AnyAction, combineReducers, Dispatch } from "redux";

type CommandlineState = {
  content: string | null;
};

const commandlineSlice = createSlice({
  name: "commandline",
  initialState: { content: null } as CommandlineState,
  reducers: {
    handleKeyPressEvent: (
      state: CommandlineState,
      action: PayloadAction<string>
    ) => {
      const key = action.payload;
      if (state.content === null) {
        if (key === ":") {
          state.content = "";
        }
      } else {
        if (state.content !== null) {
          if (key === "Escape") {
            state.content = null;
          } else if (key === "Backspace" || key === "Delete") {
            if (state.content.length > 1) {
              state.content = state.content.slice(0, state.content.length - 1);
            } else if (state.content.length === 1) {
              state.content = null;
            }
          } else if (key !== "Enter") {
            state.content = state.content + key;
          }
        }
      }
    },
    clear: (state: CommandlineState) => {
      state.content = null;
    },
  },
});

export type AppThunkDispatch = ThunkDispatch<RootState, unknown, AnyAction>;

async function fetchStateThunk(dispatch: AppThunkDispatch) {
  const frontendState: FrontendState | null = await invoke("fetch");

  if (frontendState !== null) {
    console.log(frontendState);
    dispatch(
      load({
        type: "loaded",
        ...frontendState,
      })
    );
  }
}

async function invokeCommandThunk(
  dispatch: AppThunkDispatch,
  getState: typeof store.getState
) {
  await invoke("app_command", {
    command: getState().commandline.content ?? "",
  });

  await fetchStateThunk(dispatch);

  dispatch(clear());
}

export function invokeCommand() {
  return invokeCommandThunk;
}

type FrontendState = {
  populatedGoals: Array<PopulatedGoal>;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
};

export async function loadCommandThunk(dispatch: AppThunkDispatch) {
  await invoke("load");

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

export function useCommandline(): CommandlineState {
  return useSelector((root: RootState) => root.commandline);
}

export const { clear, handleKeyPressEvent } = commandlineSlice.actions;

export function formatCommandline(state: CommandlineState) {
  if (state.content === null) {
    return "";
  } else {
    return ":" + state.content + "|";
  }
}

export type PopulatedGoal = {
  id: number;
  parentGoalId?: number;
  name: string;
  effortToDate: number;
  effortToComplete: number;
  maxChildLayerWidth: number;
  maxChildDepth: number;
  children: Array<PopulatedGoal>;
};

export type GoalStateLoaded = {
  type: "loaded";
  populatedGoals: Array<PopulatedGoal>;
  selectedGoalId?: number;
  focusedGoals: Array<number>;
};

type GoalStateUnloaded = {
  type: "unloaded";
};

export type GoalState = { state: GoalStateLoaded | GoalStateUnloaded };

export const goalSlice = createSlice({
  name: "goal",
  initialState: { state: { type: "unloaded" } } as GoalState,
  reducers: {
    load: (state: GoalState, action: PayloadAction<GoalStateLoaded>) => {
      state.state = { ...action.payload };
    },
  },
});

export const { load } = goalSlice.actions;

export function useGoalState(): GoalStateLoaded | GoalStateUnloaded {
  return useSelector((root: RootState) => root.goal.state);
}

const rootReducer = combineReducers({
  commandline: commandlineSlice.reducer,
  goal: goalSlice.reducer,
});

const store = configureStore({ reducer: rootReducer });

export function useAppDispatch() {
  const dispatch = useDispatch();

  return dispatch as (
    action: AnyAction | ThunkAction<unknown, RootState, unknown, AnyAction>
  ) => void;
}

export type RootState = ReturnType<typeof store.getState>;
export default store;
