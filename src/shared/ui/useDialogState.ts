import {
  useCallback,
  useId,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";

/** Conditional dialog controllers get a fresh React session even while their
 * previous material exits. Drafts/secret fields must not reappear on reopen. */
export function useDialogState<T>(
  initial: T | null = null,
): [T | null, Dispatch<SetStateAction<T | null>>, string] {
  const id = useId();
  const [state, setState] = useState({ value: initial, revision: 0 });
  const setValue = useCallback<Dispatch<SetStateAction<T | null>>>((update) => {
    setState((previous) => {
      const value =
        typeof update === "function"
          ? (update as (value: T | null) => T | null)(previous.value)
          : update;
      if (Object.is(value, previous.value)) return previous;
      return {
        value,
        revision:
          value !== null && value !== previous.value
            ? previous.revision + 1
            : previous.revision,
      };
    });
  }, []);
  return [state.value, setValue, `${id}:${state.revision}`];
}
