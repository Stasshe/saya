# saya: OS標準パッケージマネージャの薄いラッパー

## Context

chezmoi(dotfiles管理)、mise(開発ツール管理)に対し、OSパッケージ管理(APT/pacman)の「意図記録」と「再現」担うツール無い。`apt-mark showmanual`等あるが、OS導入時基盤パッケージも混在、ユーザー意思で入れた分との区別弱い。

sayaは大規模統合パッケージマネージャでなく、OS標準パッケージマネージャに以下2つだけ足す薄いラッパー。

- **意図記録**: `sudo apt install foo`の使用感変えず、直接指定パッケージ名だけ自動でマニフェストへ記録(依存解決分は記録しない)。
- **一方向適用**: マニフェストにあって未インストールのものだけインストール。マニフェストから消えてもアンインストールしない。

### 確定設計判断(ユーザー対話で確定)

- **manifest保存先とsudo記録**: 設定ファイルは実行ユーザーの`~/.config/saya/packages.toml`に保存する。`sudo`経由では`SUDO_UID`から元ユーザーを解決し、rootではなく元ユーザー側のhomeに保存する。記録時は各エントリに`sudo = true/false`を保存する。
- **sudo方針**: `apply`と`capture enable/disable`はOSパッケージ操作・`/usr/local/bin`更新を伴うためroot必須。`status`、`doctor`、manifest編集系はroot必須にしない。
- **root書き込み回避**: rootでユーザーhome配下へmanifestを書かない。manifest保存前に元ユーザーへ権限を落としてから書き込む。
- **symlink健全性懸念**: 「OSリリースアップグレード時shim初期化/削除されないか」懸念に対し、`/usr/local/bin`はdpkg管理外で通常影響受けないが、確実性高めるため`saya doctor`コマンド新設、shim整合性いつでも検証・再修復促せるようにする。
- **初期実装スコープ**: APT backend完全実装し動作確認。pacman backendはBackend trait実装の骨格のみ(開発環境にpacman無いため実機確認不可、コンパイル通ることのみ保証)。
- **マニフェスト同時書き込み対策**: atomic write(tmpファイル→rename)のみ実装。file lockingは初期版不要(shim同時実行は稀ケースとして許容)。

## アーキテクチャ概要

単一バイナリ、`argv[0]`ディスパッチで3つの顔持つ。

```
argv[0] == "saya"            -> 通常CLI
argv[0] == "apt"|"apt-get"   -> APT shim
argv[0] == "pacman"          -> pacman shim
```

`saya capture enable`が`/usr/local/bin/{apt,apt-get,pacman}`にsaya実体へのsymlink設置。既存`sudo apt install foo`の使用感変えず、直接指定パッケージ名だけ自動記録できる。

CLAUDE.md方針(過剰抽象化禁止・シンプル・拡張性)に従い、trait抽象化は「pacman backend後で実装可能にする」具体的必要性ある`Backend`にのみ適用。他は素朴な構造体・関数で十分。

## クレート選定

- CLI解析: `clap` 4.x (derive) — 事実上の標準
- シリアライズ: `serde` (derive) + `toml` — 定番の組み合わせ
- エラー処理: `anyhow` — 独自エラー型(thiserror)作る必要性薄い
- UID→passwd解決: `libc` — `getpwuid_r`をFFI直呼び。nixは今回使う関数1つに対し依存重すぎる

shim内部からの実コマンド呼び出し(`/usr/bin/apt`等)は絶対パス固定、shell介さず`std::process::Command`実行。無限ループ(shim→shim)防ぐため。

**execについての設計判断**: 元仕様「execで」だが、終了コード判定後記録処理必要なため`Command::status()`(fork+wait)使う。真の`exec`(プロセス置換)では制御戻らず後処理できないため、実装時コメントで明記する。

## ファイル構成

```
saya/
├── Cargo.toml
└── src/
    ├── main.rs            # argv[0]判定 → ディスパッチ
    ├── cli.rs             # clap derive構造体(saya本来のサブコマンド)
    ├── manifest.rs        # packages.toml の構造体・load/save(atomic write)
    ├── privilege.rs       # 実行ユーザー解決・root要求・権限降格
    ├── backend/
    │   ├── mod.rs         # Backend trait、distro判定によるbackend選択
    │   ├── apt.rs         # APT backend(完全実装)
    │   └── pacman.rs      # pacman backend(trait実装の骨格のみ)
    ├── shim.rs            # argv[0]==apt/apt-get/pacman時の引数解析・記録・委譲実行
    ├── capture.rs         # capture enable/disable(symlink設置/削除)、doctor診断
    └── commands/
        ├── mod.rs
        ├── apply.rs       # saya apply
        ├── status.rs      # saya status
        ├── add.rs         # saya add / forget(対称操作のため1ファイル)
        └── import.rs      # saya import --manual [--edit]
```

`commands/`が「CLIサブコマンドのハンドラ」、それ以外が「ロジック・ドメイン層」の二層構造。

## モジュール詳細

### manifest.rs
```rust
#[derive(Serialize, Deserialize, Default)]
pub struct Manifest {
    pub version: u32,
    pub packages: BTreeMap<String, PackageEntry>, // BTreeMapで書き込み順を安定化
}

#[derive(Serialize, Deserialize, Default)]
pub struct PackageEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sudo: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apt: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pacman: Vec<String>,
}
```
- `git = {}`(apt/pacman空)は論理名そのまま実パッケージ名として使う。この解釈ロジックを`PackageEntry::resolve_names(&self, logical: &str, kind: BackendKind) -> Vec<String>`として実装。
- `sudo`はそのエントリを記録した呼び出しがsudo経由だったかを表す。旧manifest互換のため読み込み時は省略可能。
- `Manifest::load(path)` — ファイル無ければ`Default`返す(初回実行対応)。
- `Manifest::save(path)` — atomic write(`.tmp`書き込み→`fs::rename`)。rootでユーザーhomeへ保存する呼び出し側は、保存前に`privilege::drop_to_user()`で元ユーザーへ権限を落とす。
- `Manifest::find_logical_name_by_real(real_name, kind)` — shim記録時、既存論理名の再利用判定に使う。

### privilege.rs
```rust
pub struct InvocationUser { pub uid: u32, pub gid: u32, pub home: PathBuf, pub used_sudo: bool }
pub fn require_root() -> Result<()>;
pub fn resolve_invocation_user() -> Result<InvocationUser>;
pub fn drop_to_user(user: &InvocationUser) -> Result<()>;
```
`SUDO_UID`優先理由: 数値なので名前変更・特殊文字の曖昧さ無い。非sudoでは現在のuidを使う。`drop_to_user`はmanifest保存前にrootから元ユーザーへ不可逆に権限を落とすために使う。

### backend/mod.rs
```rust
pub enum BackendKind { Apt, Pacman }
pub trait Backend {
    fn kind(&self) -> BackendKind;
    fn is_installed(&self, real_pkg_name: &str) -> Result<bool>;
    fn install(&self, real_pkg_names: &[String]) -> Result<()>;
    fn list_manually_installed(&self) -> Result<Vec<String>>;
}
pub fn detect_backend() -> Result<Box<dyn Backend>>; // /etc/os-release の ID/ID_LIKE
```
uninstall/removeはsaya側から呼ばないため意図的に作らない(YAGNI)。

### backend/apt.rs(完全実装)
- `is_installed`: `dpkg-query -W -f='${Status}' <pkg>`
- `install`: `/usr/bin/apt-get install -y <pkgs>`(絶対パス)
- `list_manually_installed`: `apt-mark showmanual`の標準出力を1行1パッケージ名としてパース

### backend/pacman.rs(骨格)
trait実装のみ。`pacman -Q`/`pacman -S --noconfirm`/`pacman -Qqe`相当をmanページベースで実装し、`// NOTE: untested on real Arch system`を明記。

### shim.rs
1. `argv[0]`のbasenameで種別判定(`apt`/`apt-get`/`pacman`)
2. 実コマンド絶対パス(`/usr/bin/apt`等)を`Command::status()`で実行
3. 終了コード0なら`parse_install_targets(kind, &args) -> Vec<String>`で直接指定パッケージ名のみ抽出
4. 既存マニフェストに論理名あれば再利用、無ければ実パッケージ名そのまま新規論理名として追加し、`sudo`有無も記録
5. 保存が必要なら元ユーザーへ権限を落としてatomic write
6. 元の終了コードで`process::exit`

**`parse_install_targets`仕様**:
- APT: サブコマンドが`install`であること。`update`/`upgrade`/`remove`/`autoremove`/`purge`は無視。`-`始まりオプション除外。`.deb`終端・`://`含む・`=`含む(バージョン固定)・`/`含む(リポジトリ指定)は記録対象から除外。
- pacman: `pacman -S foo`/`pacman -Syu foo`(引数あり)のみ記録。`-Syu`単体・`-R`・`-Sc`は無視。

clapを使わず素朴な文字列フィルタで実装する(aptの引数解析にclapへ寄せるのは過剰設計)。

### capture.rs
```rust
const SHIM_NAMES: &[&str] = &["apt", "apt-get", "pacman"];
const SHIM_DIR: &str = "/usr/local/bin";
pub fn enable() -> Result<()>;  // 冪等。既に正しいsymlinkなら何もしない
pub fn disable() -> Result<()>; // saya実体へのsymlinkである場合のみ削除(他人のファイルは消さない)
pub fn doctor() -> Result<DoctorReport>;
```
`doctor`は以下を検証する(新設コマンド、リリースアップグレード対策):
1. `/usr/local/bin/{name}`がsaya実体への正しいsymlinkか
2. `/usr/bin/{name}`(実体)が存在するか
3. `PATH`環境変数を自前で分割し、`/usr/local/bin`が`/usr/bin`より先に解決されるか(外部`which`コマンドには依存しない)

問題があれば`sudo saya capture enable`の再実行を促すメッセージを出す。

### commands/
- `apply.rs`: 全エントリで`is_installed`確認、未インストールの実パッケージ名集めて`install`一括実行。
- `status.rs`: 同様にdiff計算し表示のみ(installは呼ばない)。
- `add.rs` / `forget`: manifestへのキー追加・削除、save。対称操作なので1ファイルにまとめる。保存前に必要なら元ユーザーへ権限を落とす。
- `import.rs`: `list_manually_installed`から未収録分を一覧。`--edit`なら元ユーザーへ権限を落としてからtmpfileに書き出し`$EDITOR`(無ければ`vi`)起動、編集後マージ。`--edit`無しは候補を標準出力するだけで保存しない(誤操作防止の2段階確認)。

### cli.rs / main.rs
clap derive。`main.rs`で`argv[0]`のbasenameを見て`shim::run`か`run_saya_cli`に振り分け。shim経路は`Cli::parse()`を通さない(aptの引数をclapに渡さない)。通常CLIは先に`resolve_invocation_user()`でmanifest所有ユーザーを決め、rootが必要なサブコマンドだけ`require_root()`を呼ぶ。

## Cargo.toml
```toml
[package]
name = "saya"
version = 略
edition = "2024"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
libc = "0.2"
serde = { version = "1", features = ["derive"] }
toml = "1"
```

## 実装順序

1. `cargo init` + 依存追加 → `cargo build`通ることを確認
2. `manifest.rs`(外部依存なし、unit test: resolve_names, load/save)
3. `privilege.rs`(unit test: SUDO_UID読み取りロジック)
4. `backend/mod.rs` + `backend/apt.rs`(distro判定、出力パース部分をunit test)
5. `backend/pacman.rs`(骨格、コンパイル確認)
6. `cli.rs`(`cargo run -- --help`で構造確認)
7. `commands/apply.rs`, `status.rs`(読み取り系から)
8. `commands/add.rs`(add/forget、書き込み系)
9. `shim.rs`(`parse_install_targets`をunit testで手厚くカバー: install検出、オプション除外、除外パターン全種、update/upgrade/remove類の無視)
10. `capture.rs`(symlink操作・doctor。root権限要、実機確認はsudo環境で手動)
11. `commands/import.rs`(他モジュール依存のため最後)
12. `main.rs`のargv[0]ディスパッチを最終結線、全体統合

外部コマンド・root権限・symlinkに依存しない部分を先に固めて自動テストでカバーし、I/Oが絡む部分を後ろに回す方針。

## 検証方法

**自動(cargo test、root不要)**:
- `manifest.rs`: load/save(tempdir使用)、`resolve_names`、`find_logical_name_by_real`
- `shim.rs`: `parse_install_targets`への多数の入力パターン(最も分岐が多いため重点的に)
- `backend/apt.rs`: `apt-mark showmanual`出力パース部分を関数として切り出してテスト
- `privilege.rs`: 現在の自分のuidを使った`getpwuid_r`呼び出し確認

**手動確認が必要(副作用・root権限・実機依存のため)**:
- `backend/apt.rs`の実際の`apt-get install`/`dpkg-query`実行
- `capture.rs`のsymlink設置(`sudo saya capture enable`を実行し`/usr/local/bin`配下を確認、`saya doctor`で検証)
- shim経由のEnd-to-Endフロー(`sudo apt install <test-package>`を実行し、終了コード0で`~/.config/saya/packages.toml`に記録されるか確認)
- pacman backend全体(この環境に無いため確認不可、コンパイルのみ保証)

## YAGNI: 今回作らないもの
file locking、`pacman -U`対応、バージョン固定パッケージの記録、独自エラー型(thiserror)、設定ファイルの複数プロファイル対応。いずれも要件に明記されていないため実装しない。

## Critical Files
- `/home/stasshe_c/dev/github/saya/Cargo.toml`
- `/home/stasshe_c/dev/github/saya/src/main.rs`
- `/home/stasshe_c/dev/github/saya/src/manifest.rs`
- `/home/stasshe_c/dev/github/saya/src/shim.rs`
- `/home/stasshe_c/dev/github/saya/src/backend/apt.rs`
- `/home/stasshe_c/dev/github/saya/src/capture.rs`
- `/home/stasshe_c/dev/github/saya/src/privilege.rs`
