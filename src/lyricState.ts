import { invoke } from "@tauri-apps/api/core";
import { useSyncExternalStore } from "react";
import { playback } from "./playback";

type LyricResult = {
  syncedLyrics?: string | null;
  plainLyrics?: string | null;
  instrumental?: boolean;
};
type NativeState = {
  trackId: string;
  status: "idle" | "loading" | "ready" | "unavailable" | "error";
  result: LyricResult | null;
  error: string | null;
};
export type LyricLine = { time: number; text: string };
type Snapshot = NativeState & { lines: LyricLine[] };
const listeners = new Set<() => void>();
let snapshot: Snapshot = {
  trackId: "",
  status: "idle",
  result: null,
  error: null,
  lines: [],
};
let initialized = false,
  key = "",
  generation = 0,
  busy = false,
  timer: number | undefined,
  lastDuration: number | null = null;
export function parseLyrics(text: string): LyricLine[] {
  const lines: LyricLine[] = [];
  for (const line of text.split("\n")) {
    const stamps = [...line.matchAll(/\[(\d+):(\d+(?:\.\d+)?)\]/g)];
    const text = line.replace(/\[\d+:\d+(?:\.\d+)?\]/g, "").trim();
    for (const stamp of stamps) {
      const seconds = Number(stamp[2]);
      if (seconds < 60)
        lines.push({ time: Number(stamp[1]) * 60 + seconds, text });
    }
  }
  return lines.sort((a, b) => a.time - b.time);
}
function publish(state: NativeState) {
  snapshot = {
    ...state,
    lines:
      state.result?.syncedLyrics === snapshot.result?.syncedLyrics
        ? snapshot.lines
        : parseLyrics(state.result?.syncedLyrics ?? ""),
  };
  for (const listener of listeners) listener();
}
async function refresh() {
  if (busy || !snapshot.trackId) return;
  busy = true;
  const request = generation,
    trackId = snapshot.trackId;
  try {
    const duration =
      playback.state?.track?.id === trackId &&
      !playback.loading &&
      !playback.state.loading
        ? playback.state.duration
        : null;
    const state = await invoke<NativeState>("lyric_state", {
      trackId,
      duration,
    });
    if (request === generation && state.trackId === trackId) publish(state);
  } catch {
    if (request === generation)
      publish({ trackId, status: "error", error: "provider", result: null });
  } finally {
    busy = false;
    if (request !== generation) {
      void refresh();
      return;
    }
    if (snapshot.status === "loading" || snapshot.status === "idle")
      timer = window.setTimeout(() => void refresh(), 1000);
    else if (snapshot.status === "error" || snapshot.status === "unavailable") {
      timer = window.setTimeout(
        () => {
          if (request !== generation) return;
          const track = playback.selected ?? playback.current?.track;
          if (!track || track.id !== trackId) return;
          void invoke("prepare_lyrics", {
            trackId,
            title: track.title,
            artist: track.channel ?? "",
          })
            .then(() => {
              if (request === generation) void refresh();
            })
            .catch(() => {
              if (request === generation) void refresh();
            });
        },
        snapshot.status === "unavailable" ? 301000 : 61000,
      );
    }
  }
}
function update() {
  const track = playback.selected ?? playback.current?.track;
  const next = track
    ? JSON.stringify([track.id, track.title, track.channel ?? ""])
    : "";
  if (next !== key) {
    key = next;
    generation++;
    lastDuration = null;
    clearTimeout(timer);
    publish({
      trackId: track?.id ?? "",
      status: track ? "loading" : "idle",
      result: null,
      error: null,
    });
    if (!track) return;
    const request = generation;
    // Only command dispatch is awaited, never provider I/O or media preparation.
    void invoke("prepare_lyrics", {
      trackId: track.id,
      title: track.title,
      artist: track.channel ?? "",
    })
      .then(() => {
        if (request === generation) void refresh();
      })
      .catch(() => {
        if (request === generation)
          publish({
            trackId: track.id,
            status: "error",
            error: "provider",
            result: null,
          });
      });
  } else if (
    track &&
    playback.state?.track?.id === track.id &&
    !playback.loading &&
    !playback.state.loading
  ) {
    const duration = playback.state.duration;
    if (duration && duration !== lastDuration) {
      lastDuration = duration;
      clearTimeout(timer);
      void refresh();
    }
  }
}
export function initializeLyrics() {
  if (initialized) return;
  initialized = true;
  window.addEventListener("playback-changed", update);
  window.addEventListener("playback-state", update);
  update();
}
export function useLyrics() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => snapshot,
  );
}
