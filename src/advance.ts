export type RepeatMode = "off" | "once" | "song" | "queue";
export function advance(
  index: number,
  count: number,
  repeat: RepeatMode,
  autoplay: boolean,
): { index: number | null; repeat: RepeatMode; recommend: boolean } {
  if (index < 0 || count === 0)
    return { index: null, repeat, recommend: false };
  if (repeat === "once" || repeat === "song")
    return {
      index,
      repeat: repeat === "once" ? "off" : repeat,
      recommend: false,
    };
  if (!autoplay && repeat !== "queue")
    return { index: null, repeat, recommend: false };
  if (index + 1 < count) return { index: index + 1, repeat, recommend: false };
  if (repeat === "queue") return { index: 0, repeat, recommend: false };
  return { index: null, repeat, recommend: autoplay };
}
