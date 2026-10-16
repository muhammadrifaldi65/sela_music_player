import type { Track } from "./playback";

// Round-robin, skipping duplicates before advancing to the next genre.
export function balanceTracks(groups: Track[][], limit = 24): Track[] {
  const positions = groups.map(() => 0),
    seen = new Set<string>(),
    result: Track[] = [];
  while (result.length < limit) {
    let added = false;
    for (
      let index = 0;
      index < groups.length && result.length < limit;
      index++
    ) {
      const group = groups[index];
      while (
        positions[index] < group.length &&
        seen.has(group[positions[index]].id)
      )
        positions[index]++;
      const track = group[positions[index]++];
      if (track) {
        seen.add(track.id);
        result.push(track);
        added = true;
      }
    }
    if (!added) break;
  }
  return result;
}
