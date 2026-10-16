import { useEffect, useRef, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playback } from "./playback";
import { useLyrics } from "./lyricState";
import { useTranslation } from "./i18n";
const errorMessages: Record<string, string> = {
  metadata: "Metadata belum cukup untuk mencari lirik.",
  response: "Respons lirik tidak valid.",
  cooldown: "Penyedia lirik meminta jeda. Coba lagi nanti.",
};
export function Lyrics() {
  const { t } = useTranslation();
  const lyrics = useLyrics();
  const [position, setPosition] = useState(playback.state?.position ?? 0),
    [seekError, setSeekError] = useState("");
  const activeRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    const update = () => setPosition(playback.state?.position ?? 0);
    window.addEventListener("playback-state", update);
    return () => window.removeEventListener("playback-state", update);
  }, []);
  useEffect(() => setSeekError(""), [lyrics.trackId]);
  const { lines, result } = lyrics;
  const message =
    lyrics.status === "idle"
      ? t("Putar lagu untuk melihat lirik.")
      : lyrics.status === "loading"
        ? t("Mencari lirik…")
        : lyrics.status === "unavailable"
          ? t("Lirik tidak tersedia untuk lagu ini.")
          : lyrics.status === "error"
            ? t(
                errorMessages[lyrics.error ?? ""] ??
                  "Penyedia lirik sedang tidak tersedia.",
              )
            : result?.instrumental
              ? t("Instrumental")
              : "";
  let active = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].time <= position) active = i;
    else break;
  }
  useEffect(() => {
    const line = activeRef.current;
    const panel = line?.parentElement;
    if (!line || !panel) return;
    const top =
      line.getBoundingClientRect().top -
      panel.getBoundingClientRect().top +
      panel.scrollTop -
      panel.clientHeight / 2 +
      line.clientHeight / 2;
    panel.scrollTo({
      top: Math.max(0, top),
      behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "auto"
        : "smooth",
    });
  }, [active]);
  const track = playback.selected ?? playback.current?.track;
  return (
    <section
      className="lyrics-panel"
      style={
        track
          ? ({
              "--artwork": `url("https://i.ytimg.com/vi/${track.id}/hqdefault.jpg")`,
            } as CSSProperties)
          : undefined
      }
      aria-label={t("Lirik")}
    >
      <h2>{t("Lirik")}</h2>
      <p className="lyrics-source">{t("Sumber: LRCLIB")}</p>
      {message && <p role="status">{message}</p>}
      {seekError && (
        <p role="alert">
          {t("Gagal memindahkan posisi pemutaran.")} {seekError}
        </p>
      )}
      {lines.length ? (
        <div className="lyrics-scroll">
          {lines.map((line, index) => (
            <button
              key={index}
              ref={index === active ? activeRef : undefined}
              aria-current={index === active ? "true" : undefined}
              className={index === active ? "lyric active" : "lyric"}
              onClick={() =>
                void invoke("seek", { seconds: line.time })
                  .then(() => setSeekError(""))
                  .catch((e) => setSeekError(String(e)))
              }
            >
              {line.text || "♪"}
            </button>
          ))}
        </div>
      ) : (
        <p className="plain-lyrics">{result?.plainLyrics ?? ""}</p>
      )}
    </section>
  );
}
