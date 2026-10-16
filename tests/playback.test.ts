import { strict as assert } from "node:assert";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
const events = new EventTarget();
Object.assign(globalThis, { window: events });
const enqueue = Promise.withResolvers<number>();
const skipped = Promise.withResolvers<void>();
const played = Promise.withResolvers<void>();
const playStarted = Promise.withResolvers<void>();
const commands: string[] = [];
mockIPC((command) => {
  commands.push(command);
  if (command === "platform") return "windows";
  if (command === "enqueue") return enqueue.promise;
  if (command === "playback_next") return skipped.promise;
  if (command === "play") {
    playStarted.resolve();
    return played.promise;
  }
  return undefined;
});
// Playback resolves platform at module initialization; install IPC before testing that boundary.
const { playTrack, changePlayback, playback } =
  await import("../src/playback.ts");
const first = playTrack({ id: "first", title: "First" });
assert.equal(playback.selected?.id, "first");
const next = changePlayback("playback_next");
enqueue.resolve(1);
await first;
assert.equal(
  commands.includes("play"),
  false,
  "Pending enqueue must not play after a newer skip",
);
assert.equal(
  playback.loading,
  true,
  "Stale completion must not clear newer intent loading",
);
skipped.resolve();
await next;
assert.equal(playback.loading, false);
const obsolete = playTrack({ id: "old", title: "Old" }, 2);
await playStarted.promise;
const latest = changePlayback("playback_previous");
played.reject(new Error("obsolete load failure"));
await obsolete;
await latest;
assert.equal(playback.current, null, "Obsolete play must not select its entry");
assert.equal(playback.requesting, false);
clearMocks();
console.log(
  "Playback intents supersede pending selection and suppress obsolete failures",
);
