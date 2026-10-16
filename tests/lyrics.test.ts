import { strict as assert } from "node:assert";
import { registerHooks } from "node:module";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
registerHooks({ resolve(specifier, context, nextResolve) {
  return nextResolve(specifier === "./playback" ? "./playback.ts" : specifier, context);
} });
const events = new EventTarget();
Object.assign(globalThis, { window: Object.assign(events, { setTimeout }) });
const selection = Promise.withResolvers<void>();
const reads: string[] = [];
mockIPC((command, args) => {
  if (command === "platform") return "windows";
  if (command === "select_lyric_provider") return selection.promise;
  if (command === "lyric_state") {
    reads.push(args.trackId as string);
    return { trackId: args.trackId, status: "ready", result: null, error: null };
  }
});
// IPC must exist before playback's module-level platform request; exercise that loading boundary.
const { playback } = await import("../src/playback.ts");
const { initializeLyrics, selectLyricProvider, parseLyrics } = await import("../src/lyricState.ts");
const settle = () => new Promise<void>((resolve) => setImmediate(resolve));
function select(id: string) {
  playback.selected = { id, title: id };
  window.dispatchEvent(new Event("playback-changed"));
}
select("first");
initializeLyrics();
await settle();
const pending = selectLyricProvider("lrclib");
select("second");
await settle();
select("first");
await settle();
const before = reads.length;
selection.resolve();
await pending;
await settle();
assert.equal(reads.length, before, "Obsolete provider completion must not refresh a revisited track");
assert.deepEqual(parseLyrics("[00:02.50]Second\n[00:01][00:03]Repeat\n[00:61]Invalid"), [
  { time: 1, text: "Repeat" }, { time: 2.5, text: "Second" }, { time: 3, text: "Repeat" },
]);
clearMocks();
console.log("Lyrics reject obsolete provider completion and preserve real LRC timestamps");
