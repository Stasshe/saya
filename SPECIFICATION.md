# saya: OS標準パッケージマネージャの薄いラッパー

## Context

chezmoi(dotfiles管理)、mise(開発ツール管理)に対し、OSパッケージ管理(APT/pacman)の「意図記録」と「再現」だけを担う小さいツール。

sayaは大規模統合パッケージマネージャではなく、OS標準パッケージマネージャに以下2つだけを足す。

- **意図記録**: `saya install foo`でインストールに成功した直接指定パッケージ名だけマニフェストへ記録する。
- **一方向適用**: マニフェストにあって未インストールのものだけインストールする。マニフェストから消えてもアンインストールしない。

## 設計判断

- **明示的な install**: apt/pacman の shim や自動キャプチャは作らない。ユーザーは `saya install <packages...>` を使う。
- **manifest保存先とsudo記録**: 設定ファイルは実行ユーザーの `~/.config/saya/packages.toml` に保存する。`sudo` 経由では `SUDO_UID` から元ユーザーを解決し、rootではなく元ユーザー側のhomeに保存する。
- **sudo方針**: `saya install` は非rootなら内部で `/usr/bin/sudo` 経由で apt/pacman を実行する。`sudo saya install` でも動く。`apply` はroot必須のまま。
- **root書き込み回避**: rootでユーザーhome配下へmanifestを書かない。manifest保存前に元ユーザーへ権限を落としてから書き込む。
- **初期実装スコープ**: APT backendは実装済み。pacman backendはBackend trait実装の骨格のみで、開発環境にpacmanが無いため実機確認不可。
- **マニフェスト同時書き込み対策**: atomic write(tmpファイル→rename)のみ実装。file lockingは初期版では作らない。

## アーキテクチャ概要

単一バイナリの通常CLIのみ。

```text
saya install <packages...>  -> install through detected backend, then record
saya apply                  -> install missing manifest packages
saya status                 -> show install status
saya add / forget           -> edit manifest without installing
saya import --manual        -> list or import manually-installed packages
```

Backend traitでAPT/pacman差分を吸収する。trait抽象化は「OS backendを差し替える」必要性がある箇所に限定する。

外部コマンド呼び出しは絶対パス固定で、shellを介さず `std::process::Command` を使う。

## ファイル構成

```text
saya/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── cli.rs
    ├── manifest.rs
    ├── privilege.rs
    ├── backend/
    │   ├── mod.rs
    │   ├── apt.rs
    │   └── pacman.rs
    └── commands/
        ├── mod.rs
        ├── install.rs
        ├── apply.rs
        ├── status.rs
        ├── add.rs
        └── import.rs
```

## モジュール詳細

### manifest.rs

```rust
pub struct Manifest {
    pub schema_version: u32,
    pub packages: BTreeMap<String, PackageEntry>,
}

pub struct PackageEntry {
    pub sudo: Option<bool>,
    pub apt: Vec<String>,
    pub pacman: Vec<String>,
}
```

- `schema_version` はマニフェストファイル形式のバージョンで、saya本体のリリースバージョンではない。
- `git = {}` のように backend 固有名リストが空なら、論理名そのものを実パッケージ名として使う。
- `sudo` はそのエントリを記録した install が root/sudo を伴ったかを表す。
- `Manifest::save(path)` は同じディレクトリの `.tmp` へ書いてから rename する。

### privilege.rs

```rust
pub struct InvocationUser { pub uid: u32, pub gid: u32, pub home: PathBuf, pub used_sudo: bool }
pub fn require_root() -> Result<()>;
pub fn is_effective_root() -> bool;
pub fn resolve_invocation_user() -> Result<InvocationUser>;
pub fn drop_to_user(user: &InvocationUser) -> Result<()>;
```

`SUDO_UID` を優先して元ユーザーを決める。`drop_to_user` はmanifest保存前にrootから元ユーザーへ不可逆に権限を落とす。

### backend

```rust
pub enum BackendKind { Apt, Pacman }
pub trait Backend {
    fn kind(&self) -> BackendKind;
    fn is_installed(&self, real_pkg_name: &str) -> Result<bool>;
    fn install(&self, real_pkg_names: &[String]) -> Result<()>;
    fn list_manually_installed(&self) -> Result<Vec<String>>;
}
```

- `detect_backend()` は `/etc/os-release` の `ID`/`ID_LIKE` から APT/pacman を選ぶ。
- `install()` はrootなら直接 package manager を呼び、非rootなら `/usr/bin/sudo` 経由で呼ぶ。
- uninstall/remove は saya の責務にしない。

### commands/install.rs

1. 検出した backend で `args.packages` をインストールする。
2. 成功した場合だけ、未記録のパッケージを `logical == real_name` でmanifestへ追加する。
3. 保存が必要なら元ユーザーへ権限を落としてから保存する。

### commands/apply.rs

manifest の各エントリについて `is_installed` を確認し、未インストールの実パッケージ名だけをまとめて `install()` に渡す。

### commands/status.rs

manifest と現在のインストール状態を表示する。インストールはしない。

### commands/add.rs

`saya add` / `saya forget` でmanifestだけを編集する。パッケージマネージャは呼ばない。

### commands/import.rs

`list_manually_installed` から未収録分を一覧する。`--edit` の場合だけエディタで候補を編集してmanifestへ取り込む。

## 検証方法

**自動(cargo test、root不要)**:

- manifest の load/save、名前解決、既存記録判定
- distro backend 判定
- apt manual list パース
- privilege の passwd lookup
- install command の成功時記録

**手動確認が必要**:

- APT実機での `saya install <package>` と `sudo saya install <package>`
- APT実機での `sudo saya apply`
- pacman backend全体

## YAGNI

file locking、pacmanの詳細オプション対応、バージョン固定パッケージの記録、独自エラー型、設定ファイルの複数プロファイル対応。
