# Sela: penyegaran UI dan Android

## Status dan cakupan

Arsitektur disetujui pengguna; dokumen ini menunggu tinjauan sebelum rencana implementasi. Target pengujian pengguna: Windows dan ponsel POCO/OPPO Android 12 ke atas. Deliverable: installer Windows .exe dan APK ARM64 bertanda tangan yang dapat dipasang langsung. Bukan APK pembungkus halaman tanpa pemutar nyata.

Dokumen ini memperluas batas Windows-only pada spesifikasi music-desktop-design. Kebutuhan lama yang belum selesai tidak dinyatakan selesai oleh rilis ini. Tidak menambahkan akun, server aplikasi, sinkronisasi antarperangkat, atau publikasi Play Store.

## Pengalaman pengguna

- Identitas Sela tetap; susunan katalog mengikuti YouTube Music: cover persegi dan carousel untuk rilisan, baris lagu ringkas untuk Trending dan Putar lagi, judul/artis terbaca, aksi tambahan dalam menu berlabel aksesibel.
- New releases dan Trending tetap berdasarkan kategori sumber asli. Jangan memotong rekomendasi acak lalu mengganti label. Error dan kategori kosong ditampilkan jujur.
- Desktop: navbar kiri tetap, area konten menggulir, player tetap di bawah. Android: navigasi sentuh ringkas, safe-area dan keyboard tidak menutup kontrol; tidak memaksakan lebar sidebar desktop.
- Library mengikuti alur Spotify: kartu dan daftar playlist dapat diklik, halaman detail dengan cover, judul, jumlah lagu, Putar semua, daftar lagu dan menu tindakan. Combobox pembuka playlist dihapus.
- Tambah ke playlist memakai dialog/menu daftar playlist, bukan select bawaan. Mendukung membuat playlist, tambah/hapus lagu, ubah nama, hapus playlist dengan konfirmasi, serta mengubah urutan. Reorder mempunyai kontrol sentuh/keyboard selain drag.
- Putar semua mengganti antrean aktif dengan urutan playlist secara atomik dan mulai dari lagu pertama; playlist kosong tidak memulai playback. Kegagalan persiapan lagu terlihat, tidak menampilkan sukses palsu.
- Playlist/riwayat tetap lokal, persisten setelah restart, independen antara Windows dan Android. Data pengguna Windows yang sudah ada tidak dihapus.

## Arsitektur

Tauri 2 dan React/TypeScript tetap menjadi UI bersama. Pemisahan platform hanya untuk fungsi yang memang berbeda. Kode katalog dan SQLite yang portabel dipakai bersama; API UI tetap mempertahankan identitas lagu dan entri antrean.

Windows mempertahankan libmpv, yt-dlp dan Node pendamping. Resource .exe/.dll, tray, Discord IPC dan integrasi Windows hanya dibangun di Windows, tidak masuk APK.

Android memakai integrasi Kotlin Media3 ExoPlayer, MediaSession dan foreground media playback service. Service memiliki state playback dan antrean aktif agar musik, next/previous, repeat/autoplay tidak bergantung pada JavaScript WebView yang ditangguhkan. UI membaca snapshot dan menerima perubahan dari native. Layar terkunci/berpindah aplikasi tidak memulai player kedua. Audio focus, interupsi panggilan, headphone terlepas, pause/resume dan penghentian eksplisit ditangani native. Foreground service tidak menjanjikan bertahan setelah force-stop oleh OS/pengguna.

Ekstraksi pencarian/stream Android berjalan lokal melalui dependensi Android yang dipelihara, bukan executable Windows atau proxy pihak ketiga. Pemilihan paket dan versi adalah gerbang riset pertama pada rencana implementasi: buktikan dukungan ABI ARM64, Android API 31+, resolusi URL stream nyata, dependensi runtime dan hak redistribusi sebelum mengintegrasikan. Kegagalan kandidat tidak boleh diganti stub atau stream contoh. URL stream berumur pendek diselesaikan saat diperlukan; ID video tetap identitas persisten.

## Penyimpanan, file dan keamanan

SQLite disimpan di sandbox masing-masing platform. Operasi reorder dan penggantian antrean memakai transaksi. Native service dan UI tidak mempunyai dua sumber state antrean yang bertentangan; antrean persisten menjadi sumber urutan, service menjadi sumber posisi/status pemutaran.

File Android dipilih melalui Storage Access Framework, bukan path Windows atau izin akses seluruh penyimpanan. URI yang dipilih pengguna dipertahankan izinnya bila tersedia; akses yang dicabut menghasilkan error. Tidak meminta izin mikrofon atau kontak.

Terapkan deklarasi foreground service/media playback dan izin notifikasi sesuai versi Android; izin ditolak tidak merusak data atau memalsukan status playback. Validasi ID/URL dan batas ukuran/waktu pada input jaringan. Jangan mencatat URL stream bertanda tangan, token, atau path pribadi. Konten remote tidak memperoleh akses bridge Tauri.

## Build dan distribusi

- Android minimum API 31 (Android 12), ABI arm64-v8a. Target SDK mengikuti toolchain Tauri/Media3 yang kompatibel saat implementasi dan dicatat di konfigurasi.
- Siapkan JDK, SDK, NDK dan Rust target kompatibel. Pemeriksaan awal hanya menemukan Rust Windows; SDK tidak ditemukan di lokasi standar dan variabel SDK/JAVA_HOME kosong.
- APK uji ditandatangani dengan kunci pengujian yang konsisten, tidak diklaim sebagai rilis Play Store. Kunci/password tidak dimasukkan ke repositori. Laporkan identitas build, versi, checksum dan lokasi artefak. Installer Windows tidak menimpa aplikasi terpasang tanpa tindakan pengguna.
- Distribusikan pemberitahuan lisensi dependensi yang dibundel. Dependency dengan ketentuan tak terpenuhi tidak disisipkan diam-diam.

## Verifikasi penerimaan

1. Build frontend, pemeriksaan Rust, build Windows dan Android selesai tanpa error; warning yang tersisa dicatat.
2. Jalankan installer Windows hasil build pada runtime aktual; Home, pencarian, playlist dan playback nyata bekerja tanpa dev server.
3. Instal APK pada emulator/perangkat Android 12+ yang tersedia. Buktikan bukan hanya build sukses: pencarian nyata, putar dari kedua kategori, pause/resume, seek, next/previous dan Putar semua.
4. Buat playlist, tambah/hapus/reorder lagu, ubah nama, restart; isi dan urutan bertahan. Tidak ada combobox dalam alur pembuka/tambah playlist.
5. Android: pindah aplikasi dan kunci layar; audio berlanjut, notifikasi/layar kunci mengendalikan sesi yang sama. Uji audio focus/headphone dan pemulihan UI saat kembali.
6. Jaringan gagal, item tidak dapat diakses, file tidak lagi diizinkan dan playlist kosong menghasilkan state jelas, tidak menghapus data atau menjalankan fallback palsu.
7. Inspeksi visual desktop dan layar ponsel: konten tidak meluber, kontrol terjangkau, fokus/label aksesibel, navigasi/player tidak tertutup.
8. Berikan installer .exe dan APK bertanda tangan berikut checksum dan petunjuk pemasangan singkat. Pisahkan bukti emulator dari uji POCO/OPPO fisik; jangan mengklaim perangkat fisik teruji jika tidak tersedia.

## Batas dan risiko

YouTube Music adalah integrasi tidak resmi; kategori dan ekstraksi bisa berubah. Pemilihan extractor Android perlu bukti runtime, tidak cukup kompatibilitas kompilasi. Pembatasan baterai vendor POCO/OPPO dapat memengaruhi service; hasil emulator tidak membuktikan perilaku vendor. Pengujian perangkat pengguna adalah verifikasi tambahan, bukan pengganti smoke Android yang tersedia di lingkungan pengembangan.
