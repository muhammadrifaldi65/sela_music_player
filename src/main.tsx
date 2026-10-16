import { I18nProvider, useTranslation, t as tr, errorText } from "./i18n";
import React, { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";
import { Search } from "./Search";
import { Transport } from "./Transport";
import { PlaylistTracks } from "./PlaylistTracks";
import { RenamePlaylist } from "./RenamePlaylist";
import { Queue } from "./Queue";
import { Home, History } from "./Discovery";
import { NowPlaying } from "./NowPlaying";
import { CommunityPlaylists } from "./CommunityPlaylists";
import { Discord } from "./Discord";
import { PreferenceGate } from "./Genres";
import { Settings } from "./SettingsDialog";
import { useSettings } from "./settings";
import { initializeLyrics } from "./lyricState";
initializeLyrics();
type Playlist = { id: number; name: string };
function App() {
  const { t: tr, locale } = useTranslation();
  useSettings();
  const [settingsOpen, setSettingsOpen] = useState(false);
  useEffect(() => {
    const preventContextMenu = (event: MouseEvent) => event.preventDefault();
    document.addEventListener("contextmenu", preventContextMenu);
    return () => document.removeEventListener("contextmenu", preventContextMenu);
  }, []);
  const [compact, setCompact] = useState(
    () => localStorage.getItem("sidebar-compact") === "true",
  );
  const [items, setItems] = useState<Playlist[]>([]),
    [name, setName] = useState(""),
    [error, setError] = useState(""),
    [page, setPage] = useState("home"),
    [queueOpen, setQueueOpen] = useState(false),
    [selected, setSelected] = useState<number | null>(null);
  async function refresh() {
    const lists = await invoke<Playlist[]>("playlists");
    setItems(lists);
    setSelected((id) => (lists.some((p) => p.id === id) ? id : null));
  }
  useEffect(() => {
    const changed = () => void refresh().catch((e) => setError(errorText(e)));
    changed();
    window.addEventListener("library-changed", changed);
    return () => window.removeEventListener("library-changed", changed);
  }, []);
  useEffect(() => {
    const toggle = () => setQueueOpen((v) => !v);
    const key = (event: KeyboardEvent) => {
      if (document.querySelector("dialog[open]")) return;
      if (event.key === "Escape") setQueueOpen(false);
      if (event.ctrlKey && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPage("search");
        requestAnimationFrame(() => document.getElementById("search")?.focus());
      }
    };
    window.addEventListener("keydown", key);
    window.addEventListener("toggle-queue", toggle);
    return () => {
      window.removeEventListener("keydown", key);
      window.removeEventListener("toggle-queue", toggle);
    };
  }, []);
  useEffect(() => {
    const open = (event: Event) => {
      setSelected((event as CustomEvent<number>).detail);
      setPage("library");
    };
    window.addEventListener("open-playlist", open);
    return () => window.removeEventListener("open-playlist", open);
  }, []);
  function openPlaylist(id: number) {
    setSelected(id);
    setPage("library");
  }
  async function create(e: React.FormEvent) {
    e.preventDefault();
    try {
      const id = await invoke<number>("create_playlist", { name });
      setName("");
      setError("");
      await refresh();
      openPlaylist(id);
      window.dispatchEvent(new Event("library-changed"));
    } catch (e) {
      setError(errorText(e));
    }
  }
  async function remove(playlist: Playlist) {
    if (!confirm(tr("Hapus playlist “{name}”?", { name: playlist.name })))
      return;
    try {
      await invoke("delete_playlist", { id: playlist.id });
      await refresh();
      window.dispatchEvent(new Event("library-changed"));
    } catch (e) {
      setError(errorText(e));
    }
  }
  const list = items.find((p) => p.id === selected);
  return (
    <>
      <div className={compact ? "shell sidebar-compact" : "shell"}>
        <aside>
          <div className="brand-row">
            <button
              className="nav-toggle"
              aria-label={tr("Ringkas navigasi")}
              aria-expanded={!compact}
              onClick={() => {
                setCompact(!compact);
                localStorage.setItem("sidebar-compact", String(!compact));
              }}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            </button>
            <h1>
              Sela<span>By PGM Community</span>
            </h1>
          </div>
          <nav aria-label={tr("Navigasi")}>
            {Object.entries({
              home: "Home",
              search: "Cari musik",
              history: "Riwayat",
              community: "Komunitas",
              library: "Playlist saya",
            }).map(([id, label]) => (
              <button
                key={id}
                title={tr(label)}
                aria-label={tr(label)}
                onClick={() => {
                  setPage(id);
                  if (id === "library") setSelected(null);
                }}
                aria-current={
                  page === id && (id !== "library" || !list)
                    ? "page"
                    : undefined
                }
              >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path
                    d={
                      (
                        {
                          home: "M3 10 12 3l9 7v11h-6v-7H9v7H3Z",
                          search:
                            "M21 21l-6-6M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
                          history: "M3 4v6h6M3 10a9 9 0 1 1 1 8M12 7v5l3 2",
                          community: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2M16 3a4 4 0 0 1 0 8M22 21v-2a4 4 0 0 0-3-3.87M13 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
                          library: "M4 4v16M9 4v16M14 5l4-1 4 15-4 1Z",
                        } as Record<string, string>
                      )[id]
                    }
                  />
                </svg>
                <span className="nav-label">{tr(label)}</span>
              </button>
            ))}
          </nav>
          <nav className="sidebar-playlists" aria-label={tr("Playlist saya")}>
            {items.map((p) => (
              <button
                key={p.id}
                title={p.name}
                onClick={() => openPlaylist(p.id)}
                aria-current={
                  page === "library" && selected === p.id ? "page" : undefined
                }
              >
                {p.name}
              </button>
            ))}
          </nav>
          <p className="privacy">
            {tr("Tersimpan di perangkat ini.")}
            <br />
            {tr("Akun eksternal opsional.")}
          </p>
        </aside>
        <main>
          <header className="app-header">
            <span>{tr("Musik, di ruangmu.")}</span>
            <div className="header-actions">
              <Discord />
              <button
                className="settings-open"
                onClick={() => setSettingsOpen(true)}
              >
                {tr("Pengaturan")}
              </button>
            </div>
          </header>
          <div hidden={page !== "home"}>
            {page === "home" && <Home />}
          </div>
          <div hidden={page !== "search"}>
            <Search />
          </div>
          <div hidden={page !== "history"}>
            <History />
          </div>
          {page === "community" && <CommunityPlaylists />}
          <div hidden={page !== "library"}>
            {error && <p role="alert">{error}</p>}
            {list ? (
              <>
                <button onClick={() => setSelected(null)}>
                  {tr("Kembali ke playlist")}
                </button>
                <PlaylistTracks key={list.id} playlist={list} />
                <div className="playlist-management">
                  <RenamePlaylist
                    key={`${list.id}:${list.name}`}
                    id={list.id}
                    name={list.name}
                    onSaved={refresh}
                  />
                  <button onClick={() => void remove(list)}>
                    {tr("Hapus playlist")}
                  </button>
                </div>
              </>
            ) : (
              <>
                <h2>{tr("Playlist saya")}</h2>
                <form onSubmit={create}>
                  <label htmlFor="name">{tr("Nama playlist")}</label>
                  <div className="form-row">
                    <input
                      id="name"
                      value={name}
                      onChange={(e) => setName(e.target.value)}
                      required
                      maxLength={120}
                      placeholder={tr("Perjalanan pulang")}
                    />
                    <button type="submit">{tr("Buat playlist")}</button>
                  </div>
                </form>
                {!items.length && (
                  <p>
                    {tr(
                      "Belum ada playlist. Buat koleksi pertama untuk menyimpan lagu favorit.",
                    )}
                  </p>
                )}
                <section aria-label="Playlist" className="playlists">
                  {items.map((p) => (
                    <article key={p.id}>
                      <button
                        className="playlist-card-open"
                        onClick={() => openPlaylist(p.id)}
                        aria-label={tr("Buka playlist {name}", {
                          name: p.name,
                        })}
                      >
                        <span className="cover" aria-hidden="true">
                          <svg viewBox="0 0 80 80"><path d="M32 54V24l28-6v30M32 30l28-6" /><ellipse cx="24" cy="55" rx="8" ry="6" /><ellipse cx="52" cy="49" rx="8" ry="6" /></svg>
                        </span>
                        <h3>{p.name}</h3>
                        <span>{tr("Buka playlist")}</span>
                      </button>
                      <div className="playlist-card-actions">
                      <RenamePlaylist
                        key={`${p.id}:${p.name}`}
                        id={p.id}
                        name={p.name}
                        onSaved={refresh}
                      />
                      <button onClick={() => void remove(p)}>
                        {tr("Hapus playlist")}
                      </button>
                      </div>
                    </article>
                  ))}
                </section>
              </>
            )}
          </div>
        </main>
      </div>
      <div className={queueOpen ? "queue-container open" : "queue-container"}>
        <button className="queue-close" onClick={() => setQueueOpen(false)}>
          {tr("Tutup antrean")}
        </button>
        <Queue />
      </div>
      <Transport />
      {settingsOpen && <Settings onClose={() => setSettingsOpen(false)} />}
    </>
  );
}
createRoot(document.getElementById("root")!).render(
  <I18nProvider>
    <PreferenceGate>
      <App />
      <NowPlaying />
    </PreferenceGate>
  </I18nProvider>,
);
