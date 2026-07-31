import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useInvokeResource<T>(
  command: string,
  set: (value: T) => void,
  onError: (message: string) => void,
): () => void {
  function refresh() {
    invoke<T>(command)
      .then(set)
      .catch((err) => onError(String(err)));
  }
  useEffect(refresh, []);
  return refresh;
}
