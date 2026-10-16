import { strict as assert } from "node:assert";
import { advance } from "../src/advance.ts";
assert.deepEqual(advance(0, 2, "once", false), {
  index: 0,
  repeat: "off",
  recommend: false,
});
assert.deepEqual(advance(0, 2, "off", false), {
  index: null,
  repeat: "off",
  recommend: false,
});
assert.deepEqual(advance(1, 2, "queue", false), {
  index: 0,
  repeat: "queue",
  recommend: false,
});
assert.deepEqual(advance(1, 2, "song", true), {
  index: 1,
  repeat: "song",
  recommend: false,
});
assert.deepEqual(advance(1, 2, "off", true), {
  index: null,
  repeat: "off",
  recommend: true,
});
assert.deepEqual(advance(0, 2, "off", true), {
  index: 1,
  repeat: "off",
  recommend: false,
});
assert.deepEqual(advance(1, 2, "off", false), {
  index: null,
  repeat: "off",
  recommend: false,
});
assert.deepEqual(advance(1, 2, "queue", true), {
  index: 0,
  repeat: "queue",
  recommend: false,
});
console.log("Repeat and autoplay boundaries passed");
