 # Sela 
<img width="128" height="128" alt="128x128" src="https://github.com/user-attachments/assets/f3b81716-5246-44bc-bde8-eb00ec9ef2bb" />


**Pemutar musik desktop oleh PGM Community.**

Sela menggabungkan pencarian musik YouTube, discovery dari YouTube Music, playlist lokal, lirik, dan Discord Rich Presence dalam aplikasi berbasis Tauri. Tidak perlu akun Sela atau backend aplikasi yang di-host sendiri. Playlist, antrean, riwayat, dan preferensi genre disimpan secara lokal menggunakan SQLite.

**Status:** versi pengembangan `0.1.1`. Target utama Windows x64. Port Android masih dalam pengembangan; belum ada APK atau verifikasi playback Android.

## Fitur

- **Cari dan putar musik** — pencarian YouTube dengan streaming audio melalui yt-dlp dan libmpv.
- **Home musik** — rekomendasi, rilisan baru, dan trending dari sumber YouTube Music; kategori Explore menggunakan wilayah Indonesia.
- **Discovery berdasarkan genre** — pilih genre saat pertama membuka aplikasi, lalu ubah preferensinya. Rekomendasi memakai pencarian genre, bukan klasifikasi genre terverifikasi atau machine learning perilaku.
- **Playlist lokal** — buat, ubah nama, hapus, tambah/hapus lagu, atur urutan, dan putar seluruh playlist.
- **Antrean pemutaran** — lihat, tambah, hapus, dan atur urutan lagu; antrean disimpan secara lokal.
- **Kontrol audio** — putar/jeda, sebelumnya/berikutnya (termasuk rekomendasi saat antrean habis), seek, volume responsif, repeat sekali/lagu/antrean, dan autoplay native yang tetap berjalan saat jendela diminimalkan. Kegagalan sementara dicoba ulang secara terbatas; pause manual tetap dihormati.
- **Radio autoplay Windows** — saat antrean habis, mengambil kandidat radio YouTube Music berdasarkan lagu, menghindari ID/versi lagu dalam antrean dan 50 riwayat terakhir, serta memprioritaskan artis di luar tiga pemutaran terakhir. Urutan antrean manual tetap didahulukan; ini bukan salinan personalisasi akun YouTube Music.
- **Riwayat mendengarkan** — tersimpan lintas sesi, dikelompokkan berdasarkan tanggal lokal.
- **Lirik** — mengambil lirik dari LRCLIB, menyorot baris aktif pada lirik tersinkron, dan mendukung klik baris untuk seek jika timing tersedia.
- **Now Playing** — artwork dan lirik dengan latar blur serta panel kaca. Windows mendukung pilihan Audio/Video dalam halaman yang sama melalui libmpv; pergantian mempertahankan posisi, volume, dan status jeda. Video disembunyikan saat halaman ditutup atau tertutup panel lain. Android tetap audio.
- **Discord Rich Presence** — tampilkan aktivitas mendengarkan; dapat dinyalakan/dimatikan melalui tombol Discord. Memerlukan Discord desktop berjalan.
- **Antarmuka responsif** — sidebar ringkas, drawer antrean, dan navigasi untuk lebar layar kecil. `Ctrl+K` membuka pencarian.
- **Pengaturan** — bahasa Indonesia/English, riwayat, tray, startup dan notifikasi opt-in, tampilan glass/glow, mode/volume awal, pintasan global, Discord, Last.fm, cache, dan impor/ekspor playlist. Preferensi disimpan lokal; konflik pintasan dilaporkan.
- **Last.fm** — memerlukan kunci/rahasia aplikasi dan otorisasi akun melalui Settings. Kredensial Windows dilindungi DPAPI; pengiriman scrobble menggunakan waktu dengar aktual dan antrean retry lokal.

Streaming, discovery, artwork, dan lirik memerlukan internet serta ketersediaan layanan sumber. Penyimpanan lokal bukan fitur unduh musik offline.
<img width="1917" height="1025" alt="Screenshot 2026-10-14 225650" src="https://github.com/user-attachments/assets/73798a40-975e-4a1d-8b33-7ed52f1e97ab" />
<img width="1917" height="1028" alt="Screenshot 2026-10-14 225725" src="https://github.com/user-attachments/assets/b4a11ed3-a6b7-485d-9d07-8c9b6393c189" />
<img width="1917" height="1022" alt="image" src="https://github.com/user-attachments/assets/0e3fc84a-e5ca-4529-95c0-3a79819dc661" />
<img width="1917" height="1022" alt="image" src="https://github.com/user-attachments/assets/fe5756cc-5a6b-4cde-bf43-98968e1dfe57" />

Track antrean berikutnya disiapkan lebih awal bila tersedia; klik lagu langsung menampilkan status penyiapan. Lirik mulai disiapkan bersamaan dengan ekstraksi audio/video, tanpa menunggu panel lirik atau durasi media; hasil di-cache dan disesuaikan saat durasi tersedia. Respons penyedia tetap dapat terlambat atau terkena rate-limit. Animasi menghormati pengaturan reduced motion.

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
- Tidak ada akun Sela atau sinkronisasi playlist cloud. Last.fm opsional membutuhkan otorisasi akun terpisah.
- Permintaan musik dan metadata menghubungi YouTube/YouTube Music; lirik menghubungi LRCLIB. Aktivitas lagu dikirim ke Discord ketika Rich Presence aktif dan Last.fm setelah akun dihubungkan.

## Batasan saat ini

- Android belum selesai. Catatan pengembangan terakhir menyebut packaging APK terhambat izin symbolic link Windows; belum ada verifikasi playback Android.
- Pemutaran file lokal memiliki implementasi native, tetapi alur dialog file belum terverifikasi end-to-end.
- Auto-next native telah diuji melalui EOF dan jendela diminimalkan; pengujian berjam-jam dan semua kondisi jaringan belum dilakukan.
- Lirik per kata dan terjemahan isi lirik belum tersedia. UI Indonesia/English, video Windows, Settings, dan integrasi Last.fm sudah diimplementasikan; otorisasi Last.fm nyata memerlukan kredensial dan persetujuan pengguna.
- Artwork playlist, lifecycle antrean lengkap, gapless playback, integrasi Windows, installer pada mesin bersih, dan update aplikasi masih memerlukan pengerjaan atau verifikasi.
- Tampilan status ringkas Discord di member list belum terverifikasi secara langsung.
- Packaging redistribusi beserta pemberitahuan lisensi runtime native belum selesai. Jangan menganggap build lokal sudah siap sebagai rilis publik.

Detail target: [spesifikasi aplikasi](docs/superpowers/specs/2026-10-14-music-desktop-design.md) dan [rencana implementasi](docs/superpowers/plans/2026-10-14-music-desktop.md).



## Lisensi

<<<<<<< HEAD
Repository menyertakan GNU General Public License v3 pada `LICENSE`. Dependency dan runtime pihak ketiga tetap mengikuti lisensinya masing-masing; periksa ketentuan yt-dlp, mpv/libmpv, Node.js, dan dependency Android sebelum redistribusi.
=======
Dependency dan runtime pihak ketiga tetap mengikuti lisensinya masing-masing; periksa ketentuan yt-dlp, mpv/libmpv, Node.js, dan dependency Android sebelum redistribusi.
>>>>>>> f6bf7343fc3b0539619337f9a82238d16aae6d99
