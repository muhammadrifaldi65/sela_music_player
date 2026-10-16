import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { errorText, useTranslation } from "./i18n";
import { playback, playTrack, type Track } from "./playback";
import "./CommunityPlaylists.css";
import { discoveryChoice } from "./discoveryRotation";

type CommunityPlaylist = { id: string; title: string; author: string; thumbnail: string };

function Artwork({ src }: { src: string }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [src]);
  return src && !failed ? (
    <img src={src} alt="" loading="lazy" referrerPolicy="no-referrer" onError={() => setFailed(true)} />
  ) : <span className="community-art-fallback" aria-hidden="true">♫</span>;
}

export function CommunityPlaylists({ onLoadingChange }: { onLoadingChange?: (loading: boolean) => void } = {}) {
  const { t: tr } = useTranslation();
  const [input, setInput] = useState("");
  const [search, setSearch] = useState({ query: "", attempt: 0 });
  const [lists, setLists] = useState<CommunityPlaylist[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [selected, setSelected] = useState<CommunityPlaylist | null>(null);
  const [tracks, setTracks] = useState<Track[]>([]);
  const [trackLoading, setTrackLoading] = useState(false);
  const [trackError, setTrackError] = useState("");
  const [trackAttempt, setTrackAttempt] = useState(0);
  const [actionError, setActionError] = useState("");
  const [saving, setSaving] = useState(false);
  const [starting, setStarting] = useState(false);
  const [playerLoading, setPlayerLoading] = useState(playback.loading);
  const [saved, setSaved] = useState<Record<string, number>>({});
  const alive = useRef(false);
  const actionBusy = useRef(false);
  const detailHeading = useRef<HTMLHeadingElement>(null);
  const lastCard = useRef<HTMLButtonElement | null>(null);
  const selectedId = useRef<string | null>(null);
  selectedId.current = selected?.id ?? null;
  useEffect(() => {
    onLoadingChange?.(loading || trackLoading || starting || saving);
  }, [loading, trackLoading, starting, saving, onLoadingChange]);

  useEffect(() => {
    alive.current = true;
    const changed = () => setPlayerLoading(playback.loading);
    window.addEventListener("playback-changed", changed);
    return () => {
      alive.current = false;
      window.removeEventListener("playback-changed", changed);
    };
  }, []);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setLists([]);
    void invoke<CommunityPlaylist[]>("community_playlists", { query: search.query || discoveryChoice("community", ["chill music", "indie favorites", "acoustic songs", "pop discoveries", "late night music", "indonesia music"]) })
      .then((rows) => { if (active) setLists(rows); })
      .catch((e) => { if (active) setError(errorText(e)); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [search]);

  useEffect(() => {
    let active = true;
    setTracks([]);
    setTrackError("");
    setActionError("");
    setTrackLoading(!!selected);
    if (!selected) return;
    detailHeading.current?.focus();
    void invoke<Track[]>("community_playlist_tracks", { id: selected.id })
      .then((rows) => { if (active) setTracks(rows); })
      .catch((e) => { if (active) setTrackError(errorText(e)); })
      .finally(() => { if (active) setTrackLoading(false); });
    return () => { active = false; };
  }, [selected, trackAttempt]);

  async function start(track?: Track) {
    if (actionBusy.current || playback.loading || !selected || !tracks.length) return;
    const id = selected.id;
    actionBusy.current = true;
    setStarting(true);
    setActionError("");
    try {
      if (track) await playTrack(track);
      else {
        const revision = playback.revision;
        let firstEntry: number | undefined;
        try {
          for (const item of tracks) {
            const entry = await invoke<number>("enqueue", { track: { ...item, channel: item.channel ?? null } });
            firstEntry ??= entry;
          }
        } finally {
          window.dispatchEvent(new Event("queue-changed"));
        }
        if (playback.revision === revision && firstEntry !== undefined)
          await playTrack(tracks[0], firstEntry);
      }
    } catch (e) {
      if (alive.current && selectedId.current === id) setActionError(errorText(e));
    } finally {
      actionBusy.current = false;
      if (alive.current) setStarting(false);
    }
  }

  async function save() {
    if (!selected || actionBusy.current || saved[selected.id] !== undefined) return;
    const playlist = selected;
    actionBusy.current = true;
    setSaving(true);
    setActionError("");
    try {
      const id = await invoke<number>("save_community_playlist", { id: playlist.id, name: playlist.title.trim().slice(0, 120) });
      window.dispatchEvent(new Event("library-changed"));
      if (alive.current) setSaved((previous) => ({ ...previous, [playlist.id]: id }));
    } catch (e) {
      if (alive.current && selectedId.current === playlist.id) setActionError(errorText(e));
    } finally {
      actionBusy.current = false;
      if (alive.current) setSaving(false);
    }
  }

  return (
    <section className="community-view" aria-label={tr("Playlist komunitas")}>
      <header className="community-heading">
        <div><h2>{tr("Playlist komunitas")}</h2><p>{tr("Temukan pilihan musik dari pendengar di YouTube Music.")}</p></div>
        <form className="community-search" role="search" onSubmit={(event) => {
          event.preventDefault();
          setSelected(null);
          setSearch((previous) => ({ query: input.trim(), attempt: previous.attempt + 1 }));
        }}>
          <label className="community-sr-only" htmlFor="community-query">{tr("Cari playlist komunitas")}</label>
          <input id="community-query" type="search" maxLength={200} value={input} onChange={(event) => setInput(event.target.value)} placeholder={tr("Cari suasana, genre, atau playlist")} />
          <button type="submit">{tr("Cari")}</button>
        </form>
      </header>
      {selected ? (
        <div className="community-detail">
          <button className="community-back" onClick={() => {
            setSelected(null);
            requestAnimationFrame(() => lastCard.current?.focus());
          }}>‹ {tr("Kembali ke playlist")}</button>
          <div className="community-hero">
            <div className="community-hero-art"><Artwork src={selected.thumbnail} /></div>
            <div className="community-hero-copy">
              <p className="community-source">YouTube Music</p>
              <h3 ref={detailHeading} tabIndex={-1}>{selected.title}</h3>
              <p>{selected.author || tr("Komunitas YouTube Music")}</p>
              {!trackLoading && !trackError && <p>{tr("{count} lagu", { count: tracks.length })}</p>}
              <div className="community-actions">
                <button className="community-primary" disabled={trackLoading || !tracks.length || starting || saving || playerLoading} onClick={() => void start()}>{starting ? tr("Menyiapkan pemutaran…") : tr("Putar semua")}</button>
                {saved[selected.id] !== undefined ? (
                  <button onClick={() => window.dispatchEvent(new CustomEvent("open-playlist", { detail: saved[selected.id] }))}>{tr("Buka di koleksi")}</button>
                ) : <button disabled={saving || starting || trackLoading || !!trackError || !tracks.length} onClick={() => void save()}>{saving ? tr("Menyimpan…") : tr("Simpan ke koleksi")}</button>}
              </div>
              {saved[selected.id] !== undefined && <p className="community-notice" role="status">{tr("Playlist tersimpan di koleksi.")}</p>}
            </div>
          </div>
          {actionError && <p className="community-error" role="alert">{actionError}</p>}
          {trackLoading && <p className="community-state" role="status">{tr("Memuat lagu…")}</p>}
          {trackError && <div className="community-state community-error" role="alert"><p>{trackError}</p><button onClick={() => setTrackAttempt((attempt) => attempt + 1)}>{tr("Coba lagi")}</button></div>}
          {!trackLoading && !trackError && !tracks.length && <p className="community-state">{tr("Playlist ini belum memiliki lagu yang dapat diputar.")}</p>}
          {!trackLoading && !trackError && tracks.length > 0 && <ol className="community-tracks" aria-label={tr("Isi playlist {name}", { name: selected.title })}>
            {tracks.map((track, index) => <li key={`${track.id}-${index}`}>
              <button className="community-track" disabled={starting || saving || playerLoading} onClick={() => void start(track)} aria-label={tr("Putar {title}", { title: track.title })}>
                <span className="community-track-index" aria-hidden="true">{index + 1}</span>
                <span className="community-track-art"><Artwork src={`https://i.ytimg.com/vi/${encodeURIComponent(track.id)}/mqdefault.jpg`} /></span>
                <span className="community-track-copy"><strong>{track.title}</strong><span>{track.channel || "YouTube Music"}</span></span>
                <span className="community-track-play" aria-hidden="true">▷</span>
              </button>
            </li>)}
          </ol>}
        </div>
      ) : (
        <section className="community-results" aria-busy={loading} aria-labelledby="community-results-title">
          <div className="community-results-heading"><h3 id="community-results-title">{search.query ? tr('Hasil untuk “{query}”', { query: search.query }) : tr("Dari komunitas, untuk kamu")}</h3>{search.query && <button onClick={() => { setInput(""); setSearch({ query: "", attempt: search.attempt + 1 }); }}>{tr("Jelajahi semua")}</button>}</div>
          {loading && <div className="community-state" role="status"><span className="community-loading-mark" aria-hidden="true" />{tr("Memuat playlist…")}</div>}
          {error && <div className="community-state community-error" role="alert"><p>{error}</p><button onClick={() => setSearch((previous) => ({ ...previous, attempt: previous.attempt + 1 }))}>{tr("Coba lagi")}</button></div>}
          {!loading && !error && !lists.length && <div className="community-state"><h4>{tr("Playlist tidak ditemukan")}</h4><p>{tr("Coba kata kunci lain untuk menemukan playlist publik.")}</p></div>}
          <div className="community-grid">{lists.map((playlist) => <button className="community-card" key={playlist.id} ref={(node) => { if (node && lastCard.current?.dataset.playlistId === playlist.id) lastCard.current = node; }} data-playlist-id={playlist.id} onClick={(event) => { lastCard.current = event.currentTarget; setSelected(playlist); }}>
            <span className="community-card-art"><Artwork src={playlist.thumbnail} /><span className="community-card-open" aria-hidden="true">↗</span></span>
            <span className="community-card-title">{playlist.title}</span><span className="community-card-author">{playlist.author || tr("Komunitas YouTube Music")}</span>
          </button>)}</div>
        </section>
      )}
    </section>
  );
}
