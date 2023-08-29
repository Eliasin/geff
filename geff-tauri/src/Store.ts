import {
  configureStore,
  createSlice,
  PayloadAction,
  ThunkAction,
} from "@reduxjs/toolkit";
import { useDispatch, useSelector } from "react-redux";
import { AnyAction, combineReducers } from "redux";

type CommandlineTyping = { type: "typing"; content: string };
type CommandlineError = { type: "error"; error: string };
type CommandlineEmpty = { type: "empty" };

type CommandlineStore = {
  state: CommandlineTyping | CommandlineError | CommandlineEmpty;
};

const commandlineSlice = createSlice({
  name: "commandline",
  initialState: { state: { type: "empty" } } as CommandlineStore,
  reducers: {
    handleKeyPressEvent: (
      store: CommandlineStore,
      action: PayloadAction<string>
    ) => {
      const key = action.payload;
      if (store.state.type === "empty" || store.state.type === "error") {
        if (key === ":") {
          store.state = { type: "typing", content: ":" };
        } else if (key === "Escape") {
          store.state = { type: "empty" };
        }
      } else {
        if (store.state !== null && store.state.type === "typing") {
          if (key === "Escape") {
            store.state = { type: "empty" };
          } else if (key === "Backspace" || key === "Delete") {
            if (store.state.content.length > 1) {
              store.state.content = store.state.content.slice(
                0,
                store.state.content.length - 1
              );
            } else if (store.state.content.length === 1) {
              store.state = { type: "empty" };
            }
          } else if (key === "Enter") {
            store.state = { type: "empty" };
          } else {
            store.state.content = store.state.content + key;
          }
        }
      }
    },
    displayError: (
      store: CommandlineStore,
      action: PayloadAction<{ error: string }>
    ) => {
      const { error } = action.payload;
      store.state = { type: "error", error };
    },
  },
});

export function useCommandline(): CommandlineStore {
  return useSelector((root: RootState) => root.commandline);
}

export function formatCommandline(state: CommandlineStore) {
  if (state.state.type === "empty") {
    return "";
  } else if (state.state.type === "typing") {
    return state.state.content + "|";
  } else if (state.state.type === "error") {
    return state.state.error;
  }
}

export const { handleKeyPressEvent, displayError } = commandlineSlice.actions;

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

const goalSlice = createSlice({
  name: "goal",
  initialState: { state: { type: "unloaded" } } as GoalState,
  reducers: {
    load: (state: GoalState, action: PayloadAction<GoalStateLoaded>) => {
      state.state = { ...action.payload };
    },
  },
});

export function useGoalState(): GoalStateLoaded | GoalStateUnloaded {
  return useSelector((root: RootState) => root.goal.state);
}

export const { load } = goalSlice.actions;

export type ActiveActivity = "Goals" | "Help";

export type ActivityState = { activeActivity: ActiveActivity };

const activitySlice = createSlice({
  name: "activity",
  initialState: { activeActivity: "Goals" } as ActivityState,
  reducers: {
    setActiveActivity: (
      state: ActivityState,
      action: PayloadAction<ActiveActivity>
    ) => {
      state.activeActivity = action.payload;
    },
  },
});

export function useActiveActivity(): ActiveActivity {
  return useSelector((root: RootState) => root.activity.activeActivity);
}

export const { setActiveActivity } = activitySlice.actions;

export type CommandlineDisplayState = {
  fontSizePixels: number;
  backgroundColor: string;
  fontColor: string;
};

export type DisplayState = {
  commandline: CommandlineDisplayState;
};

type DisplayStore = {
  state: DisplayState;
};

const displaySlice = createSlice({
  name: "display",
  initialState: {
    state: {
      commandline: {
        fontSizePixels: 14,
        backgroundColor: "gray",
        fontColor: "black",
      },
    },
  } as DisplayStore,
  reducers: {
    update: (state: DisplayStore, action: PayloadAction<DisplayState>) => {
      state.state = action.payload;
    },
  },
});

export function useCommandlineDisplayState(): CommandlineDisplayState {
  return useSelector((root: RootState) => root.display.state.commandline);
}

export const { update } = displaySlice.actions;

const rootReducer = combineReducers({
  commandline: commandlineSlice.reducer,
  goal: goalSlice.reducer,
  display: displaySlice.reducer,
  activity: activitySlice.reducer,
});

const store = configureStore({ reducer: rootReducer });

export function useAppDispatch() {
  const dispatch = useDispatch();

  return dispatch as (
    action: AnyAction | ThunkAction<unknown, RootState, unknown, AnyAction>
  ) => void;
}

export type RootState = ReturnType<typeof store.getState>;
export type RootGetState = typeof store.getState;
export default store;
