import { useTranslation, t as tr, errorText } from "./i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playTrack } from "./playback";
type Track = { id: string; title: string; channel?: string };
import { PlaylistPicker } from "./PlaylistPicker";
export function Search() {
  const { t: tr, locale } = useTranslation();
  const [query, setQuery] = useState(""),
    [tracks, setTracks] = useState<Track[]>([]),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [current, setCurrent] = useState("");
  const [adding, setAdding] = useState<Track | null>(null),
    [notice, setNotice] = useState("");
  const revision = useRef(0),
    playing = useRef(0);
  useEffect(() => {
    const id = ++revision.current;
    if (!query.trim()) {
      setTracks([]);
      setBusy(false);
      setError("");
      return;
    }
    setBusy(true);
    const timer = setTimeout(() => {
      invoke<{ entries?: Track[] }>("search", { query })
        .then((r) => {
          if (id === revision.current) {
            setTracks(r.entries ?? []);
            setError("");
          }
        })
        .catch((e) => {
          if (id === revision.current) setError(errorText(e));
        })
        .finally(() => {
          if (id === revision.current) setBusy(false);
        });
    }, 450);
    return () => {
      clearTimeout(timer);
      revision.current++;
    };
  }, [query]);
  async function play(track: Track) {
    const request = ++playing.current;
    try {
      setCurrent(tr("Memuat {title}", { title: track.title }));
      await playTrack(track);
      if (request === playing.current) {
        setCurrent(track.title);
        setError("");
      }
    } catch (e) {
      if (request === playing.current) {
        setCurrent("");
        setError(errorText(e));
      }
    }
  }
  return (
    <section aria-label={tr("Cari musik")}>
      <div className="discovery-heading">
        <span>{tr("Koleksi suara, tanpa batas suasana.")}</span>
        <h2>{tr("Putar duniamu.")}</h2>
        <p>{tr("Cari lagu favorit. Simpan penemuan baru. Nikmati waktumu.")}</p>
      </div>
      <label htmlFor="search">{tr("Cari musik di YouTube")}</label>
      <input
        id="search"
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={tr("Judul lagu atau artis")}
        style={{ width: "100%" }}
      />
      {busy && <p role="status">{tr("Mencari…")}</p>}
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {current && <p role="status">{current}</p>}
      {query.trim() && !busy && !error && !tracks.length && (
        <p>{tr("Tidak ada hasil. Coba judul atau artis lain.")}</p>
      )}
      <div className="track-grid">
        {tracks.map((t) => (
          <article key={t.id}>
            <img
              width="96"
              alt=""
              src={`https://i.ytimg.com/vi/${encodeURIComponent(t.id)}/mqdefault.jpg`}
            />
            <div style={{ flex: 1 }}>
              <strong>{t.title}</strong>
              <p>{t.channel}</p>
            </div>
            <button onClick={() => void play(t)}>{tr("Putar")}</button>
            <button onClick={() => setAdding(t)}>
              {tr("Tambah ke playlist")}
            </button>
            <button
              onClick={async () => {
                try {
                  await invoke("enqueue", {
                    track: { ...t, channel: t.channel ?? null },
                  });
                  window.dispatchEvent(new Event("queue-changed"));
                  setNotice(
                    tr("{title} ditambahkan ke antrean.", { title: t.title }),
                  );
                } catch (e) {
                  setError(errorText(e));
                }
              }}
            >
              {tr("Tambah ke antrean")}
            </button>
          </article>
        ))}
      </div>
      {adding && (
        <PlaylistPicker
          track={adding}
          onClose={() => setAdding(null)}
          onAdded={() =>
            setNotice(
              tr("{title} ditambahkan ke playlist.", { title: adding.title }),
            )
          }
        />
      )}
    </section>
  );
}
