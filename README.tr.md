<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-animated-dark.svg">
    <img src="assets/logo-animated.svg" alt="panosync logosu: kilitli bir veri bir panodan diğerine gidiyor" width="168">
  </picture>
</p>

<h1 align="center">panosync</h1>

<p align="center"><b>Bir bilgisayarda kopyala, diğerinde yapıştır. Uçtan uca şifreli.</b></p>

<p align="center">
  <a href="https://github.com/msbayindir/panosync/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/msbayindir/panosync/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/msbayindir/panosync/releases/latest"><img alt="Son sürüm" src="https://img.shields.io/github/v/release/msbayindir/panosync?sort=semver"></a>
  <a href="#lisans"><img alt="Lisans: MIT veya Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue"></a>
  <img alt="Platformlar: macOS, Linux, Windows" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-informational">
</p>

<p align="center"><a href="README.md">English</a> · <b>Türkçe</b> · <a href="https://msbayindir.github.io/panosync/">Web sitesi</a></p>

<p align="center"><img src="docs/demo.gif" alt="tmux içinde bir aracı sunucu ve iki istemci: dizüstünde kopyalanan metin masaüstüne gidiyor, duraklatma sırasında kopyalanan parola gönderilmiyor" width="900"></p>
<p align="center"><sub><a href="docs/demo.md">VHS</a> ile yapılmış terminal kaydı: yerel bir aracı sunucu ve ayrı panoları olan iki istemci.</sub></p>

```bash
brew install msbayindir/tap/panosync      # macOS ve Linux; Homebrew yoksa:
curl -fsSL https://raw.githubusercontent.com/msbayindir/panosync/main/install.sh | sh
```

**panosync**, macOS, Linux ve Windows bilgisayarların arasında panoyu (clipboard) senkronize eder; cihazlar
farklı ağlarda olabilir. Her şey cihazlarında uçtan uca şifrelenir; aradaki, senin barındırdığın küçük aracı
sunucu sadece okuyamadığı baytları iletir.

- **Her tarafta tek küçük dosya.** İstemci 3–4 MB'lık, çalışma zamanı gerektirmeyen tek bir dosya; aracı sunucu yaklaşık 1,3 MB.
- **Tek adımda kendi sunucunda.** Coolify'ın (ya da herhangi bir Docker sunucusunun) olduğu gibi derlediği bir Dockerfile veya `ghcr.io`'da hazır imaj.
- **Hesap yok, web paneli yok.** İlk cihazdaki tek komut, diğer cihazlarda çalıştırılacak tek komutu yazar.
- **Bilerek sadece metin.** Bir makinede kopyala, diğerinde yapıştır. Bu kadar.

> Proje ilk sürümlerinde **pano** adını taşıyordu; o sürümler buradaki v1 protokolüyle konuşmaz.

**İçindekiler:** [Nasıl çalışır](#nasıl-çalışır) · [Hızlı kurulum](#hızlı-kurulum) · [Komutlar](#komutlar) ·
[Güvenlik modeli](#güvenlik-modeli) · [Alternatiflerle karşılaştırma](#alternatiflerle-karşılaştırma) ·
[Sorun giderme](#sorun-giderme) · [Geliştirme](#geliştirme) · [Lisans](#lisans)

## Nasıl çalışır

```
   macOS                     aracı sunucu (senin sunucun)                  Ubuntu
 ┌───────────────┐   wss://      ┌─────────────────────┐   wss://      ┌───────────────┐
 │ Cmd+C         │               │                     │               │               │
 │ anahtarınla   │ ────────────► │ şifreli veriyi aynı │ ────────────► │ şifreyi çöz,  │
 │ şifrele       │               │ kanaldaki diğer     │               │ panoya yaz    │
 │               │ ◄──────────── │ cihazlara iletir    │ ◄──────────── │               │
 └───────────────┘               │                     │               └───────────────┘
                                 │ görür: IP'ler,      │
     Windows ◄─────────────────► │ zamanlama, boyut    │
                                 │ asla: anahtar ya da │
                                 │ metnin kendisi      │
                                 └─────────────────────┘
```

1. İlk cihazda `panosync setup` rastgele 32 baytlık bir anahtar üretir.
2. Her cihaz bu anahtardan (BLAKE3 ile) bir **şifreleme anahtarı** ve ayrı bir **kanal kimliği** türetir.
3. Cihazlar aracıya `/ws?channel=<kanal kimliği>` adresinden bağlanır. Aracı bağlantıları kanal kimliğine
   göre gruplar ve her mesajı o kanaldaki diğer cihazlara iletir.
4. Her mesaj cihazdan çıkmadan önce XChaCha20-Poly1305 ile şifrelenir.

Protokolün tamamı ve tehdit modeli: [docs/architecture.md](docs/architecture.md) (İngilizce).

İki program var:

| Program | Nerede çalışır | Ne yapar |
|---|---|---|
| `panosync` | macOS / Linux / Windows bilgisayarların | panoyu izler, değişiklikleri şifreleyip gönderir, gelen metni panoya yazar |
| `panosync-relay` | bir sunucu (Docker, Coolify, herhangi bir VPS) | aracı sunucu |

## Hızlı kurulum

`wss://` (TLS üzerinden WebSocket) ile erişilebilen bir aracı sunucuya ve her makinede istemciye ihtiyacın var.

### 1. Aracı sunucuyu çalıştır

Aracı, 8080 portunda düz `ws://` konuşur ve TLS için önünde bir ters vekil (reverse proxy) bekler. Kısa hali
aşağıda; Coolify, Docker, systemd, Caddy, nginx ve sadece yerel ağ seçeneklerinin hepsi, neyle test
edildikleriyle birlikte [docs/self-hosting.md](docs/self-hosting.md) belgesinde (İngilizce).

**Coolify** (TLS'i Coolify halleder)

1. Bu depoyu fork'la (ya da Coolify'ı doğrudan bu depoya yönlendir) ve ondan yeni bir kaynak oluştur.
2. **Build Pack:** `Dockerfile` · **Ports Exposes:** `8080` · **Domains:** `https://panosync.example.com`
3. Deploy et, sonra `https://panosync.example.com/health` adresini aç. `ok` yazmalı.

Alan adın yoksa Coolify'ın `sslip.io` adresleri de çalışır (ör. `https://panosync.1.2.3.4.sslip.io`).

**Docker**

```bash
docker run -d --name panosync-relay --restart unless-stopped -p 8080:8080 ghcr.io/msbayindir/panosync-relay:latest
```

Önüne herhangi bir TLS ters vekili koy. [Caddy](https://caddyserver.com) ile bütün ayar şu kadar:

```
panosync.example.com {
    reverse_proxy localhost:8080
}
```

nginx'te WebSocket `Upgrade` başlıkları şart; bkz. [docs/self-hosting.md](docs/self-hosting.md#https-with-nginx).

**Kaynaktan:** `cargo run --release -p panosync-relay` (portu değiştirmek için `PORT`).

### 2. İstemciyi kur

**Homebrew (macOS ve Linux)**

```bash
brew install msbayindir/tap/panosync
```

Formula, release sayfasındaki hazır binary'leri kurar; `brew services start panosync` panosync'i oturum
açılınca başlatır.

**Kurulum betiği (macOS ve Linux)**

```bash
curl -fsSL https://raw.githubusercontent.com/msbayindir/panosync/main/install.sh | sh
```

Betik işletim sistemine ve işlemcine uygun binary'yi seçer, SHA-256 özetini release'teki `SHA256SUMS` ile
karşılaştırır ve `~/.local/bin` içine kurar (`PANOSYNC_INSTALL_DIR=/usr/local/bin` ile yeri, `PANOSYNC_VERSION=v0.1.0`
ile sürümü değiştirebilirsin). İstersen önce oku: [install.sh](install.sh).

**Windows:** [son sürümden](https://github.com/msbayindir/panosync/releases/latest) `panosync-x86_64-pc-windows-msvc.zip`
dosyasını indir, aç ve `panosync.exe`'yi `PATH` içindeki bir klasöre koy.

**Elle indirme:** her release'te `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu` ve `x86_64-pc-windows-msvc` arşivleri ile `SHA256SUMS` var. Binary'ler kod
imzalı değil. macOS'ta tarayıcıyla indirdiysen karantina işaretini bir kez kaldır: `xattr -d com.apple.quarantine ./panosync`.

**cargo-binstall:** `cargo binstall --git https://github.com/msbayindir/panosync panosync` platformuna uygun
hazır binary'yi release'ten indirir (panosync crates.io'da olmadığı için `--git` gerekiyor).

**Kaynaktan:** `cargo install --git https://github.com/msbayindir/panosync panosync` (Rust 1.88+).

### 3. Cihazları eşleştir

**İlk** cihazda kurulumu anahtar vermeden yap; yeni bir anahtar üretilir:

```bash
panosync setup --server wss://panosync.example.com
```

Çıktı, **diğer** her cihazda çalıştıracağın komutu aynen yazar:

```bash
panosync setup --server wss://panosync.example.com --key <anahtarın>
```

> Bu anahtarı bilen herkes panonu okuyabilir. Sohbet ya da e-posta yerine [croc](https://github.com/schollz/croc),
> [LocalSend](https://localsend.org) gibi bir araçla aktar ya da elle yaz. `panosync key` anahtarı sonradan tekrar gösterir.

### 4. Çalıştır

```bash
panosync
```

Bir makinede bir şey kopyala, diğerinde yapıştır. Günlükte `sent (26 bytes)` ve `received (26 bytes)` satırları
görünür. **Panonun içeriği günlüklere hiçbir zaman yazılmaz, sadece uzunluğu yazılır.**

**Oturum açınca başlatma**

- **Homebrew (macOS ve Linux):** `brew services start panosync`.
- **Linux (systemd):** [packaging/panosync.service](packaging/panosync.service) dosyasının başındaki adımlar.
- **macOS (launchd):** [packaging/com.msbayindir.panosync.plist](packaging/com.msbayindir.panosync.plist) dosyasının başındaki adımlar.
- **Windows:** <kbd>Win</kbd>+<kbd>R</kbd> ile `shell:startup` klasörünü aç ve içine `panosync.exe`'ye bir kısayol koy.

## Komutlar

| Komut | Türkçe karşılığı | Ne yapar |
|---|---|---|
| `panosync` / `panosync start` | `baslat` | Senkronizasyonu başlatır |
| `panosync setup --server <url> [--key <anahtar>]` | `kurulum --sunucu … --anahtar …` | Cihazı ayarlar; `--key` verilmezse yeni anahtar üretir |
| `panosync key` | `anahtar` | Başka bir cihaz eklemek için anahtarı gösterir |
| `panosync config` | `ayar` | Ayar dosyasının yolunu gösterir |
| `panosync pause <dakika>` | `duraklat` | Çalışan istemci o süre boyunca pano değişikliklerini göndermez; `panosync pause 0` devam ettirir |

`pause`, örneğin parola kopyalayacağın anlar için: duraklatma sırasında kopyalanan metin, duraklatma bittikten
sonra da gönderilmez. Diğer cihazlardan gelen metin yine panoya yazılır. Bitiş zamanı ayar dosyasının yanındaki
`paused_until` dosyasında tutulur; komut `panosync` çalışırken de çalışmıyorken de verilebilir.

Ayar klasörü Linux'ta `~/.config/panosync`, macOS'ta `~/Library/Application Support/panosync`, Windows'ta
`%APPDATA%\panosync`. Başka bir klasör için `PANOSYNC_CONFIG_DIR` (örneğin bir makinede iki istemci çalıştırmak için).

Ayrıntılı günlük için `RUST_LOG=debug`.

## Güvenlik modeli

**Neler korunuyor**

- **Metnin gizliliği ve bütünlüğü.** Her mesaj cihazlarından hiç çıkmayan bir anahtarla XChaCha20-Poly1305
  (rastgele 24 baytlık nonce) kullanılarak şifrelenir. Değiştirilmiş, kesilmiş ya da başka anahtarla şifrelenmiş mesaj reddedilir.
- **Aracı anahtarı hiç öğrenmez.** Şifreleme anahtarı ve kanal kimliği anahtarından BLAKE3 ile farklı bağlamlarda
  türetilir; aracının gördüğü kanal kimliği şifreleme anahtarı hakkında bir şey söylemez ve anahtarı bilmeyen biri tahmin edemez.
- **Tekrar oynatma.** Mesajlar rastgele bir kimlik ve zaman damgası taşır; tekrar eden kimlikler ve 5 dakikadan eski mesajlar reddedilir.
- **İletim.** Bağlantılar uçtan uca şifrelemenin üstüne ayrıca TLS (`wss://`) kullanır.
- **Ayar dosyası.** macOS ve Linux'ta anahtar dosyası `600` izniyle oluşturulur (sadece senin kullanıcın okuyabilir).
- **Aracıdaki kötüye kullanım sınırları.** Mesaj başına 1 MB, kanal başına 16 cihaz, bağlantı başına 10 saniyede 50 mesaj.

**Bilinen sınırlar**

- **Aracı üst verileri görür.** Panonu okuyamaz, ama bağlanan IP adreslerini, mesajların ne zaman gönderildiğini
  ve yaklaşık boyutlarını (yani kopyalanan metnin kabaca uzunluğunu) görebilir. Zamanla değişmeyen ve cihazlarını
  birbirine bağlayan kanal kimliğini de görür.
- **Tek ortak anahtar, ileri gizlilik yok.** Bütün cihazlar tek bir sabit anahtarı paylaşır. Bir cihaz ele geçirilirse
  kaydedilmiş geçmiş trafik de gelecekteki trafik de çözülebilir ve cihaz bazında iptal yoktur: `panosync setup --server …`
  ile yeni anahtar üretip bütün cihazları yeniden kur.
- **Parolalar da senkronize olur.** Şifre yöneticisinden kopyaladıkların dahil her şey diğer cihazlarına (şifreli) gider.
  Şifre yöneticisi algılaması henüz yok; istemediğin anlarda `panosync pause <dakika>` kullan ya da `panosync`'i durdur.
- **Teslimatı aracı kontrol eder.** Kötü niyetli bir aracı mesajları düşürebilir, geciktirebilir ve 5 dakikalık pencere
  içinde, o arada yeniden başlamış bir cihaza yakın tarihli bir mesajı tekrar gönderebilir (görülen mesaj kimlikleri bellekte tutulur).
- **Saatler aşağı yukarı doğru olmalı.** Saati 5 dakikadan fazla sapan cihazın mesajları reddedilir.
- **Sadece metin.** Resim ya da dosya yok; 512 KB'tan büyük metin gönderilmez.

**Windows notu.** panosync Windows'ta dosya izinlerini kendisi ayarlamaz. Ayar dosyası `%APPDATA%\panosync\config.toml`
konumundadır ve varsayılan olarak kullanıcı profilinin izinlerini (senin hesabın, SYSTEM ve Administrators) devralır.
`icacls "%APPDATA%\panosync\config.toml"` ile kontrol edebilirsin. `%APPDATA%` *dolaşan* (roaming) profildir;
dolaşan profili olan bir etki alanı bilgisayarında anahtar etki alanının profil sunucusuna kopyalanabilir.

Bir güvenlik açığı mı buldun? [SECURITY.md](SECURITY.md).

## Alternatiflerle karşılaştırma

panosync tek bir iş yapar. Başka araçlar çok daha fazlasını yapıyor ve sana onlardan biri daha uygun olabilir.
Özellikle [ClipCascade](https://github.com/Sathvik-Rao/ClipCascade) çok daha fazla özelliği olan, daha olgun ve daha eksiksiz bir proje.

| | panosync | ClipCascade | KDE Connect | Apple Universal Clipboard |
|---|---|---|---|---|
| Platformlar | macOS, Linux, Windows | Windows, macOS, Linux (arayüzlü ve CLI), **Android**; iOS yapılacaklar listesinde | Linux, Windows, macOS, **Android, iOS**, Plasma Mobile, SailfishOS | Mac, iPhone, iPad, Apple Vision Pro |
| İçerik | sadece metin | **metin, resim ve dosya** | pano, birçok özellikten biri (dosya paylaşımı, bildirim senkronu, sanal touchpad, …) | metin, resim, fotoğraf, video, dosya |
| Farklı ağlar | evet, kendi aracın üzerinden | evet, sunucusu üzerinden; isteğe bağlı **eşler arası (P2P) mod** (sunucu o zaman sadece giriş ve sinyalleşmeyi yapar) | cihazlar yerel ağda doğrudan konuşur; diğer durumlar için VPN | cihazlar yakın olmalı; Bluetooth, Wi-Fi ve Handoff açık |
| Ne çalıştırırsın | durumsuz tek aracı binary'si ya da konteyneri, tek değişkenle (`PORT`) | kullanıcı hesapları, web paneli ve veritabanı volume'ü olan Java 21 sunucusu (JAR ya da Docker imajı); ücretsiz bir topluluk sunucusu da sunuluyor | hiçbir şey | hiçbir şey |
| Kimlik | tek ortak anahtar, hesap yok | kullanıcı adı ve parola, sunucu başına birden çok kullanıcı | cihazlar arası eşleştirme | aynı Apple hesabı |
| Şifreleme | uçtan uca, her zaman açık | uçtan uca, varsayılan olarak açık | cihazlar arasında TLS | (burada ele alınmadı) |
| Ekstralar | `pause` komutu | cihaz başına gönder/al yönü, bildirimler, ayar arayüzü | birçok eklenti | işletim sistemine gömülü |
| Lisans | MIT veya Apache-2.0 | GPL-3.0 | GPL-2.0 / GPL-3.0 | kapalı kaynak |

panosync'in sunduğu fark esas olarak **hafiflik ve sadelik**:

- **Hafif:** istemci 3–4 MB'lık tek bir dosya, aracı yaklaşık 1,3 MB (release derlemeleri: istemci macOS arm64'te 3,0 MB,
  Linux arm64'te 3,7 MB); çalışma zamanı, veritabanı ya da anahtar dışında ayar yok.
- **Kendi sunucunda kolay:** aracı, Coolify'ın olduğu gibi derlediği tek bir Dockerfile, tek bir `docker run` ya da tek bir
  systemd birimi. Durum tutmadığı için yedeklenecek ya da taşınacak bir şey yok.
- **Kullanımı sade:** hesap yok, ayar ekranı yok; ilk cihazdaki `panosync setup` diğerleri için tek komutu yazar.
- **Denetlenebilecek kadar küçük:** protokol [tek sayfaya](docs/architecture.md) sığıyor.

Android, resim ya da dosya, bir arayüz veya tek sunucuda birden çok kullanıcı gerekiyorsa **ClipCascade'i seç**.
Cihazların aynı ağdaysa ve telefon entegrasyonu da istiyorsan **KDE Connect'i seç**. Bütün cihazların Apple ise
**Universal Clipboard'u kullan**. Farklı ağlardaki masaüstü bilgisayarlar arasında metin senkronu, kendi barındırdığın ve
bir öğleden sonrada okuyabileceğin bir aracıyla istiyorsan **panosync'i seç**.

Kaynaklar (Kasım 2026'da kontrol edildi): [ClipCascade README](https://github.com/Sathvik-Rao/ClipCascade) (v3.5.1),
[KDE Connect README](https://invent.kde.org/network/kdeconnect-kde) ve [indirme sayfası](https://kdeconnect.kde.org/download.html),
Apple'ın [Universal Clipboard destek makalesi](https://support.apple.com/en-us/102430).

## Sorun giderme

**Hiçbir şey gelmiyor / bağlanamıyor**
- `https://<aracın>/health` adresi `ok` dönüyor mu?
- Adres `wss://` ile mi başlıyor? (`ws://` sadece TLS olmadan, ör. yerel denemede çalışır.)
- İki makinenin saati de otomatik ayarlı mı?
- `RUST_LOG=debug panosync` ile çalıştır.

**Linux (Wayland):** panosync, birleştirici (compositor) destekliyorsa Wayland data-control protokolünü, desteklemiyorsa
XWayland'i kullanır. Ubuntu'da GNOME + Wayland (XWayland üzerinden) ile test edildi. Kopyalama ya da yapıştırma
çalışmıyorsa dağıtımını, masaüstünü ve günlük çıktısını yazarak bir issue aç.

**macOS izin istiyor:** macOS sürümüne göre terminalin ya da `panosync` için pano erişim izni istenebilir. İzin ver.

## Geliştirme

```bash
cargo test                       # bütün testler, gerçek bir aracıya karşı uçtan uca test dahil
cargo clippy --all-targets
cargo run -p panosync-relay      # 8080 portunda yerel aracı
```

Yerel deneme için istemciyi `--server ws://127.0.0.1:8080` ile kur. Klasör yapısı, mimari ve katkı kuralları için
[README.md](README.md#development), [docs/architecture.md](docs/architecture.md) ve [CONTRIBUTING.md](CONTRIBUTING.md).

## Lisans

[Apache Lisansı 2.0](LICENSE-APACHE) ya da [MIT lisansı](LICENSE-MIT); hangisini istersen.

Aksini açıkça belirtmedikçe, bu projeye eklenmek üzere gönderdiğin her katkı (Apache-2.0'daki tanımıyla) ek bir koşul
olmadan yukarıdaki gibi çift lisanslı kabul edilir.
