export function discoveryChoice(key: string, choices: readonly string[]): string {
  let index = Math.floor(Math.random() * choices.length);
  try {
    const previous = sessionStorage.getItem(`discovery:${key}`);
    const value = previous === null ? -1 : Number(previous);
    index = Number.isInteger(value) && value >= 0 ? (value + 1) % choices.length : 0;
    sessionStorage.setItem(`discovery:${key}`, String(index));
  } catch {
    // Storage unavailable: discovery still works without remembering the previous choice.
  }
  return choices[index];
}
