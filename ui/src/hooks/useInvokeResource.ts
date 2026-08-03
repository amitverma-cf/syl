import { useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useInvokeResource<T>(
  command: string,
  set: (value: T) => void,
  onError: (message: string) => void,
): () => void {
  const refresh = useCallback(() => {
    invoke<T>(command)
      .then(set)
      .catch((err) => onError(String(err)));
  }, [command, set, onError]);
  useEffect(refresh, [refresh]);
  return refresh;
}
