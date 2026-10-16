export function Icon({
  name,
}: {
  name: "play" | "pause" | "next" | "previous" | "volume" | "queue";
}) {
  const paths = {
    play: "M8 5v14l11-7z",
    pause: "M7 5h3v14H7z M14 5h3v14h-3z",
    next: "M5 5v14l10-7z M17 5h2v14h-2z",
    previous: "M19 5v14L9 12z M5 5h2v14H5z",
    volume: "M3 9h4l5-4v14l-5-4H3z M16 8q5 4 0 8",
    queue: "M4 6h16 M4 12h16 M4 18h10",
  };
  return (
    <svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true">
      <path
        d={paths[name]}
        fill={name === "queue" || name === "volume" ? "none" : "currentColor"}
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
    </svg>
  );
}
