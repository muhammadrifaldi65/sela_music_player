import { useTranslation, t as tr, errorText } from "./i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playTrack, playback, type Entry } from "./playback";
export function Queue() {
  const { t: tr, locale } = useTranslation();
  const [entries, setEntries] = useState<Entry[]>([]),
    [error, setError] = useState("");
  const [state, setState] = useState(playback.state);
  const [changing, setChanging] = useState(false);
  const refreshRequest = useRef(0);
  useEffect(() => {
    const update = () => setState(playback.state);
    window.addEventListener("playback-state", update);
    return () => window.removeEventListener("playback-state", update);
  }, []);
  async function refresh() {
    const request = ++refreshRequest.current;
    try {
      const next = await invoke<Entry[]>("queue_entries");
      if (request !== refreshRequest.current) return;
      setEntries(next); setError("");
    } catch (e) {
      if (request === refreshRequest.current) setError(errorText(e));
    }
  }
  useEffect(() => {
    void refresh();
    const changed = () => void refresh();
    window.addEventListener("queue-changed", changed);
    return () => {
      refreshRequest.current++;
      window.removeEventListener("queue-changed", changed);
    };
  }, []);
  async function move(from: number, to: number) {
    if (from === to) return;
    const next = [...entries];
    const [entry] = next.splice(from, 1);
    next.splice(to, 0, entry);
    try {
      await invoke("reorder_queue", { ids: next.map((e) => e.entry_id) });
      window.dispatchEvent(new Event("queue-changed"));
    } catch (e) {
      setError(errorText(e));
    }
  }
  return (
    <section className="queue-panel" aria-label={tr("Antrean")}>
      <h2>{tr("Antrean")}</h2>
      <label className="queue-autoplay">
        {tr("Putar otomatis")}
        <input type="checkbox" role="switch" checked={state?.autoplay ?? false} disabled={!state || changing}
          onChange={async (event) => {
            const autoplay = event.target.checked;
            setChanging(true);
            try {
              await invoke("set_playback_options", { repeat: state?.repeat ?? "off", autoplay });
              setState((value) => value ? { ...value, autoplay } : value);
              setError("");
            } catch (error) { setError(errorText(error)); }
            finally { setChanging(false); }
          }} />
      </label>
      {entries.length === 0 && <p>{tr("Tambahkan lagu dari pencarian.")}</p>}
      <ol>
        {entries.map((e, index) => (
          <li
            key={e.entry_id}
            className={state?.entryId === e.entry_id ? "queue-song is-playing" : "queue-song"}
            aria-current={state?.entryId === e.entry_id ? "true" : undefined}
            draggable
            onDragStart={(event) =>
              event.dataTransfer.setData("text/plain", String(e.entry_id))
            }
            onDragOver={(event) => event.preventDefault()}
            onDrop={(event) => {
              event.preventDefault();
              const from = entries.findIndex(
                (e) =>
                  e.entry_id ===
                  Number(event.dataTransfer.getData("text/plain")),
              );
              if (from >= 0) void move(from, index);
            }}
          >
            <span className="queue-number">{state?.entryId === e.entry_id ? "♪" : index + 1}</span>
            <img className="queue-artwork" src={`https://i.ytimg.com/vi/${encodeURIComponent(e.track.id)}/default.jpg`} alt="" loading="lazy" />
            <span className="queue-metadata"><strong>{e.track.title}</strong><span>{e.track.channel}</span></span>
            <button
              onClick={() =>
                void playTrack(e.track, e.entry_id).catch((e) =>
                  setError(errorText(e)),
                )
              }
            >
              {tr("Putar dari antrean")}
            </button>
            <button
              disabled={index === 0}
              onClick={() => void move(index, index - 1)}
            >
              {tr("Naik")}
            </button>
            <button
              disabled={index === entries.length - 1}
              onClick={() => void move(index, index + 1)}
            >
              {tr("Turun")}
            </button>
            <button
              onClick={async () => {
                try {
                  await invoke("remove_queue_entry", { id: e.entry_id });
                  window.dispatchEvent(new Event("queue-changed"));
                } catch (e) {
                  setError(errorText(e));
                }
              }}
            >
              {tr("Hapus dari antrean")}
            </button>
          </li>
        ))}
      </ol>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
