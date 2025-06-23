import { useState, useCallback } from "react";

export const useActionDelay = (defaultDelayMs: number = 1000) => {
  const [isDelaying, setIsDelaying] = useState(false);

  // Simple delay function that returns a Promise
  const delayAction = useCallback(
    (delayMs: number = defaultDelayMs): Promise<void> => {
      return new Promise((resolve) => {
        setIsDelaying(true);
        setTimeout(() => {
          setIsDelaying(false);
          resolve();
        }, delayMs);
      });
    },
    [defaultDelayMs]
  );

  // Wrapper function that prevents execution if already delaying
  const executeWithDelay = useCallback(
    async <T>(
      action: () => Promise<T> | T,
      delayMs: number = defaultDelayMs
    ): Promise<T | null> => {
      if (isDelaying) {
        return null;
      }

      try {
        const result = await action();
        await delayAction(delayMs);
        return result;
      } catch (error) {
        setIsDelaying(false);
        throw error;
      }
    },
    [isDelaying, delayAction, defaultDelayMs]
  );

  // Debounced action that prevents rapid successive calls
  const debouncedAction = useCallback(
    async <T>(
      action: () => Promise<T> | T,
      debounceMs: number = 300
    ): Promise<T | null> => {
      if (isDelaying) {
        return null;
      }

      setIsDelaying(true);

      return new Promise((resolve) => {
        setTimeout(async () => {
          try {
            const result = await action();
            setIsDelaying(false);
            resolve(result);
          } catch (error) {
            setIsDelaying(false);
            throw error;
          }
        }, debounceMs);
      });
    },
    [isDelaying]
  );

  return {
    isDelaying,
    delayAction,
    executeWithDelay,
    debouncedAction,
  };
};
