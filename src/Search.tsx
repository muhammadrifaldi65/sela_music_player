import { useTranslation, errorText } from "./i18n";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playTrack, type Track } from "./playback";
import { PlaylistPicker } from "./PlaylistPicker";
import { useGenreDiscovery } from "./GenreDiscovery";
import { usePreferences } from "./Genres";
import "./Search.css";

const historyKey = "music-search-history";
const moods = [
  { title: "Santai", query: "chill relaxing music" },
  { title: "Fokus", query: "instrumental focus study music" },
  { title: "Semangat", query: "feel good upbeat music" },
  { title: "Olahraga", query: "workout music" },
];
function readHistory(): string[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(historyKey) ?? "[]");
    return Array.isArray(value)
      ? [...new Set(value.filter((item): item is string =>
          typeof item === "string" && !!item.trim() && item.length <= 200,
        ).map((item) => item.trim()))].slice(0, 8)
      : [];
  } catch {
    return [];
  }
}

export function Search() {
  const { t: tr } = useTranslation();
  const discovery = useGenreDiscovery();
  const { genres, selected } = usePreferences();
  const [recent, setRecent] = useState(readHistory);
  const [attempt, setAttempt] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState(""),
    [tracks, setTracks] = useState<Track[]>([]),
    [error, setError] = useState(""),
    [busy, setBusy] = useState(false),
    [current, setCurrent] = useState("");
  const [adding, setAdding] = useState<Track | null>(null),
    [notice, setNotice] = useState("");
  const revision = useRef(0),
    playing = useRef(0);
  function updateHistory(next: string[]) {
    setRecent(next);
    try {
      localStorage.setItem(historyKey, JSON.stringify(next));
    } catch {
      setNotice(tr("Riwayat pencarian hanya tersedia selama sesi ini."));
    }
  }
  function remember(value: string) {
    const term = value.trim().replace(/\s+/g, " ").slice(0, 200);
    if (term) updateHistory([term, ...recent.filter((item) =>
      item.toLocaleLowerCase() !== term.toLocaleLowerCase(),
    )].slice(0, 8));
  }
  function search(value: string) {
    remember(value);
    setQuery(value.trim());
    setAttempt((value) => value + 1);
  }
  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        input.current?.focus();
        input.current?.select();
      }
    };
    window.addEventListener("keydown", focusSearch);
    return () => window.removeEventListener("keydown", focusSearch);
  }, []);
  useEffect(() => {
    const id = ++revision.current;
    if (!query.trim()) {
      setTracks([]);
      setBusy(false);
      setError("");
      return;
    }
    setBusy(true);
    setTracks([]);
    setError("");
    const timer = setTimeout(() => {
      invoke<{ entries?: Track[] }>("search", { query: query.trim() })
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
  }, [query, attempt]);
  async function play(track: Track) {
    remember(query);
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
    <section className="search-surface" aria-label={tr("Cari musik")}>
      <h2>{tr("Cari")}</h2>
      <form className="search-form" role="search" onSubmit={(event) => {
        event.preventDefault();
        search(query);
      }}>
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true">
          <circle cx="10.5" cy="10.5" r="7.5" /><path d="m16 16 5 5" />
        </svg>
        <input
          ref={input}
          id="search"
          type="search"
          aria-label={tr("Cari musik di YouTube")}
          aria-keyshortcuts="Control+k Meta+k"
          autoComplete="off"
          maxLength={200}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={tr("Judul lagu atau artis")}
        />
        <kbd>Ctrl+K</kbd>
        <button type="submit" disabled={!query.trim()}>{tr("Cari")}</button>
      </form>
      {recent.length > 0 && (
        <section className="search-recent" aria-labelledby="search-recent-heading">
          <div className="search-section-heading">
            <h3 id="search-recent-heading">{tr("Pencarian terbaru")}</h3>
            <button onClick={() => updateHistory([])}>{tr("Hapus semua")}</button>
          </div>
          <ul>
            {recent.map((term) => (
              <li key={term}>
                <button onClick={() => search(term)}>{term}</button>
                <button aria-label={tr("Hapus pencarian {query}", { query: term })}
                  onClick={() => updateHistory(recent.filter((item) => item !== term))}>×</button>
              </li>
            ))}
          </ul>
        </section>
      )}
      {busy && <p role="status">{tr("Mencari…")}</p>}
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {current && <p role="status">{current}</p>}
      {query.trim() && !busy && !error && !tracks.length && (
        <p>{tr("Tidak ada hasil. Coba judul atau artis lain.")}</p>
      )}
      {!query.trim() && (
        <div className="search-discovery">
          <section aria-labelledby="search-for-you">
            <div className="search-section-heading">
              <h3 id="search-for-you">{tr("Untuk kamu")}</h3>
              <button onClick={discovery.edit}>{tr("Selera musik")}</button>
            </div>
            <p className="search-description">{tr("Pilihan berdasarkan genre favoritmu")}</p>
            {discovery.loading && <p role="status">{tr("Memuat rekomendasi…")}</p>}
            {discovery.errors.length > 0 && <div role="alert">
              {discovery.errors.map((message) => <p key={message}>{message}</p>)}
              <button onClick={discovery.retry} disabled={discovery.loading}>{tr("Coba lagi")}</button>
            </div>}
            <div className="search-tile-grid">
              {discovery.items.slice(0, 8).map((track) => (
                <button className="search-art-tile" key={track.id}
                  onClick={() => search(track.channel || track.title)}>
                  <img src={`https://i.ytimg.com/vi/${encodeURIComponent(track.id)}/hqdefault.jpg`} alt="" loading="lazy" />
                  <span>{track.channel || track.title}</span>
                  <small>{track.title}</small>
                </button>
              ))}
              {!discovery.items.length && genres.filter((genre) => selected.includes(genre.id)).map((genre) => (
                <button className="search-art-tile search-genre-tile" key={genre.id} onClick={() => search(genre.query)}>
                  <span>{genre.label}</span>
                  <small>{tr("Jelajahi genre")}</small>
                </button>
              ))}
            </div>
          </section>
          <section aria-labelledby="search-moods">
            <div className="search-section-heading"><h3 id="search-moods">{tr("Suasana & momen")}</h3></div>
            <div className="search-tile-grid">
              {moods.map((mood) => (
                <button className="search-mood-tile" key={mood.title} onClick={() => search(mood.query)}>
                  <span>{tr(mood.title)}</span>
                  <svg width="42" height="42" viewBox="0 0 42 42" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                    <path d="M7 16v10m7-17v24m7-20v16m7-24v32m7-20v8" />
                  </svg>
                </button>
              ))}
            </div>
          </section>
        </div>
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
            <button onClick={() => { remember(query); setAdding(t); }}>
              {tr("Tambah ke playlist")}
            </button>
            <button
              onClick={async () => {
                remember(query);
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
