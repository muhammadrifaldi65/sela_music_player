import { useTranslation, errorText } from "./i18n";
import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playback, platform, changePlayback } from "./playback";
import { Lyrics } from "./Lyrics";
import { Queue } from "./Queue";
import { Icon } from "./Icon";
import "./PlayerLyrics.css";
export function NowPlaying() {
  const { t: tr } = useTranslation();
  const [tab, setTab] = useState<"queue" | "lyrics">("lyrics");
  const [phase, setPhase] = useState<"closed" | "entering" | "open" | "closing">("closed"),
    [, refresh] = useState(0),
    [desktop, setDesktop] = useState(false),
    [switching, setSwitching] = useState(false),
    [error, setError] = useState("");
  const back = useRef<HTMLButtonElement>(null),
    surface = useRef<HTMLDivElement>(null),
    panel = useRef<HTMLElement>(null);
  const surfaceWrites = useRef(Promise.resolve());
  const open = phase !== "closed";
  const track = playback.selected ?? playback.current?.track;
  useLayoutEffect(() => {
    if (phase !== "entering" && phase !== "closing") return;
    let active = true;
    // Follow the CSS duration, including reduced motion; ignore interrupted runs.
    const animations = panel.current?.getAnimations() ?? [];
    void Promise.allSettled(animations.map((animation) => animation.finished)).then(() => {
      if (active) setPhase(phase === "closing" ? "closed" : "open");
    });
    return () => { active = false; };
  }, [phase]);
  useEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    back.current?.focus();
    const shell = document.querySelector<HTMLElement>(".shell");
    if (shell) shell.inert = true;
    const overflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !document.querySelector("dialog[open]")) {
        event.preventDefault();
        setPhase((value) => value === "closed" ? value : "closing");
      }
    };
    window.addEventListener("keydown", key);
    return () => {
      window.removeEventListener("keydown", key);
      if (shell) shell.inert = false;
      document.body.style.overflow = overflow;
      previous?.focus();
    };
  }, [open]);
  useEffect(() => {
    void platform
      .then((value) => setDesktop(value !== "android"))
      .catch((e) => setError(errorText(e)));
    const show = () => {
      if (playback.current || playback.selected) {
        setPhase((value) => value === "closed" || value === "closing" ? "entering" : "closing");
      }
    };
    const changed = () => refresh((v) => v + 1);
    window.addEventListener("toggle-now-playing", show);
    window.addEventListener("playback-changed", changed);
    return () => {
      window.removeEventListener("toggle-now-playing", show);
      window.removeEventListener("playback-changed", changed);
    };
  }, []);
  useLayoutEffect(() => {
    if (!desktop || phase !== "open" || !surface.current) return;
    let frame = 0,
      alive = true,
      busy = false;
    type Bounds = {
      x: number;
      y: number;
      width: number;
      height: number;
      visible: boolean;
    };
    let pending: Bounds | null = null;
    const send = async () => {
      if (busy) return;
      busy = true;
      try {
        while (pending) {
          const bounds = pending;
          pending = null;
          surfaceWrites.current = surfaceWrites.current.then(async () => {
            if (!alive) return;
            try {
              await invoke("set_video_bounds", bounds);
            } catch (e) {
              if (alive) setError(errorText(e));
            }
          });
          await surfaceWrites.current;
        }
      } finally {
        busy = false;
      }
    };
    const measure = () => {
      frame = 0;
      const rect = surface.current?.getBoundingClientRect(),
        clip = panel.current?.getBoundingClientRect();
      if (!rect || !clip) return;
      const fullyVisible =
        rect.top >= clip.top &&
        rect.bottom <= clip.bottom &&
        rect.left >= 0 &&
        rect.right <= window.innerWidth;
      const obscured = !!document.querySelector(
        "dialog[open],.queue-container.open",
      );
      pending = {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
        visible:
          playback.mode === "video" &&
          !document.hidden &&
          fullyVisible &&
          !obscured,
      };
      void send();
    };
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(measure);
    };
    const observer = new ResizeObserver(schedule);
    observer.observe(surface.current);
    if (panel.current) observer.observe(panel.current);
    const overlays = new MutationObserver(schedule);
    overlays.observe(document.body, {
      subtree: true,
      attributes: true,
      attributeFilter: ["class", "open"],
      childList: true,
    });
    window.addEventListener("playback-changed", schedule);
    window.addEventListener("resize", schedule);
    window.addEventListener("scroll", schedule, true);
    document.addEventListener("visibilitychange", schedule);
    window.visualViewport?.addEventListener("resize", schedule);
    window.visualViewport?.addEventListener("scroll", schedule);
    schedule();
    return () => {
      alive = false;
      cancelAnimationFrame(frame);
      observer.disconnect();
      overlays.disconnect();
      window.removeEventListener("playback-changed", schedule);
      window.removeEventListener("resize", schedule);
      window.removeEventListener("scroll", schedule, true);
      document.removeEventListener("visibilitychange", schedule);
      window.visualViewport?.removeEventListener("resize", schedule);
      window.visualViewport?.removeEventListener("scroll", schedule);
      pending = null;
      surfaceWrites.current = surfaceWrites.current.then(async () => {
        try {
          await invoke("set_video_bounds", {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            visible: false,
          });
        } catch (e) {
          setError(errorText(e));
        }
      });
    };
  }, [desktop, phase, !!track]);
  if (!open || !track) return null;
  const artwork = `https://i.ytimg.com/vi/${track.id}/hqdefault.jpg`;
  return (
    <section
      ref={panel}
      className={`expanded-player is-${phase}`}
      style={{ "--artwork": `url("${artwork}")` } as CSSProperties}
      aria-label={tr("Sedang diputar")}
      role="region"
    >
      <div className="expanded-header">
        <button ref={back} className="back-player" onClick={() => setPhase("closing")}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>{tr("Kembali")}
        </button>
        <span>{tr("Sedang diputar")}</span>
        <span className="expanded-brand">Sela</span>
      </div>
      <div className="expanded-columns">
        <div className="expanded-art">
          {desktop && (
            <div
              className="media-mode"
              role="group"
              aria-label={tr("Mode pemutaran")}
            >
              {(["audio", "video"] as const).map((mode) => (
                <button
                  key={mode}
                  aria-pressed={playback.mode === mode}
                  disabled={switching || playback.loading}
                  onClick={async () => {
                    setSwitching(true);
                    setError("");
                    try {
                      await changePlayback("set_video_mode", mode);
                    } catch (e) {
                      setError(errorText(e));
                    } finally {
                      setSwitching(false);
                    }
                  }}
                >
                  {mode === "audio" ? tr("Audio") : tr("Video")}
                </button>
              ))}
            </div>
          )}
          <div
            ref={surface}
            className={`media-surface ${playback.mode === "video" ? "video-mode" : ""}`}
            aria-label={
              playback.mode === "video" ? tr("Video lagu") : tr("Artwork lagu")
            }
          >
            <img
              src={artwork}
              alt={tr("Artwork {title}", { title: track.title })}
            />
          </div>
          {(switching || playback.loading) && (
            <p role="status" className="media-status">
              {tr("Menyiapkan media…")}
            </p>
          )}
          {error && <p role="alert">{error}</p>}
          <h2>{track.title}</h2>
          <p>{track.channel}</p>
        </div>
        <aside className="player-sidebar">
          <div className="player-tabs" role="group" aria-label={tr("Tampilan pemutar")}>
            <button aria-pressed={tab === "queue"} onClick={() => setTab("queue")}><Icon name="queue" />{tr("Antrean")}</button>
            <button aria-pressed={tab === "lyrics"} onClick={() => setTab("lyrics")}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15a3 3 0 0 0 3-3V5a3 3 0 0 0-6 0v7a3 3 0 0 0 3 3Zm-6-4v1a6 6 0 0 0 12 0v-1M12 18v4m-4 0h8" /></svg>{tr("Lirik")}</button>
          </div>
          {tab === "lyrics" ? <Lyrics key={track.id} /> : <Queue />}
        </aside>
      </div>
    </section>
  );
}
