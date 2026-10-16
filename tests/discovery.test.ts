import assert from "node:assert/strict";
import { discoveryChoice } from "../src/discoveryRotation.ts";
const values = new Map<string, string>();
Object.defineProperty(globalThis, "sessionStorage", { configurable: true, value: {
  getItem: (key: string) => values.get(key) ?? null,
  setItem: (key: string, value: string) => values.set(key, value),
} });
assert.equal(discoveryChoice("pop", ["audio", "live", "new"]), "audio");
assert.equal(discoveryChoice("pop", ["audio", "live", "new"]), "live");
assert.equal(discoveryChoice("rock", ["audio", "live", "new"]), "audio");
assert.equal(discoveryChoice("pop", ["audio", "live", "new"]), "new");
assert.equal(discoveryChoice("pop", ["audio", "live", "new"]), "audio");
values.set("discovery:pop", "corrupt");
assert.equal(discoveryChoice("pop", ["audio", "live"]), "audio");
Object.defineProperty(globalThis, "sessionStorage", { get() { throw new Error("blocked"); } });
assert.ok(["audio", "live"].includes(discoveryChoice("pop", ["audio", "live"])));
console.log("Discovery rotates per source, wraps, and tolerates blocked or corrupt storage");
