import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "../services/api";
import type { Clip } from "../types";

/** Clip list for a search query, kept fresh by backend change events. */
export function useClips(query: string) {
  const [clips, setClips] = useState<Clip[]>([]);

  const refresh = useCallback(async () => {
    try {
      setClips(await api.listClips(query));
    } catch (e) {
      console.error("list_clips failed", e);
    }
  }, [query]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const unlisten = listen("clips-changed", () => void refresh());
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refresh]);

  return { clips, refresh };
}

/** Subscribes to a backend event for the lifetime of the component. */
export function useTauriEvent(name: string, handler: () => void) {
  useEffect(() => {
    const unlisten = listen(name, handler);
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [name, handler]);
}
