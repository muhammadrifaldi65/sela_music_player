# Sela

**Pemutar musik desktop oleh PGM Community.**

Sela menggabungkan pencarian musik YouTube, discovery dari YouTube Music, playlist lokal, lirik, dan Discord Rich Presence dalam aplikasi berbasis Tauri. Tidak perlu akun Sela atau backend aplikasi yang di-host sendiri. Playlist, antrean, riwayat, dan preferensi genre disimpan secara lokal menggunakan SQLite.

**Status:** versi pengembangan `0.1.0`. Target utama Windows x64. Port Android masih dalam pengembangan; belum ada APK atau verifikasi playback Android.

## Fitur

- **Cari dan putar musik** — pencarian YouTube dengan streaming audio melalui yt-dlp dan libmpv.
- **Home musik** — rekomendasi, rilisan baru, dan trending dari sumber YouTube Music; kategori Explore menggunakan wilayah Indonesia.
- **Discovery berdasarkan genre** — pilih genre saat pertama membuka aplikasi, lalu ubah preferensinya. Rekomendasi memakai pencarian genre, bukan klasifikasi genre terverifikasi atau machine learning perilaku.
- **Playlist lokal** — buat, ubah nama, hapus, tambah/hapus lagu, atur urutan, dan putar seluruh playlist.
- **Antrean pemutaran** — lihat, tambah, hapus, dan atur urutan lagu; antrean disimpan secara lokal.
- **Kontrol audio** — putar/jeda, lagu sebelumnya/berikutnya, seek, volume, repeat sekali/lagu/antrean, dan opsi autoplay.
- **Riwayat mendengarkan** — tersimpan lintas sesi, dikelompokkan berdasarkan tanggal lokal.
- **Lirik** — mengambil lirik dari LRCLIB, menyorot baris aktif pada lirik tersinkron, dan mendukung klik baris untuk seek jika timing tersedia.
- **Now Playing** — tampilan artwork dan lirik yang diperluas dari pemutar bawah.
- **Discord Rich Presence** — tampilkan aktivitas mendengarkan; dapat dinyalakan/dimatikan melalui tombol Discord. Memerlukan Discord desktop berjalan.
- **Antarmuka responsif** — sidebar ringkas, drawer antrean, dan navigasi untuk lebar layar kecil. `Ctrl+K` membuka pencarian.

Streaming, discovery, artwork, dan lirik memerlukan internet serta ketersediaan layanan sumber. Penyimpanan lokal bukan fitur unduh musik offline.
<img width="1917" height="1025" alt="Screenshot 2026-10-14 225650" src="https://github.com/user-attachments/assets/285dfcee-e0cc-4ce8-8450-b55912974a86" />
<img width="1917" height="1028" alt="Screenshot 2026-10-14 225725" src="https://github.com/user-attachments/assets/26eae2a2-b900-40d4-ad25-28624c677837" />
<img width="1917" height="1020" alt="Screenshot 2026-10-14 225845" src="https://github.com/user-attachments/assets/f89e2cfa-bce8-4464-b26b-104f739325d4" />
<img width="1917" height="1035" alt="Screenshot 2026-10-14 225909" src="https://github.com/user-attachments/assets/8888cf1e-f6ca-4352-8b27-db6d92310df3" />

## Teknologi

| Bagian | Teknologi |
| --- | --- |
| Antarmuka | React 19, TypeScript, Vite 6 |
| Aplikasi native | Tauri 2, Rust |
| Penyimpanan lokal | SQLite melalui rusqlite |
| Audio Windows | libmpv |
| Pencarian dan ekstraksi audio | yt-dlp, runtime Node.js |
| Lirik | LRCLIB |
| Aktivitas Discord | Discord Rich Presence |

## Menjalankan di Windows

### 1. Siapkan perangkat pengembangan

- Windows x64.
- [Node.js](https://nodejs.org/) 22.18+ beserta npm, agar perintah pengujian TypeScript juga dapat dijalankan.
- [Rust stable](https://rustup.rs/) dengan toolchain MSVC.
- Visual Studio Build Tools dengan workload **Desktop development with C++** dan Windows SDK.
- Microsoft Edge WebView2 Runtime.

Lihat [prasyarat Tauri untuk Windows](https://v2.tauri.app/start/prerequisites/).

### 2. Siapkan runtime native

Binary runtime **tidak disertakan dalam repository**. Sebelum menjalankan atau membangun aplikasi, siapkan struktur berikut:

```text
landing/
├── yt-dlp.exe
├── native-runtime/
│   ├── libmpv-2.dll
│   └── node.exe
├── package.json
└── src-tauri/
```

Nama folder hasil clone boleh berbeda dari `landing`.

| File | Sumber |
| --- | --- |
| `yt-dlp.exe` | [Rilis resmi yt-dlp](https://github.com/yt-dlp/yt-dlp/releases), binary Windows x64 |
| `native-runtime/libmpv-2.dll` | Ekstrak DLL dari [mpv.net 7.1.2.0 portable x64](https://github.com/mpvnet-player/mpv.net/releases/tag/v7.1.2.0) |
| `native-runtime/node.exe` | Binary dari [distribusi resmi Node.js Windows x64](https://nodejs.org/en/download) |

`src-tauri/tauri.windows.conf.json` memetakan ketiga file tersebut sebagai resource aplikasi. Node.js pada PATH saja tidak menggantikan kebutuhan `native-runtime/node.exe`.

### 3. Instal dependensi dan jalankan

Jalankan dari root repository menggunakan PowerShell:

```powershell
npm ci
npm run tauri -- dev
```

Jika Cargo belum dikenali setelah instalasi Rust, buka terminal baru atau tambahkan ke PATH sesi:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
```

`npm run dev` hanya menjalankan frontend Vite. Fitur native seperti database dan playback memerlukan aplikasi Tauri, bukan browser biasa.

## Build dan pengujian

```powershell
# Pemeriksaan TypeScript dan build frontend
npm run build

# Pengujian keputusan repeat/autoplay
npm test

# Pengujian pembagian rekomendasi genre
node --experimental-strip-types tests/genres.test.ts

# Pengujian backend Rust
cargo test --manifest-path src-tauri/Cargo.toml

# Build aplikasi dan installer Windows NSIS
npm run tauri -- build
```

Installer Windows dihasilkan di `src-tauri/target/release/bundle/nsis/`. Jangan commit installer ke repository; gunakan GitHub Releases jika ingin membagikannya setelah verifikasi dan pemenuhan lisensi dependency.

## Struktur proyek

```text
src/                    Antarmuka React dan logika frontend
src-tauri/src/          Backend Rust, database, audio, dan integrasi layanan
src-tauri/android/      Plugin native Android
src-tauri/gen/android/  Proyek Android dan Gradle wrapper
src-tauri/icons/        Ikon aplikasi
tests/                  Pengujian frontend
docs/superpowers/       Spesifikasi dan rencana pengembangan
```

## Data dan privasi

- Database `sela.sqlite` dibuat di direktori data aplikasi melalui API `app_data_dir` Tauri, bukan di folder source.
- Beberapa preferensi antarmuka/pemutaran tersimpan di localStorage WebView.
- Tidak ada login aplikasi atau sinkronisasi akun/cloud.
- Permintaan musik dan metadata menghubungi YouTube/YouTube Music; lirik menghubungi LRCLIB. Aktivitas lagu dikirim ke Discord ketika Rich Presence aktif.

## Batasan saat ini

- Android belum selesai. Catatan pengembangan terakhir menyebut packaging APK terhambat izin symbolic link Windows; belum ada verifikasi playback Android.
- Pemutaran file lokal memiliki implementasi native, tetapi alur dialog file belum terverifikasi end-to-end.
- Keputusan auto-next memiliki pengujian, tetapi auto-next playback end-to-end belum terverifikasi.
- Lirik per kata, terjemahan, video, scrobbling, dan pengaturan/lokalisasi lengkap belum tersedia.
- Artwork playlist, lifecycle antrean lengkap, gapless playback, integrasi Windows, installer pada mesin bersih, dan update aplikasi masih memerlukan pengerjaan atau verifikasi.
- Tampilan status ringkas Discord di member list belum terverifikasi secara langsung.
- Packaging redistribusi beserta pemberitahuan lisensi runtime native belum selesai. Jangan menganggap build lokal sudah siap sebagai rilis publik.

Detail target: [spesifikasi aplikasi](docs/superpowers/specs/2026-10-14-music-desktop-design.md) dan [rencana implementasi](docs/superpowers/plans/2026-10-14-music-desktop.md).



## Lisensi

Lisensi untuk source aplikasi belum ditetapkan dalam repository ini. Dependency dan runtime pihak ketiga tetap mengikuti lisensinya masing-masing; periksa ketentuan yt-dlp, mpv/libmpv, Node.js, dan dependency Android sebelum redistribusi.
