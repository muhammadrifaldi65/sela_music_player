import { useTranslation, t as tr, errorText } from "./i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  playback,
  playTrack,
  changePlayback,
  type Entry,
  type PlaybackState,
} from "./playback";
import { Icon } from "./Icon";
import type { RepeatMode } from "./advance";
import { createVolumeWriter, type VolumeWriter } from "./volume";
function clock(seconds: number | null) {
  if (seconds === null) return "0:00";
  return `${Math.floor(seconds / 60)}:${Math.floor(seconds % 60)
    .toString()
    .padStart(2, "0")}`;
}
export function Transport() {
  const { t: tr, locale } = useTranslation();
  const [state, setState] = useState<PlaybackState>({
    position: null,
    duration: null,
    volume: 100,
    paused: true,
    ended: false,
  });
  const [entries, setEntries] = useState<Entry[]>([]),
    [error, setError] = useState(""),
    [, render] = useState(0);
  const [repeat, setRepeat] = useState<RepeatMode>("off"),
    [autoplay, setAutoplay] = useState(true),
    [nativeReady, setNativeReady] = useState(false);
  const [volume, setVolume] = useState(100),
    [skipping, setSkipping] = useState(false);
  const dragging = useRef(false);
  const optionRevision = useRef(0),
    optionPending = useRef(false);
  const [changingOptions, setChangingOptions] = useState(false);
  const volumeWriter = useRef<VolumeWriter | null>(null);
  if (!volumeWriter.current)
    volumeWriter.current = createVolumeWriter(
      (value) => invoke("set_volume", { volume: value }),
      (e) => setError(errorText(e)),
    );
  useEffect(() => {
    let active = true,
      pending = false;
    let queueRevision = 0,
      historyRevision: number | undefined;
    const changed = () => {
      const request = ++queueRevision;
      render((n) => n + 1);
      void invoke<Entry[]>("queue_entries")
        .then((rows) => {
          if (active && request === queueRevision) setEntries(rows);
        })
        .catch((e) => {
          if (active && request === queueRevision) setError(errorText(e));
        });
    };
    changed();
    window.addEventListener("playback-changed", changed);
    window.addEventListener("queue-changed", changed);
    const poll = async () => {
      if (pending) return;
      pending = true;
      const writer = volumeWriter.current!;
      const revision = writer.revision;
      const volumeBusy = writer.busy;
      const requesting = playback.requesting;
      const selectionRevision = playback.revision;
      const optionsVersion = optionRevision.current;
      const optionsBusy = optionPending.current;
      try {
        const next = await invoke<PlaybackState>("playback_state");
        if (
          !active ||
          requesting ||
          playback.requesting ||
          selectionRevision !== playback.revision
        )
          return;
        setState(next);
        playback.state = next;
        if (
          next.historyRevision !== undefined &&
          next.historyRevision !== historyRevision
        ) {
          historyRevision = next.historyRevision;
          window.dispatchEvent(new Event("history-changed"));
        }
        if (
          !dragging.current &&
          !volumeBusy &&
          !writer.busy &&
          writer.revision === revision
        )
          setVolume(next.volume);
        if (
          !optionsBusy &&
          !optionPending.current &&
          optionsVersion === optionRevision.current
        ) {
          if (next.repeat) setRepeat(next.repeat);
          if (next.autoplay !== undefined) setAutoplay(next.autoplay);
        }
        setNativeReady(true);
        if (
          !requesting &&
          !playback.requesting &&
          selectionRevision === playback.revision
        ) {
          const changedEntry =
            (playback.current?.entry_id ?? null) !== (next.entryId ?? null);
          const changedLoading = playback.loading !== !!next.loading;
          const changedMode = !!next.mode && playback.mode !== next.mode;
          playback.current =
            next.track && next.entryId != null
              ? { entry_id: next.entryId, track: next.track }
              : null;
          playback.loading = !!next.loading;
          if (next.mode) playback.mode = next.mode;
          if (changedEntry || changedLoading || changedMode)
            window.dispatchEvent(new Event("playback-changed"));
          if (changedEntry) {
            if (next.historyRevision === undefined)
              window.dispatchEvent(new Event("history-changed"));
            window.dispatchEvent(new Event("queue-changed"));
          }
        }
        window.dispatchEvent(new Event("playback-state"));
      } catch (e) {
        if (active && selectionRevision === playback.revision)
          setError(errorText(e));
      } finally {
        pending = false;
      }
    };
    void poll();
    const timer = setInterval(() => void poll(), 500);
    return () => {
      active = false;
      clearInterval(timer);
      window.removeEventListener("playback-changed", changed);
      window.removeEventListener("queue-changed", changed);
    };
  }, []);
  async function options(nextRepeat: RepeatMode, nextAutoplay: boolean) {
    if (optionPending.current) return;
    optionPending.current = true;
    setChangingOptions(true);
    const request = ++optionRevision.current;
    try {
      await invoke("set_playback_options", {
        repeat: nextRepeat,
        autoplay: nextAutoplay,
      });
      if (request !== optionRevision.current) return;
      setRepeat(nextRepeat);
      setAutoplay(nextAutoplay);
    } catch (e) {
      if (request === optionRevision.current) setError(errorText(e));
    } finally {
      if (request === optionRevision.current) {
        optionPending.current = false;
        setChangingOptions(false);
      }
    }
  }
  async function skip(direction: number) {
    setSkipping(true);
    setError("");
    try {
      await changePlayback(
        direction < 0 ? "playback_previous" : "playback_next",
      );
    } catch (e) {
      setError(errorText(e));
    } finally {
      setSkipping(false);
    }
  }
  const index = entries.findIndex(
    (e) => e.entry_id === playback.current?.entry_id,
  );
  const current = playback.selected ?? playback.current?.track;
  const repeatLabels: Record<RepeatMode, string> = {
    off: tr("Ulang mati"),
    once: tr("Ulang sekali"),
    song: tr("Ulang lagu terus"),
    queue: tr("Ulang antrean"),
  };
  const failure = error || state.error;
  return (
    <footer
      className="player-bar"
      aria-label={tr("Kontrol pemutar")}
      aria-busy={playback.loading}
    >
      <button
        className="now-playing"
        aria-label={tr("Buka artwork dan lirik")}
        disabled={!current}
        onClick={() => window.dispatchEvent(new Event("toggle-now-playing"))}
      >
        {current ? (
          <img
            src={`https://i.ytimg.com/vi/${current.id}/mqdefault.jpg`}
            alt=""
          />
        ) : (
          <div className="empty-art">
            <Icon name="volume" />
          </div>
        )}
        <div>
          <strong>{current?.title ?? tr("Temukan lagu berikutnya")}</strong>
          <span role={playback.loading ? "status" : undefined}>
            {playback.loading
              ? tr("Menyiapkan media…")
              : (current?.channel ?? "Sela · By PGM Community")}
          </span>
        </div>
      </button>
      <div className="transport">
        <div className="transport-buttons">
          <button
            className="icon-button"
            aria-label={tr("Lagu sebelumnya")}
            title={tr("Lagu sebelumnya")}
            disabled={!current || skipping}
            onClick={() => void skip(-1)}
          >
            <Icon name="previous" />
          </button>
          <button
            className="play-button"
            aria-label={
              state.paused || state.position === null ? tr("Putar") : tr("Jeda")
            }
            title={tr("Putar / jeda")}
            disabled={playback.loading || (!current && state.position === null)}
            onClick={async () => {
              try {
                setError("");
                if (
                  (state.position === null || state.ended || state.error) &&
                  playback.current
                )
                  await playTrack(
                    playback.current.track,
                    playback.current.entry_id,
                  );
                else await invoke("set_paused", { paused: !state.paused });
              } catch (e) {
                setError(errorText(e));
              }
            }}
          >
            <Icon
              name={state.paused || state.position === null ? "play" : "pause"}
            />
          </button>
          <button
            className="icon-button"
            aria-label={tr("Lagu berikutnya")}
            title={tr("Lagu berikutnya")}
            disabled={
              !current ||
              skipping ||
              (!autoplay && repeat !== "queue" && index >= entries.length - 1)
            }
            onClick={() => void skip(1)}
          >
            <Icon name="next" />
          </button>
          <button
            className="repeat-button"
            aria-label={tr(repeatLabels[repeat])}
            title={tr("{label} — klik untuk ganti", {
              label: tr(repeatLabels[repeat]),
            })}
            aria-pressed={repeat !== "off"}
            disabled={!nativeReady || changingOptions}
            onClick={() => {
              const modes: RepeatMode[] = ["off", "once", "song", "queue"];
              void options(
                modes[(modes.indexOf(repeat) + 1) % modes.length],
                autoplay,
              );
            }}
          >
            <svg
              width="23"
              height="23"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.7"
              aria-hidden="true"
            >
              <path d="M4 10V8a3 3 0 0 1 3-3h13m-4-3 4 3-4 3M20 14v2a3 3 0 0 1-3 3H4m4-3-4 3 4 3" />
            </svg>
            <span>
              {repeat === "once"
                ? "1×"
                : repeat === "song"
                  ? "∞"
                  : repeat === "queue"
                    ? "≡"
                    : ""}
            </span>
          </button>
        </div>
        <div className="timeline">
          <span>{clock(state.position)}</span>
          <input
            aria-label={tr("Posisi lagu")}
            type="range"
            min="0"
            max={state.duration ?? 0}
            step="1"
            value={state.position ?? 0}
            disabled={!state.duration || playback.loading}
            onChange={(e) =>
              void invoke("seek", { seconds: Number(e.target.value) }).catch(
                (e) => setError(errorText(e)),
              )
            }
          />
          <span>{clock(state.duration)}</span>
        </div>
      </div>
      <div className="player-extras">
        <Icon name="volume" />
        <input
          aria-label={tr("Volume")}
          aria-valuetext={`${volume}%`}
          type="range"
          min="0"
          max="100"
          value={volume}
          onPointerDown={(e) => {
            dragging.current = true;
            e.currentTarget.setPointerCapture(e.pointerId);
          }}
          onPointerUp={() => {
            dragging.current = false;
          }}
          onPointerCancel={() => {
            dragging.current = false;
          }}
          onLostPointerCapture={() => {
            dragging.current = false;
          }}
          onBlur={() => {
            dragging.current = false;
          }}
          onChange={(e) => {
            const value = Number(e.target.value);
            setVolume(value);
            volumeWriter.current!.set(value);
          }}
        />
        <button
          className="icon-button"
          aria-label={tr("Lihat antrean")}
          title={tr("Lihat antrean")}
          onClick={() => window.dispatchEvent(new Event("toggle-queue"))}
        >
          <Icon name="queue" />
        </button>
        <div className="playback-options">
          <label>
            <input
              type="checkbox"
              checked={autoplay}
              disabled={!nativeReady || changingOptions}
              onChange={(e) => void options(repeat, e.target.checked)}
            />{" "}
            Autoplay
          </label>
        </div>
      </div>
      {failure && (
        <p role="alert" className="player-error">
          {error || errorText(state.error)}
          {error && (
            <button aria-label={tr("Tutup pesan")} onClick={() => setError("")}>
              ×
            </button>
          )}
        </p>
      )}
    </footer>
  );
}
