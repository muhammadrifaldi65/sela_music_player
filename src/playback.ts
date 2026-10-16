import { invoke } from "@tauri-apps/api/core";
import type { RepeatMode } from "./advance";
export type Track = { id: string; title: string; channel?: string | null };
export type Entry = { entry_id: number; track: Track };
export type PlaybackState = {
  position: number | null;
  duration: number | null;
  volume: number;
  paused: boolean;
  ended: boolean;
  loading?: boolean;
  error?: string | null;
  track?: Track | null;
  entryId?: number | null;
  repeat?: RepeatMode;
  autoplay?: boolean;
  mode?: "audio" | "video";
  historyRevision?: number;
};
export const playback = {
  current: null as Entry | null,
  selected: null as Track | null,
  loading: false,
  requesting: false,
  revision: 0,
  mode: "audio" as "audio" | "video",
  state: null as PlaybackState | null,
};
export const platform = invoke<string>("platform");
async function startEntry(entry: Entry, request: number) {
  const mobile = (await platform) === "android";
  if (request !== playback.revision) return;
  await invoke(
    "play",
    mobile
      ? { videoId: entry.track.id, entryId: entry.entry_id }
      : {
          videoId: entry.track.id,
          entryId: entry.entry_id,
          track: entry.track,
        },
  );
  if (request !== playback.revision) return;
  if (mobile) {
    await invoke("set_paused", { paused: false });
    if (request !== playback.revision) return;
    await invoke("select_queue_entry", { id: entry.entry_id });
  }
  if (request === playback.revision) {
    playback.current = entry;
    if (mobile) window.dispatchEvent(new Event("history-changed"));
  }
}
export async function playPlaylist(playlistId: number) {
  const request = ++playback.revision;
  playback.requesting = true;
  playback.loading = true;
  playback.selected = null;
  window.dispatchEvent(new Event("playback-changed"));
  try {
    const entries = await invoke<Entry[]>("replace_queue_from_playlist", {
      playlistId,
    });
    window.dispatchEvent(new Event("queue-changed"));
    if (request !== playback.revision) return;
    if (!entries.length) throw new Error("Playlist tidak memiliki lagu.");
    playback.selected = entries[0].track;
    window.dispatchEvent(new Event("playback-changed"));
    await startEntry(entries[0], request);
  } catch (error) {
    if (request === playback.revision) throw error;
  } finally {
    if (request === playback.revision) {
      playback.requesting = false;
      playback.loading = false;
      playback.selected = null;
      window.dispatchEvent(new Event("playback-changed"));
    }
  }
}
export async function playTrack(track: Track, entryId?: number) {
  const request = ++playback.revision;
  playback.requesting = true;
  playback.loading = true;
  playback.selected = track;
  window.dispatchEvent(new Event("playback-changed"));
  try {
    const id =
      entryId ??
      (await invoke<number>("enqueue", {
        track: { ...track, channel: track.channel ?? null },
      }));
    window.dispatchEvent(new Event("queue-changed"));
    if (request === playback.revision)
      await startEntry({ entry_id: id, track }, request);
  } catch (error) {
    if (request === playback.revision) throw error;
  } finally {
    if (request === playback.revision) {
      playback.requesting = false;
      playback.loading = false;
      playback.selected = null;
      window.dispatchEvent(new Event("playback-changed"));
    }
  }
}
export async function changePlayback(
  command: "playback_previous" | "playback_next" | "set_video_mode",
  mode?: "audio" | "video",
) {
  const request = ++playback.revision;
  playback.requesting = true;
  playback.loading = true;
  playback.selected = null;
  window.dispatchEvent(new Event("playback-changed"));
  try {
    await invoke(command, mode ? { mode } : undefined);
  } catch (error) {
    if (request === playback.revision) throw error;
  } finally {
    if (request === playback.revision) {
      playback.requesting = false;
      playback.loading = false;
      window.dispatchEvent(new Event("playback-changed"));
    }
  }
}
