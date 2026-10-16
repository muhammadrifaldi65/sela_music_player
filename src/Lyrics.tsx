import { useEffect, useId, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playback } from "./playback";
import { useLyrics, selectLyricProvider, searchLyricsOverride } from "./lyricState";
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
  const [sourcesOpen, setSourcesOpen] = useState(false);
  const [offset, setOffset] = useState(0);
  const [pending, setPending] = useState(false);
  const [sourceError, setSourceError] = useState("");
  const [title, setTitle] = useState("");
  const [artist, setArtist] = useState("");
  const activeRef = useRef<HTMLButtonElement>(null);
  const sourceButton = useRef<HTMLButtonElement>(null);
  const sourceMenuId = useId();
  const mounted = useRef(false);
  const working = useRef(false);
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; };
  }, []);
  async function changeSource(action: () => Promise<void>) {
    if (working.current || !lyrics.trackId) return;
    working.current = true;
    setPending(true); setSourceError("");
    try { await action(); }
    catch (error) { if (mounted.current) setSourceError(String(error)); }
    finally {
      working.current = false;
      if (mounted.current) setPending(false);
    }
  }
  useEffect(() => {
    const update = () => setPosition(playback.state?.position ?? 0);
    window.addEventListener("playback-state", update);
    return () => window.removeEventListener("playback-state", update);
  }, []);
  useEffect(() => {
    setSeekError(""); setOffset(0); setSourceError("");
    setTitle(playback.selected?.title ?? playback.current?.track.title ?? "");
    setArtist(playback.selected?.channel ?? playback.current?.track.channel ?? "");
  }, [lyrics.trackId]);
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
  const track = playback.selected ?? playback.current?.track;
  const canSeek = track?.id === lyrics.trackId && playback.state?.track?.id === lyrics.trackId && !playback.loading && !playback.state?.loading;
  let active = -1;
  for (let i = 0; i < lines.length; i++) {
    if (canSeek && lines[i].time + offset <= position) active = i;
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
  }, [active, lines]);
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
      <h2 className="sr-only">{t("Lirik")}</h2>
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
              disabled={!canSeek}
              onClick={() =>
                void invoke("seek", { seconds: Math.max(0, line.time + offset) })
                  .then(() => { if (mounted.current) setSeekError(""); })
                  .catch((e) => { if (mounted.current) setSeekError(String(e)); })
              }
            >
              {line.text || "♪"}
            </button>
          ))}
        </div>
      ) : (
        <p className="plain-lyrics">{result?.plainLyrics ?? ""}</p>
      )}
      <footer className="lyrics-footer">
        <button ref={sourceButton} aria-expanded={sourcesOpen} aria-controls={sourceMenuId} disabled={!lyrics.trackId} onClick={() => setSourcesOpen(!sourcesOpen)}>
          {t("Sumber lirik")}: {lyrics.providers?.find((p) => p.id === lyrics.resolvedProvider)?.name ?? lyrics.resolvedProvider ?? t("Otomatis")} · {lines.length ? t("Sinkron") : t("Tanpa sinkronisasi")}
        </button>
        {sourcesOpen && <section id={sourceMenuId} className="lyrics-source-menu" aria-label={t("Sumber lirik")} aria-busy={pending}
          onKeyDown={(event) => {
            if (event.key !== "Escape") return;
            event.preventDefault(); event.stopPropagation();
            setSourcesOpen(false); sourceButton.current?.focus();
          }}>
          <h3>{t("Sumber lirik")}</h3>
          <p>{t("Pilihan berlaku untuk lagu ini. Otomatis mengutamakan lirik sinkron.")}</p>
          {[{ id: "automatic", name: t("Otomatis"), status: "", synced: false }, ...(lyrics.providers ?? [])].map((provider) => (
            <div className="lyric-provider" key={provider.id}>
              <label><input type="radio" name={`lyric-provider-${sourceMenuId}`} value={provider.id}
                checked={(lyrics.selectedProvider ?? "automatic") === provider.id} disabled={pending}
                onChange={() => void changeSource(() => selectLyricProvider(provider.id))} />{provider.name}</label>
              <small>{provider.synced ? t("Sinkron") : provider.status === "loading" ? t("Mencari…") : provider.status === "unavailable" ? t("Tidak ditemukan") : provider.status === "error" ? t("Tidak terjangkau") : provider.status === "ready" ? t("Tanpa sinkronisasi") : ""}</small>
              {(provider.status === "error" || provider.status === "unavailable") && <button disabled={pending} aria-label={`${t("Coba lagi")}: ${provider.name}`} onClick={() => void changeSource(() => selectLyricProvider(provider.id, true))}>{t("Coba lagi")}</button>}
            </div>
          ))}
          <label className="lyric-timing">{t("Timing (detik)")}
            <input aria-label={t("Timing (detik)")} type="number" min="-30" max="30" step="0.1" value={offset}
              onChange={(event) => { const value = event.target.valueAsNumber; if (Number.isFinite(value)) setOffset(Math.max(-30, Math.min(30, value))); }} />
          </label>
          <p>{t("Nilai positif menunda lirik agar sesuai vokal.")}</p>
          <details className="lyric-correction"><summary>{t("Lagu salah? Cari berdasarkan judul")}</summary>
            <form onSubmit={(event) => {
              event.preventDefault();
              if (title.trim() && artist.trim()) void changeSource(() => searchLyricsOverride(title.trim(), artist.trim()));
            }}>
              <input aria-label={t("Judul lagu")} placeholder={t("Judul lagu")} value={title} maxLength={500} required onChange={(event) => setTitle(event.target.value)} />
              <input aria-label={t("Artis")} placeholder={t("Artis")} value={artist} maxLength={500} required onChange={(event) => setArtist(event.target.value)} />
              <button disabled={pending || !title.trim() || !artist.trim()}>{t("Cari lirik")}</button>
            </form>
          </details>
          {sourceError && <p role="alert">{sourceError}</p>}
        </section>}
      </footer>
    </section>
  );
}
