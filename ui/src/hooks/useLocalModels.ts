import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { LocalModelInfo, SystemStats } from "../types";

const STATS_POLL_INTERVAL_MS = 5000;

export function useLocalModels(onError: (message: string) => void) {
  const [localModels, setLocalModels] = useState<LocalModelInfo[]>([]);
  const [stats, setStats] = useState<SystemStats | null>(null);

  function refresh() {
    invoke<LocalModelInfo[]>("list_local_models")
      .then(setLocalModels)
      .catch((err) => onError(String(err)));
  }

  useEffect(refresh, []);

  useEffect(() => {
    function poll() {
      invoke<SystemStats>("system_stats")
        .then(setStats)
        .catch((err) => onError(String(err)));
      refresh();
    }
    poll();
    const interval = setInterval(poll, STATS_POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, []);

  return { localModels, stats, refresh };
}
