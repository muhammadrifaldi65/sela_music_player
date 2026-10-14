# Aplikasi musik Windows — spesifikasi desain

## Tujuan dan keputusan pengguna

Aplikasi desktop Windows yang dapat di-install untuk mendengarkan musik dari YouTube tanpa iklan yang disisipkan aplikasi. Backend Rust berjalan lokal. Langsung masuk mode guest tanpa pendaftaran, login, email, atau akun Google/YouTube. Playlist, favorit, dan histori disimpan di perangkat pengguna. Aplikasi utama tidak memerlukan server milik operator. Windows saja; Linux dan macOS di luar cakupan.

## Arsitektur

- Tauri 2 sebagai shell desktop; React dan TypeScript untuk UI.
- Rust desktop mengelola libmpv, proses yt-dlp, SQLite, kredensial Windows, serta integrasi OS.
- Tidak ada server akun, API autentikasi, SMTP, atau sinkronisasi cloud. SQLite desktop menjadi sumber data playlist, favorit, histori, dan pengaturan pada profil Windows aktif.
- Listen Together dihapus atas keputusan pengguna. Tidak ada relay, ruang dengar, atau sinkronisasi antarklien.
- yt-dlp dibundel sebagai executable pendamping untuk pencarian dan resolusi URL media YouTube. libmpv dibundel dengan dependensi runtime dan pemberitahuan lisensinya.
- Media diambil langsung dari sumber oleh desktop, tidak melalui server aplikasi dan tidak disimpan sebagai koleksi hasil unduhan.

## Identitas dan keamanan

Guest adalah profil lokal persisten, bukan sesi sementara. Tidak ada password, sesi aplikasi, pendaftaran, verifikasi email, atau reset password. Data ditempatkan pada direktori data aplikasi milik pengguna Windows; tidak ada sinkronisasi lintas perangkat. Token layanan eksternal opsional disimpan di Windows Credential Manager.

Token dan URL media bertanda tangan tidak masuk log. Pemanggilan yt-dlp menggunakan argument array, tanpa shell; input YouTube dibatasi ke ID/URL host yang didukung. Cover art dibatasi ukuran dan format gambar; nama berkas sumber bukan path penyimpanan. Perintah Tauri dibatasi kapabilitasnya; konten jarak jauh tidak diberi akses perintah native.

## Katalog dan playback

Pencarian YouTube dengan preview saat mengetik, pembatalan hasil pencarian lama, serta tampilan channel dan playlist publik. Album/artis ditampilkan hanya jika metadata sumber mendukung identifikasi yang benar; tidak mengarang tipe katalog. Tidak memakai feed personal YouTube Music. Tampilan beranda menggunakan histori/favorit aplikasi dan konten publik yang tersedia, dengan label sumber yang jelas.

Resolusi URL media dilakukan ketika diperlukan; URL bertanda tangan tidak dijadikan identitas permanen. Playlist menyimpan ID video dan metadata tampilan. libmpv menyediakan playback, seek, volume, gapless melalui persiapan item berikutnya, dan normalisasi berdasarkan metadata loudness jika tersedia. Tidak menjanjikan normalisasi akurat jika sumber tidak menyediakan metadata yang diperlukan. Video opsional tampil pada area artwork tanpa memulai ulang audio ketika teknis memungkinkan; perubahan sumber yang membutuhkan reload mempertahankan posisi dan ditampilkan secara jujur.

Tidak ada iklan yang disisipkan aplikasi. Integrasi YouTube tidak resmi, dapat berubah atau diblokir. Konten privat, berbayar, dibatasi usia/wilayah tidak dijamin tersedia. Kesalahan akses ditampilkan kepada pengguna; tidak ada bypass akses atau sukses palsu.

## Library, playlist, dan histori

Pengguna dapat membuat, mengubah, menghapus playlist; mengatur judul/deskripsi/cover art; menambah, menghapus, dan mengurutkan lagu. Favorit lagu, koleksi album/artis berdasarkan metadata yang tersedia, serta histori playback disimpan sebagai data aplikasi sendiri. Histori dikelompokkan menurut hari di zona waktu pengguna, bukan bucket YouTube Music. Tidak ada subscribe channel, uploads akun YouTube, liked songs YouTube, atau pengeditan playlist YouTube.

SQLite desktop menyimpan playlist, favorit, histori, pengaturan, cache metadata, indeks file lokal, posisi playback, dan antrean. Perubahan persisten memakai transaksi dan kegagalan tulis dilaporkan. Data tetap tersedia setelah restart; tanpa internet pengguna tetap dapat mengelola playlist dan memutar file lokal. Streaming YouTube membutuhkan internet. Library file lokal tidak mengunggah file maupun path pribadi ke server.

## Antrean dan mode pemutar

Antrean mendukung drag reorder, hapus, play next, shuffle, repeat, dan pemulihan antar-restart. Penyimpanan state atomik. Radio/kelanjutan otomatis memakai rekomendasi publik jika tersedia, menghindari duplikasi dekat, dan berhenti dengan status jelas jika sumber tidak tersedia. Mini player, theater/fullscreen dengan cover dan lirik berdampingan. File lokal mempertahankan metadata sumber; aplikasi tidak menulis ulang berkas audio.

## Lirik

Panel lirik tersinkron, auto-scroll, klik untuk seek. Timing per kata dan terjemahan hanya jika penyedia menyediakan datanya; lirik per baris tidak diberi timing kata buatan. Adapter penyedia harus diperiksa cakupan, izin penggunaan, dan bentuk responsnya sebelum implementasi. Tidak mengklaim ketersediaan lirik untuk semua lagu. Jika tidak tersedia, UI menyatakan tidak tersedia.

## Integrasi

- Last.fm: autentikasi resmi, now-playing dan scrobble mengikuti persyaratan durasi layanan; deduplikasi saat retry.
- ListenBrainz: token pengguna disimpan aman; pengiriman listen mengikuti kontrak layanan.
- Discord Rich Presence: artwork, progres, toggle; kegagalan Discord tidak menghentikan musik.
- Windows SMTC: metadata, playback, seek sesuai kemampuan OS; media keys dan tombol taskbar preview.
- Tray: close-to-tray dapat dikonfigurasi, play/pause/skip, keluar eksplisit, start-on-login opsional.

## UI dan aksesibilitas

Ctrl+K pencarian, Ctrl+H daftar shortcut, menu konteks, Ctrl+wheel zoom. Navigasi keyboard, focus visible, label aksesibel, dan kontras memadai. Palet aksen, warna kustom, font pengguna, radius sudut, ikon aplikasi kustom pada permukaan runtime yang didukung Windows, serta tema adaptif dari artwork. Ikon executable installer tidak diklaim dapat diubah oleh pengaturan runtime.

Bahasa target: Inggris, Jerman, Spanyol, Prancis, Indonesia, Italia, Jepang, Korea, Polandia, Portugis Brasil, Rumania, Rusia, Tamil, Turki, Ukraina, Vietnam, Mandarin Sederhana dan Tradisional. Tidak menganggap string Inggris yang disalin sebagai terjemahan selesai. Terjemahan dibundel lokal; Weblate bukan dependensi runtime dan hanya opsional untuk alur kontribusi.

## Distribusi dan operasi

Installer Windows setup.exe membundel runtime native yang diperlukan. Update Tauri memakai manifest dan artefak bertanda tangan; kunci privat hanya di proses rilis, tidak di aplikasi. Update yt-dlp didistribusikan melalui rilis aplikasi, bukan mengunduh executable sembarang saat runtime. Penandatanganan Authenticode terpisah dari tanda tangan updater.

Tidak memerlukan hosting backend, domain aplikasi, SMTP, atau database server. Auto-update tetap membutuhkan lokasi distribusi manifest/artefak, misalnya GitHub Releases, dan kunci updater; ini bukan server akun. Installer dapat dibagikan manual tanpa menjalankan layanan sendiri. Kredensial Last.fm diperlukan untuk integrasi opsional; sertifikat Authenticode diperlukan jika menginginkan publisher Windows terverifikasi. Prasyarat ini tidak dipalsukan dengan mock.

## Urutan implementasi

1. Rust/Tauri, playback nyata YouTube dan file lokal, pencarian, antrean persisten.
2. Library lokal, playlist lengkap, favorit dan histori guest persisten.
3. Lirik, video, mini player/theater, kustomisasi dan lokalisasi.
4. Scrobbling, Discord dan integrasi Windows.
5. Packaging, updater, pemeriksaan end-to-end dan dokumentasi operasi.

Urutan bukan pengurangan cakupan. Setiap tahap harus menggunakan implementasi nyata, tanpa placeholder yang dinyatakan selesai.

## Verifikasi penerimaan

- Instal pada Windows dan jalankan tanpa toolchain pengembangan; buktikan playback lagu publik yang dapat diakses dan file lokal, seek, pause/resume, serta transisi antrean.
- Tutup/restart aplikasi; antrean dan pengaturan pulih. Uji close-to-tray dan keluar eksplisit.
- Peluncuran pertama langsung membuka pemutar tanpa login atau koneksi backend; profil guest tetap persisten setelah restart.
- Buat/edit/hapus playlist beserta cover; reorder lagu; restart memastikan data tetap benar, termasuk saat offline.
- Playback gagal, jaringan putus, media kedaluwarsa, penyimpanan lokal gagal: tampilkan kesalahan nyata tanpa kehilangan data tersimpan.
- Uji lirik bertiming, lirik tidak tersedia, video, shortcut, keyboard-only, mini player/theater pada UI aktual.
- Buktikan scrobble tidak ganda, toggle Discord, media keys/SMTC, taskbar dan tray pada Windows.
- Paket update valid diterima, tanda tangan tidak valid ditolak; rahasia tidak masuk bundle/log.
- Pengujian integrasi sumber eksternal dicatat dengan hasil aktual; kompilasi atau mock saja bukan bukti fitur bekerja.
