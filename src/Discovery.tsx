import { useTranslation, t as tr, errorText } from "./i18n";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { playTrack, type Track } from "./playback";
import { PlaylistPicker } from "./PlaylistPicker";
import { useGenreDiscovery } from "./GenreDiscovery";
type HomeShelf = { id: string; title: string; items: Track[]; error?: string };
const categories = [
  {
    id: "new-releases",
    title: "New releases",
    description: "Rilisan baru dari YouTube Music",
  },
  {
    id: "trending",
    title: "Trending",
    description: "Yang sedang ramai di YouTube Music",
  },
];
function scrollShelf(id: string, direction: number) {
  const shelf = document.getElementById(`shelf-${id}`);
  shelf?.scrollBy({
    left: direction * shelf.clientWidth,
    behavior:
      document.documentElement.dataset.animations === "false" ||
      window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
  });
}
export function Home() {
  const { t: tr, locale } = useTranslation();
  const recommendations = useGenreDiscovery();
  const [feed, setFeed] = useState<HomeShelf[]>([]),
    [recent, setRecent] = useState<Track[]>([]),
    [lists, setLists] = useState<{ id: number; name: string }[]>([]),
    [error, setError] = useState(""),
    [loading, setLoading] = useState(true),
    [filter, setFilter] = useState("Semua"),
    [destination, setDestination] = useState<Track | null>(null),
    [notice, setNotice] = useState("");
  async function load() {
    setLoading(true);
    setError("");
    try {
      setFeed(await invoke<HomeShelf[]>("music_home"));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setLoading(false);
    }
  }
  useEffect(() => {
    void load();
    let active = true;
    const update = () => {
      void Promise.all([
        invoke<Listen[]>("listening_history"),
        invoke<{ id: number; name: string }[]>("playlists"),
      ])
        .then(([history, playlists]) => {
          if (!active) return;
          const seen = new Set<string>();
          setRecent(
            history
              .filter((r) => {
                if (seen.has(r.video_id)) return false;
                seen.add(r.video_id);
                return true;
              })
              .slice(0, 10)
              .map((r) => ({
                id: r.video_id,
                title: r.title,
                channel: r.artist,
              })),
          );
          setLists(playlists);
        })
        .catch((e) => {
          if (active) setError(errorText(e));
        });
    };
    update();
    window.addEventListener("history-changed", update);
    window.addEventListener("library-changed", update);
    return () => {
      active = false;
      window.removeEventListener("history-changed", update);
      window.removeEventListener("library-changed", update);
    };
  }, []);
  const replay: HomeShelf = {
    id: "recent",
    title: "Putar lagi",
    items: recent,
  };
  const publicShelves = categories.map(
    (category) =>
      feed.find((shelf) => shelf.id === category.id) ?? {
        ...category,
        items: [],
      },
  );
  const personal: HomeShelf = {
    id: "for-you",
    title: "Untuk kamu",
    items: recommendations.items,
  };
  const shelves =
    filter === "Untuk kamu"
      ? [personal]
      : filter === "Putar lagi"
        ? [replay]
        : filter === "Koleksi"
          ? []
          : filter === "Semua"
            ? [
                personal,
                ...publicShelves,
                ...(recent.length ? [replay] : []),
                ...feed.filter(
                  (shelf) =>
                    !categories.some((category) => category.id === shelf.id),
                ),
              ]
            : publicShelves.filter((shelf) => shelf.title === filter);
  return (
    <section className="music-home" aria-busy={loading}>
      <div className="home-chips" aria-label={tr("Filter Home")}>
        {[
          "Semua",
          "Untuk kamu",
          "New releases",
          "Trending",
          "Putar lagi",
          "Koleksi",
        ].map((label) => (
          <button
            key={label}
            aria-pressed={filter === label}
            onClick={() => setFilter(label)}
          >
            {tr(label)}
          </button>
        ))}
        <button className="taste-button" onClick={recommendations.edit}>
          {tr("Selera musik")}
        </button>
      </div>
      <div className="discovery-heading">
        <h2>{tr("Musik untuk harimu")}</h2>
        <p>
          {tr(
            "Temukan rilisan terbaru, dengarkan yang sedang ramai, atau putar favoritmu lagi.",
          )}
        </p>
      </div>
      {error && (
        <p role="alert">
          {error}{" "}
          <button disabled={loading} onClick={() => void load()}>
            {tr("Coba lagi")}
          </button>
        </p>
      )}
      <p className="home-notice" role="status">
        {notice}
      </p>
      {shelves.map((shelf) => (
        <section
          className="music-shelf"
          key={shelf.id}
          aria-labelledby={`heading-${shelf.id}`}
        >
          <div className="shelf-heading">
            <div>
              <h3 id={`heading-${shelf.id}`}>{tr(shelf.title)}</h3>
              <p>
                {shelf.id === "for-you"
                  ? tr(
                      "Penemuan berbasis pencarian genre di YouTube: {genres}. Bukan klasifikasi genre terverifikasi.",
                      { genres: recommendations.labels },
                    )
                  : tr(
                      categories.find((category) => category.id === shelf.id)
                        ?.description ??
                        (shelf.id === "recent"
                          ? tr("Dari riwayat dengarmu")
                          : tr("Pilihan dari YouTube Music")),
                    )}
              </p>
            </div>
            {shelf.items.length > 0 &&
              shelf.id !== "trending" &&
              shelf.id !== "recent" && (
                <div className="shelf-controls">
                  <button
                    aria-label={tr("Geser {title} ke kiri", {
                      title: tr(shelf.title),
                    })}
                    aria-controls={`shelf-${shelf.id}`}
                    onClick={() => scrollShelf(shelf.id, -1)}
                  >
                    ‹
                  </button>
                  <button
                    aria-label={tr("Geser {title} ke kanan", {
                      title: tr(shelf.title),
                    })}
                    aria-controls={`shelf-${shelf.id}`}
                    onClick={() => scrollShelf(shelf.id, 1)}
                  >
                    ›
                  </button>
                </div>
              )}
          </div>
          {shelf.id === "for-you" && (
            <>
              {recommendations.loading && (
                <p role="status">{tr("Mencari musik sesuai genre pilihan…")}</p>
              )}
              {recommendations.errors.length > 0 && (
                <div role="alert">
                  {recommendations.errors.map((error) => (
                    <p key={error}>{error}</p>
                  ))}
                  <button
                    disabled={recommendations.loading}
                    onClick={recommendations.retry}
                  >
                    {tr("Coba lagi")}
                  </button>
                </div>
              )}
            </>
          )}
          {(shelf.id === "for-you" ? recommendations.loading : loading) &&
          shelf.id !== "recent" &&
          !shelf.items.length ? (
            <div
              className="home-skeleton"
              role="status"
              aria-label={tr("Memuat {title}", { title: tr(shelf.title) })}
            >
              {Array.from({ length: 6 }, (_, i) => (
                <div key={i} />
              ))}
            </div>
          ) : (
            <>
              {shelf.error && (
                <p role="alert">
                  {errorText(shelf.error)}{" "}
                  <button disabled={loading} onClick={() => void load()}>
                    {tr("Coba lagi")}
                  </button>
                </p>
              )}
              {!shelf.items.length && !shelf.error && (
                <p>
                  {shelf.id === "for-you"
                    ? tr(
                        "Belum ada hasil untuk genre pilihan. Coba lagi atau ubah Selera musik.",
                      )
                    : shelf.id === "recent"
                      ? tr(
                          "Belum ada riwayat. Putar lagu untuk menemukannya lagi di sini.",
                        )
                      : tr("Kategori ini belum tersedia dari YouTube Music.")}
                </p>
              )}
              <div
                className={
                  shelf.id === "trending" || shelf.id === "recent"
                    ? "home-song-list"
                    : "shelf-tracks"
                }
                id={`shelf-${shelf.id}`}
              >
                {shelf.items.map((t) => (
                  <article key={t.id}>
                    <button
                      className="art-play"
                      aria-label={tr("Putar {title}", { title: t.title })}
                      onClick={() =>
                        void playTrack(t).catch((e) => setError(errorText(e)))
                      }
                    >
                      <img
                        src={`https://i.ytimg.com/vi/${t.id}/hqdefault.jpg`}
                        alt=""
                        loading="lazy"
                      />
                      <span aria-hidden="true">▶</span>
                    </button>
                    <div className="home-track-copy">
                      <strong title={t.title}>{t.title}</strong>
                      <p title={t.channel ?? undefined}>{t.channel}</p>
                    </div>
                    <details
                      className="track-menu"
                      onKeyDown={(event) => {
                        if (event.key === "Escape") {
                          event.currentTarget.open = false;
                          event.currentTarget.querySelector("summary")?.focus();
                        }
                      }}
                      onBlur={(event) => {
                        if (!event.currentTarget.contains(event.relatedTarget))
                          event.currentTarget.open = false;
                      }}
                    >
                      <summary
                        aria-label={tr("Tindakan untuk {title}", {
                          title: t.title,
                        })}
                      >
                        <span aria-hidden="true">⋮</span>
                      </summary>
                      <div className="track-menu-actions">
                        <button
                          onClick={async (event) => {
                            const menu = event.currentTarget.closest("details");
                            if (menu) {
                              menu.open = false;
                              menu.querySelector("summary")?.focus();
                            }
                            try {
                              await invoke("enqueue", { track: t });
                              window.dispatchEvent(new Event("queue-changed"));
                              setNotice(
                                tr("{title} ditambahkan ke antrean.", {
                                  title: t.title,
                                }),
                              );
                            } catch (e) {
                              setError(errorText(e));
                            }
                          }}
                        >
                          {tr("Tambah ke antrean")}
                        </button>
                        <button
                          onClick={(event) => {
                            const menu = event.currentTarget.closest("details");
                            if (menu) {
                              menu.open = false;
                              menu.querySelector("summary")?.focus();
                            }
                            setDestination(t);
                          }}
                        >
                          {tr("Tambah ke playlist")}
                        </button>
                      </div>
                    </details>
                  </article>
                ))}
              </div>
            </>
          )}
        </section>
      ))}
      {filter === "Koleksi" && (
        <section className="music-shelf">
          <h3>{tr("Playlist pribadi")}</h3>
          {!lists.length && (
            <p>{tr("Buat playlist melalui navigasi Playlist saya.")}</p>
          )}
          <div className="home-collections">
            {lists.map((p) => (
              <button
                key={p.id}
                onClick={() =>
                  window.dispatchEvent(
                    new CustomEvent("open-playlist", { detail: p.id }),
                  )
                }
              >
                <span>{p.name.slice(0, 1)}</span>
                <strong>{p.name}</strong>
              </button>
            ))}
          </div>
        </section>
      )}
      {destination && (
        <PlaylistPicker
          track={destination}
          onClose={() => setDestination(null)}
          onAdded={() =>
            setNotice(
              tr("{title} ditambahkan ke playlist.", {
                title: destination.title,
              }),
            )
          }
        />
      )}
    </section>
  );
}
type Listen = {
  id: number;
  video_id: string;
  title: string;
  artist: string | null;
  played_at: number;
};
export function History() {
  const { t: tr, locale } = useTranslation();
  const [rows, setRows] = useState<Listen[]>([]),
    [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    const update = () => {
      void invoke<Listen[]>("listening_history")
        .then((r) => {
          if (active) setRows(r);
        })
        .catch((e) => {
          if (active) setError(errorText(e));
        });
    };
    update();
    window.addEventListener("history-changed", update);
    return () => {
      active = false;
      window.removeEventListener("history-changed", update);
    };
  }, []);
  const groups = new Map<string, Listen[]>();
  for (const row of rows) {
    const date = new Date(row.played_at * 1000).toLocaleDateString(
      locale === "id" ? "id-ID" : "en-US",
      { weekday: "long", day: "numeric", month: "long", year: "numeric" },
    );
    const group = groups.get(date) ?? [];
    group.push(row);
    groups.set(date, group);
  }
  return (
    <section>
      <h2>{tr("Riwayat putar")}</h2>
      <p>{tr("Lagu yang benar-benar mulai diputar di perangkat ini.")}</p>
      {error && <p role="alert">{error}</p>}
      {!rows.length && (
        <p>{tr("Belum ada riwayat. Putar lagu dari Home atau pencarian.")}</p>
      )}
      {[...groups].map(([date, items]) => (
        <section key={date}>
          <h3>{date}</h3>
          {items.map((row) => (
            <article className="history-row" key={row.id}>
              <img
                src={`https://i.ytimg.com/vi/${row.video_id}/mqdefault.jpg`}
                alt=""
              />
              <div>
                <strong>{row.title}</strong>
                <p>
                  {row.artist} ·{" "}
                  {new Date(row.played_at * 1000).toLocaleTimeString(
                    locale === "id" ? "id-ID" : "en-US",
                    { hour: "2-digit", minute: "2-digit" },
                  )}
                </p>
              </div>
              <button
                onClick={() =>
                  void playTrack({
                    id: row.video_id,
                    title: row.title,
                    channel: row.artist,
                  }).catch((e) => setError(errorText(e)))
                }
              >
                {tr("Putar ulang")}
              </button>
            </article>
          ))}
        </section>
      ))}
    </section>
  );
}
