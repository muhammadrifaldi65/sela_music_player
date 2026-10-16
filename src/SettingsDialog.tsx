import { useEffect, useRef, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useSettings, type AppSettings } from "./settings";
import { useTranslation, errorText } from "./i18n";
import { Discord } from "./Discord";
import type { PlaybackState } from "./playback";
import type { RepeatMode } from "./advance";
type Lastfm = {
  configured: boolean;
  connected: boolean;
  username: string | null;
  pending: number;
  error: string | null;
  authorizing: boolean;
};
const categories = [
  "Umum",
  "Tampilan",
  "Pemutaran",
  "Pintasan global",
  "Discord",
  "Scrobbling",
  "Data & penyimpanan",
  "Tentang",
];
export function Settings({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation(),
    { settings, error: loadError, save } = useSettings();
  const actionPending = useRef(false);
  const dialog = useRef<HTMLDialogElement>(null),
    [category, setCategory] = useState(categories[0]),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [notice, setNotice] = useState(""),
    [version, setVersion] = useState(""),
    [hotkeys, setHotkeys] = useState<AppSettings["hotkeys"] | null>(null),
    [options, setOptions] = useState<{
      repeat: RepeatMode;
      autoplay: boolean;
    } | null>(null),
    [service, setService] = useState<Lastfm | null>(null),
    [metrics, setMetrics] = useState<{ bytes: number; files: number } | null>(
      null,
    ),
    [apiKey, setApiKey] = useState(""),
    [apiSecret, setApiSecret] = useState("");
  useEffect(() => {
    const element = dialog.current!,
      previous = document.activeElement;
    element.showModal();
    void invoke<string | null>("get_settings_status")
      .then((status) => {
        if (status) setError(errorText(status));
      })
      .catch((e) => setError(errorText(e)));
    void getVersion()
      .then(setVersion)
      .catch((e) => setError(errorText(e)));
    return () => {
      element.close();
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);
  useEffect(() => {
    if (settings) setHotkeys(settings.hotkeys);
  }, [settings]);
  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        if (category === "Pemutaran") {
          const state = await invoke<PlaybackState>("playback_state");
          if (alive)
            setOptions({
              repeat: state.repeat ?? "off",
              autoplay: state.autoplay ?? true,
            });
        }
        if (category === "Scrobbling") {
          const state = await invoke<Lastfm>("lastfm_state");
          if (alive) setService(state);
        }
        if (category === "Data & penyimpanan") {
          const state = await invoke<{ bytes: number; files: number }>(
            "cache_metrics",
          );
          if (alive) setMetrics(state);
        }
      } catch (e) {
        if (alive) setError(errorText(e));
      }
    };
    void load();
    const timer =
      category === "Scrobbling"
        ? setInterval(() => void load(), 5000)
        : undefined;
    return () => {
      alive = false;
      clearInterval(timer);
    };
  }, [category]);
  async function run(action: () => Promise<void>) {
    if (actionPending.current) return;
    actionPending.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await action();
    } catch (e) {
      setError(errorText(e));
    } finally {
      actionPending.current = false;
      setBusy(false);
    }
  }
  async function change(patch: Partial<AppSettings>) {
    if (settings)
      await run(async () => {
        await save({ ...settings, ...patch });
        setNotice(t("Tersimpan"));
      });
  }
  const row = (label: string, control: ReactNode, help?: string) => (
    <label className="settings-row">
      <span>
        <strong>{t(label)}</strong>
        {help && <small>{t(help)}</small>}
      </span>
      {control}
    </label>
  );
  const toggle = (
    key: "startup" | "notifications" | "closeToTray" | "recordHistory",
    label: string,
    help?: string,
  ) =>
    row(
      label,
      <input
        type="checkbox"
        checked={settings?.[key] ?? false}
        onChange={(event) => void change({ [key]: event.target.checked })}
      />,
      help,
    );
  const range = (
    key: "blur" | "glow" | "textSize" | "opacity" | "radius",
    label: string,
    min: number,
    max: number,
    step = 1,
  ) =>
    row(
      label,
      <span className="settings-range">
        <input
          aria-label={t(label)}
          type="range"
          min={min}
          max={max}
          step={step}
          value={settings!.appearance[key]}
          onChange={(event) =>
            void change({
              appearance: {
                ...settings!.appearance,
                [key]: Number(event.target.value),
              },
            })
          }
        />
        <output>{settings!.appearance[key]}</output>
      </span>,
    );
  return (
    <dialog
      ref={dialog}
      className="settings-dialog"
      aria-labelledby="settings-title"
      onCancel={(event) => {
        event.preventDefault();
        if (!busy) onClose();
      }}
    >
      <header>
        <div>
          <p>Sela</p>
          <h2 id="settings-title">{t("Pengaturan")}</h2>
        </div>
        <button
          className="dialog-close"
          disabled={busy}
          aria-label={t("Tutup pengaturan")}
          onClick={onClose}
        >
          ×
        </button>
      </header>
      <div className="settings-layout">
        <nav aria-label={t("Pengaturan")}>
          {categories.map((label) => (
            <button
              key={label}
              aria-current={label === category ? "page" : undefined}
              onClick={() => {
                setCategory(label);
                setError("");
                setNotice("");
              }}
            >
              {t(label)}
            </button>
          ))}
        </nav>
        <section className="settings-content" aria-busy={busy}>
          <h3>{t(category)}</h3>
          {Boolean(error || loadError) && (
            <p role="alert">{error || errorText(loadError)}</p>
          )}
          <p className="settings-notice" role="status">
            {notice}
          </p>
          {!settings ? (
            <p role="status">{t("Memuat…")}</p>
          ) : (
            <fieldset disabled={busy}>
              {category === "Umum" && (
                <>
                  {row(
                    "Bahasa antarmuka",
                    <select
                      value={settings.language}
                      onChange={(event) =>
                        void change({
                          language: event.target
                            .value as AppSettings["language"],
                        })
                      }
                    >
                      <option value="system">{t("Ikuti sistem")}</option>
                      <option value="id">Bahasa Indonesia</option>
                      <option value="en">English</option>
                    </select>,
                  )}
                  {toggle(
                    "recordHistory",
                    "Rekam riwayat",
                    "Tidak menghapus riwayat yang sudah ada.",
                  )}
                  {toggle(
                    "closeToTray",
                    "Tutup ke tray",
                    "Sela tetap berjalan saat jendela ditutup. Gunakan Keluar untuk berhenti.",
                  )}
                  {toggle("notifications", "Notifikasi lagu")}
                  {toggle("startup", "Jalankan saat masuk")}
                  <button
                    onClick={() =>
                      void run(async () => {
                        await invoke("quit_app");
                      })
                    }
                  >
                    {t("Keluar dari Sela")}
                  </button>
                </>
              )}
              {category === "Tampilan" && (
                <>
                  {range("blur", "Blur kaca", 0, 40)}
                  {range("glow", "Intensitas glow", 0, 100)}
                  {range("textSize", "Ukuran teks", 14, 20)}
                  {range("opacity", "Opasitas panel", 0.2, 1, 0.05)}
                  {range("radius", "Sudut panel", 0, 32)}
                  {row(
                    "Animasi",
                    <input
                      type="checkbox"
                      checked={settings.appearance.animations}
                      onChange={(event) =>
                        void change({
                          appearance: {
                            ...settings.appearance,
                            animations: event.target.checked,
                          },
                        })
                      }
                    />,
                    "Preferensi kurangi gerakan sistem selalu dihormati.",
                  )}
                  {(["accent", "background"] as const).map((key) => (
                    <div key={key}>
                      {row(
                        key === "accent" ? "Warna aksen" : "Warna latar",
                        <input
                          type="color"
                          value={settings.appearance[key]}
                          onChange={(event) =>
                            void change({
                              appearance: {
                                ...settings.appearance,
                                [key]: event.target.value,
                              },
                            })
                          }
                        />,
                      )}
                    </div>
                  ))}
                </>
              )}
              {category === "Pemutaran" && (
                <>
                  {options ? (
                    <>
                      {row(
                        "Putar otomatis",
                        <input
                          type="checkbox"
                          checked={options.autoplay}
                          onChange={(event) => {
                            const next = {
                              ...options,
                              autoplay: event.target.checked,
                            };
                            void run(async () => {
                              await invoke("set_playback_options", next);
                              setOptions(next);
                            });
                          }}
                        />,
                      )}
                      {row(
                        "Ulang",
                        <select
                          value={options.repeat}
                          onChange={(event) => {
                            const next = {
                              ...options,
                              repeat: event.target.value as RepeatMode,
                            };
                            void run(async () => {
                              await invoke("set_playback_options", next);
                              setOptions(next);
                            });
                          }}
                        >
                          {(["off", "once", "song", "queue"] as const).map(
                            (mode, i) => (
                              <option key={mode} value={mode}>
                                {t(
                                  [
                                    "Ulang mati",
                                    "Ulang sekali",
                                    "Ulang lagu terus",
                                    "Ulang antrean",
                                  ][i],
                                )}
                              </option>
                            ),
                          )}
                        </select>,
                      )}
                    </>
                  ) : (
                    <p role="status">{t("Memuat…")}</p>
                  )}
                  {row(
                    "Mode awal",
                    <select
                      value={settings.initialMode}
                      onChange={(event) =>
                        void change({
                          initialMode: event.target.value as "audio" | "video",
                        })
                      }
                    >
                      <option value="audio">Audio</option>
                      <option value="video">Video</option>
                    </select>,
                    "Berlaku saat aplikasi berikutnya dibuka.",
                  )}
                  {row(
                    "Volume awal",
                    <span className="settings-range">
                      <input
                        aria-label={t("Volume awal")}
                        type="range"
                        min="0"
                        max="100"
                        value={settings.initialVolume}
                        onChange={(event) =>
                          void change({
                            initialVolume: Number(event.target.value),
                          })
                        }
                      />
                      <output>{settings.initialVolume}%</output>
                    </span>,
                  )}
                </>
              )}
              {category === "Pintasan global" && hotkeys && (
                <>
                  <p>
                    {t(
                      "Gunakan Ctrl, Alt, Shift atau Super, misalnya Ctrl+Alt+Space. Konflik tidak mengganti pintasan yang aktif.",
                    )}
                  </p>
                  {row(
                    "Aktifkan pintasan global",
                    <input
                      type="checkbox"
                      checked={hotkeys.enabled}
                      onChange={(event) =>
                        setHotkeys({
                          ...hotkeys,
                          enabled: event.target.checked,
                        })
                      }
                    />,
                  )}
                  {(
                    [
                      "playPause",
                      "next",
                      "previous",
                      "volumeUp",
                      "volumeDown",
                    ] as const
                  ).map((key, i) => (
                    <div key={key}>
                      {row(
                        [
                          "Putar / jeda",
                          "Lagu berikutnya",
                          "Lagu sebelumnya",
                          "Volume naik",
                          "Volume turun",
                        ][i],
                        <input
                          value={hotkeys[key]}
                          maxLength={80}
                          spellCheck={false}
                          onChange={(event) =>
                            setHotkeys({
                              ...hotkeys,
                              [key]: event.target.value,
                            })
                          }
                        />,
                      )}
                    </div>
                  ))}
                  <button
                    className="primary-button"
                    onClick={() => void change({ hotkeys })}
                  >
                    {t("Simpan pintasan")}
                  </button>
                  <h4>{t("Pintasan aplikasi")}</h4>
                  <p>
                    {t(
                      "Ctrl+K: cari musik. Escape: tutup dialog atau antrean.",
                    )}
                  </p>
                </>
              )}
              {category === "Discord" && (
                <>
                  <p>
                    {t("Bagikan lagu yang sedang diputar di Discord desktop.")}
                  </p>
                  <Discord controlsOnly />
                  <p>
                    {t(
                      "Koneksi dikelola oleh aplikasi Discord di perangkat ini.",
                    )}
                  </p>
                </>
              )}
              {category === "Scrobbling" && (
                <>
                  <p>
                    {t(
                      "Kredensial aplikasi Last.fm dan persetujuan akun diperlukan. Rahasia disimpan terlindungi di perangkat.",
                    )}
                  </p>
                  {service && (
                    <p role="status">
                      {service.connected
                        ? t("Terhubung sebagai {name}", {
                            name: service.username ?? "",
                          })
                        : service.authorizing
                          ? t("Menunggu persetujuan di browser")
                          : t(
                              service.configured
                                ? "Belum terhubung"
                                : "Belum dikonfigurasi",
                            )}{" "}
                      ·{" "}
                      {t("{count} pengiriman menunggu", {
                        count: service.pending,
                      })}
                    </p>
                  )}
                  {service?.error && (
                    <p role="alert">{errorText(service.error)}</p>
                  )}
                  {row(
                    "Kunci API",
                    <input
                      autoComplete="off"
                      value={apiKey}
                      maxLength={32}
                      onChange={(event) => setApiKey(event.target.value)}
                    />,
                  )}
                  {row(
                    "Rahasia API",
                    <input
                      type="password"
                      autoComplete="new-password"
                      value={apiSecret}
                      maxLength={32}
                      onChange={(event) => setApiSecret(event.target.value)}
                    />,
                  )}
                  <div className="settings-actions">
                    <button
                      disabled={
                        !/^[a-f\d]{32}$/i.test(apiKey) ||
                        !/^[a-f\d]{32}$/i.test(apiSecret)
                      }
                      onClick={() =>
                        void run(async () => {
                          setService(
                            await invoke<Lastfm>("lastfm_configure", {
                              apiKey,
                              apiSecret,
                            }),
                          );
                          setApiSecret("");
                          setApiKey("");
                          setNotice(t("Tersimpan"));
                        })
                      }
                    >
                      {t("Simpan kredensial")}
                    </button>
                    <button
                      disabled={!service?.configured}
                      onClick={() =>
                        void run(async () => {
                          await invoke("lastfm_begin_auth");
                          setService(await invoke<Lastfm>("lastfm_state"));
                        })
                      }
                    >
                      {t("Hubungkan Last.fm")}
                    </button>
                    <button
                      disabled={!service?.authorizing}
                      onClick={() =>
                        void run(async () => {
                          setService(
                            await invoke<Lastfm>("lastfm_complete_auth"),
                          );
                        })
                      }
                    >
                      {t("Selesaikan otorisasi")}
                    </button>
                    <button
                      disabled={!service?.connected && !service?.authorizing}
                      onClick={() =>
                        void run(async () => {
                          setService(await invoke<Lastfm>("lastfm_disconnect"));
                        })
                      }
                    >
                      {t("Putuskan koneksi")}
                    </button>
                  </div>
                </>
              )}
              {category === "Data & penyimpanan" && (
                <>
                  <h4>{t("Cache")}</h4>
                  <p>
                    {metrics
                      ? t("{size} MB · {count} berkas", {
                          size: (metrics.bytes / 1048576).toFixed(2),
                          count: metrics.files,
                        })
                      : t("Memuat…")}
                  </p>
                  <div className="settings-actions">
                    <button
                      onClick={() =>
                        void run(async () => {
                          setMetrics(await invoke("cache_metrics"));
                        })
                      }
                    >
                      {t("Hitung ulang")}
                    </button>
                    <button
                      onClick={() =>
                        void run(async () => {
                          setMetrics(await invoke("clear_cache"));
                          setNotice(
                            t(
                              "Cache dibersihkan tanpa menghapus playlist atau media aktif.",
                            ),
                          );
                        })
                      }
                    >
                      {t("Bersihkan cache")}
                    </button>
                  </div>
                  <h4>{t("Playlist saya")}</h4>
                  <div className="settings-actions">
                    <button
                      onClick={() =>
                        void run(async () => {
                          const json = await invoke<string>("export_playlists"),
                            url = URL.createObjectURL(
                              new Blob([json], { type: "application/json" }),
                            );
                          const link = document.createElement("a");
                          link.href = url;
                          link.download = "sela-playlists.json";
                          link.click();
                          setTimeout(() => URL.revokeObjectURL(url), 1000);
                          setNotice(t("Playlist diekspor."));
                        })
                      }
                    >
                      {t("Ekspor playlist")}
                    </button>
                    <label className="file-button">
                      {t("Impor playlist")}
                      <input
                        type="file"
                        accept="application/json,.json"
                        onChange={(event) => {
                          const file = event.target.files?.[0];
                          event.target.value = "";
                          if (!file) return;
                          void run(async () => {
                            if (file.size > 10 * 1024 * 1024)
                              throw new Error(
                                t("Berkas terlalu besar (maksimal 10 MB)."),
                              );
                            const result = await invoke<{
                              playlists: number;
                              tracks: number;
                            }>("import_playlists", { json: await file.text() });
                            window.dispatchEvent(new Event("library-changed"));
                            setNotice(
                              t("{count} playlist dan {tracks} lagu diimpor.", {
                                count: result.playlists,
                                tracks: result.tracks,
                              }),
                            );
                          });
                        }}
                      />
                    </label>
                  </div>
                  <h4>{t("Riwayat")}</h4>
                  <button
                    onClick={() => {
                      if (
                        confirm(
                          t(
                            "Hapus semua riwayat dengar? Tindakan ini tidak dapat dibatalkan.",
                          ),
                        )
                      )
                        void run(async () => {
                          await invoke("delete_listening_history", {
                            confirmed: true,
                          });
                          window.dispatchEvent(new Event("history-changed"));
                          setNotice(t("Riwayat dihapus."));
                        });
                    }}
                  >
                    {t("Hapus riwayat")}
                  </button>
                </>
              )}
              {category === "Tentang" && (
                <>
                  <p className="about-brand">Sela</p>
                  <p>By PGM Community</p>
                  <p>
                    {t("Versi")}: {version || t("Memuat…")}
                  </p>
                  <h4>{t("Lisensi aplikasi")}</h4>
                  <p>GNU General Public License v3.0</p>
                  <h4>{t("Pengakuan dependensi")}</h4>
                  <ul className="license-list">
                    <li>React · MIT</li>
                    <li>Tauri · MIT / Apache-2.0</li>
                    <li>Vite · MIT</li>
                    <li>TypeScript · Apache-2.0</li>
                    <li>libmpv / mpv · GPL-2.0-or-later / LGPL-2.1-or-later</li>
                    <li>yt-dlp · Unlicense</li>
                    <li>FFmpeg · LGPL-2.1-or-later / GPL, build-dependent</li>
                    <li>LRCLIB · lyrics provider</li>
                    <li>YouTube · media source</li>
                    <li>Last.fm · scrobbling service</li>
                  </ul>
                  <p>
                    {t(
                      "Komponen runtime memiliki lisensinya sendiri; izin distribusi harus ditinjau sebelum distribusi.",
                    )}
                  </p>
                </>
              )}
            </fieldset>
          )}
        </section>
      </div>
    </dialog>
  );
}
