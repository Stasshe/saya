# saya: OS標準パッケージマネージャの薄いラッパー

## Context

chezmoi(dotfiles管理)、mise(開発ツール管理)に対し、OSパッケージ管理(APT/pacman)の「意図記録」と「再現」だけを担う小さいツール。

sayaは大規模統合パッケージマネージャではなく、OS標準パッケージマネージャに以下2つだけを足す。

- **意図記録**: `saya add foo`でインストールに成功したパッケージをマニフェストへ記録する。
- **一方向適用**: `saya install`でマニフェストにあって未インストールのものだけインストールする。マニフェストから消えてもアンインストールしない。

## 設計判断

- **明示的な add**: apt/pacman の shim や自動キャプチャは作らない。パッケージ追加は `saya add <package>` で明示する。
- **manifest保存先**: 設定ファイルは実行ユーザーの `~/.config/saya/packages.toml` に保存する。`sudo` 経由では `SUDO_UID` から元ユーザーを解決し、rootではなく元ユーザー側のhomeに保存する。
- **sudo方針**: apt/pacman のインストール操作は常に `/usr/bin/sudo` 経由で実行する。
- **root書き込み回避**: rootでユーザーhome配下へmanifestを書かない。manifest保存前に元ユーザーへ権限を落としてから書き込む。
- **初期実装スコープ**: APT backendは実装済み。pacman backendはBackend trait実装の骨格のみで、開発環境にpacmanが無いため実機確認不可。
- **マニフェスト書き込み**: 保存内容が既存ファイルと同一なら何もしない。差分があればatomic write(tmpファイル→rename)する。file lockingは初期版では作らない。

## アーキテクチャ概要

単一バイナリの通常CLIのみ。

```text
saya self-update           -> update this binary from GitHub Releases
saya update                -> update package manager metadata
saya upgrade               -> upgrade installed packages through detected backend
saya install               -> install missing manifest packages
saya status                 -> show install status
saya add <package>          -> install through detected backend, then record
saya forget <package>       -> remove from manifest without uninstalling
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
    pub apt: Vec<String>,
    pub pacman: Vec<String>,
}
```

- `schema_version` はマニフェストファイル形式のバージョンで、saya本体のリリースバージョンではない。
- `git = {}` のように backend 固有名リストが空なら、論理名そのものを実パッケージ名として使う。
- `Manifest::save(path)` はシリアライズ結果が既存内容と同一なら書き換えず、差分があれば同じディレクトリの `.tmp` へ書いてから rename する。

### privilege.rs

```rust
pub struct InvocationUser { pub uid: u32, pub gid: u32, pub home: PathBuf }
pub fn resolve_invocation_user() -> Result<InvocationUser>;
pub fn drop_to_user(user: &InvocationUser) -> Result<()>;
```

`SUDO_UID` を優先して元ユーザーを決める。`drop_to_user` はmanifest保存前にrootから元ユーザーへ不可逆に権限を落とす。

### backend

```rust
pub enum BackendKind { Apt, Pacman }
pub trait Backend {
    fn kind(&self) -> BackendKind;
    fn update(&self) -> Result<()>;
    fn upgrade(&self) -> Result<()>;
    fn is_installed(&self, real_pkg_name: &str) -> Result<bool>;
    fn install(&self, real_pkg_names: &[String]) -> Result<()>;
    fn list_manually_installed(&self) -> Result<Vec<String>>;
}
```

- `detect_backend()` は `/etc/os-release` の `ID`/`ID_LIKE` から APT/pacman を選ぶ。
- `update()` / `upgrade()` / `install()` は `/usr/bin/sudo` 経由で package manager を呼ぶ。
- uninstall/remove は saya の責務にしない。

### commands/install.rs

manifest の各エントリについて `is_installed` を確認し、未インストールの実パッケージ名だけをまとめて `install()` に渡す。

### commands/status.rs

manifest と現在のインストール状態を表示する。インストールはしない。

### commands/add.rs

`saya add` は現在のbackend用実パッケージ名をインストールし、成功後に指定エントリでmanifestを更新する。同一エントリならインストールだけ行いmanifestは保存しない。`saya forget` はアンインストールせずmanifestから削除する。

### commands/import.rs

`list_manually_installed` から未収録分を一覧する。`--edit` の場合だけエディタで候補を編集してmanifestへ取り込む。

## 検証方法

**自動(cargo test、root不要)**:

- manifest の load/save、名前解決、既存記録判定
- distro backend 判定
- apt manual list パース
- privilege の passwd lookup
- add command の成功時記録
- install command の未導入パッケージ抽出
- manifest保存内容が同一の場合の無変更

**手動確認が必要**:

- APT実機での `saya add <package>`
- APT実機での `saya install`
- pacman backend全体

## YAGNI

file locking、pacmanの詳細オプション対応、バージョン固定パッケージの記録、独自エラー型、設定ファイルの複数プロファイル対応。
