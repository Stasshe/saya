# saya: OS標準パッケージマネージャの薄いラッパー

## Context

chezmoi(dotfiles管理)、mise(開発ツール管理)に対し、OSパッケージ管理(APT/pacman)の「意図記録」と「再現」だけを担う小さいツール。

sayaは大規模統合パッケージマネージャではなく、OS標準パッケージマネージャに以下を足す。

- **意図記録**: `saya install foo bar`でインストールに成功したパッケージをマニフェストへ記録する。
- **一方向適用**: `saya install`(引数なし)でマニフェストにあって未インストールのものだけインストールする。
- **明示的な削除**: `saya uninstall foo`でアンインストールし、マニフェストからも削除する。

## 設計判断

- **明示的な install/uninstall**: apt/pacman の shim や自動キャプチャは作らない。パッケージ追加・削除は `saya install <package>` / `saya uninstall <package>` で明示する。
- **install/addの統合**: 当初`saya add <package>`(install+記録)と`saya install`(一括反映)を別コマンドにしていたが、名前が近く役割も「install系」で揃うため統合した。npm(`npm install`=lockfile一括、`npm install <pkg...>`=追加)と同じ、引数の有無で挙動を切り替える形にした。`saya uninstall`は対称に見えるが「一括アンインストール」という概念が元々ないため、名前は常に必須。
- **backend引数の境界**: `saya install <package...> -- <arg...>`の`--`以降は、解釈・保存せず検出中backendのinstallへ渡す。値を取るオプションとパッケージ名を推測で区別せず、任意のapt-get/pacmanオプションを扱える明示境界とする。
- **非対話install**: installは常にAPTへ`-y`、pacmanへ`--noconfirm`を渡す。`saya install -y <package...>`も同じ非対話操作として受理する。
- **manifest保存先**: 設定ファイルは実行ユーザーの `~/.config/saya/packages.toml` に保存する。`sudo` 経由では `SUDO_UID` から元ユーザーを解決し、rootではなく元ユーザー側のhomeに保存する。
- **sudo方針**: apt/pacman のインストール操作は常に `/usr/bin/sudo` 経由で実行する。
- **root書き込み回避**: rootでユーザーhome配下へmanifestを書かない。manifest保存前に元ユーザーへ権限を落としてから書き込む。
- **初期実装スコープ**: APT backendは実装済み。pacman backendはBackend trait実装の骨格のみで、開発環境にpacmanが無いため実機確認不可。
- **マニフェスト書き込み**: 保存内容が既存ファイルと同一なら何もしない。差分があれば同じディレクトリに一意なtmpファイルを排他的に作成し、書き込みを同期してからrenameする。file lockingは作らない。
- **backend間の論理名共有を撤廃**: 当初はlogical name 1つにapt/pacman双方の実パッケージ名を紐付ける設計だったが、実運用でUbuntu/Arch間に共有できる要素が少ないと判明した。backendごとに独立したフラットな配列に変更し、`saya install <name>`は検出中backendの配列へその名前をそのまま追加するだけにした。同じ概念のパッケージを両OSで使う場合の対応関係は構造で強制せず、両OSで順に`install`すればファイル末尾追加の結果として自然に隣接行になる程度に留める。schema_versionを3へ上げ、旧形式との後方互換・移行は作らない(読み込み時にエラーで顕在化させる)。

## アーキテクチャ概要

単一バイナリの通常CLIのみ。

```text
saya -v / --version        -> print the saya binary version
saya self-update           -> update this binary from GitHub Releases
saya update                -> update package manager metadata
saya upgrade               -> upgrade installed packages through detected backend
saya install                -> install missing manifest packages
saya install <package...>   -> install through detected backend, then record
saya install -y <package...> -> accept the familiar non-interactive form
saya install <package...> -- <arg...> -> pass native install arguments through
saya status                 -> show install status
saya uninstall <package>    -> uninstall through detected backend, then remove from manifest
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
        ├── uninstall.rs
        ├── status.rs
        └── import.rs
```

## モジュール詳細

### manifest.rs

```rust
pub struct Manifest {
    pub schema_version: u32,
    pub apt: Vec<String>,
    pub pacman: Vec<String>,
}
```

- `schema_version` はマニフェストファイル形式のバージョンで、saya本体のリリースバージョンではない。
- `apt`/`pacman` はそれぞれのbackendで使うパッケージ名をそのまま並べたフラットな配列。backend間の対応関係は持たない。
- `Manifest::save(path)` はシリアライズ結果が既存内容と同一なら書き換えない。差分があれば同じディレクトリに一意なtmpファイルを排他的に作成し、書き込みを同期してから対象へrenameする。

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
    fn install(&self, real_pkg_names: &[String], backend_args: &[String]) -> Result<()>;
    fn uninstall(&self, real_pkg_names: &[String]) -> Result<()>;
    fn list_manually_installed(&self) -> Result<Vec<String>>;
}
```

- `detect_backend()` は `/etc/os-release` の `ID`/`ID_LIKE` から APT/pacman を選ぶ。
- `update()` / `upgrade()` / `install()` / `uninstall()` は `/usr/bin/sudo` 経由で package manager を呼ぶ。

### commands/install.rs

`saya install`(パッケージ引数なし)はmanifestの各エントリについて `is_installed` を確認し、未インストールの実パッケージ名だけをまとめて `install()` に渡す。`saya install <name...>`は検出中backendで全指定パッケージを一度にインストールし、コマンド全体の成功後に未記録名を配列へ追記する。全て記録済みならインストールだけ行いmanifestは保存しない。installは常に非対話で、`-y`指定も受理する。`--`以降の引数は両方の形式でbackendのinstall引数へ順序と値を変えずに渡し、manifestには記録しない。

### commands/uninstall.rs

`saya uninstall <name>` はmanifestへの記録や事前のインストール判定にかかわらず、検出中backendでアンインストールを実行してからmanifestの該当配列から`name`を削除する。APT backendは対象を`apt-get remove --purge`で削除後、`apt-get autoremove --purge`で不要な依存パッケージも削除する。pacman backendは`pacman -Rns`を使う。

### commands/status.rs

manifest と現在のインストール状態を表示する。インストールはしない。

### commands/import.rs

`list_manually_installed` から未収録分を一覧する。`--edit` の場合だけエディタで候補を編集してmanifestへ取り込む。

## 検証方法

**自動(cargo test、root不要)**:

- manifest の load/save、名前解決、既存記録判定
- distro backend 判定
- apt manual list パース
- privilege の passwd lookup
- install command(複数指定/manifest一括)の成功時記録・未導入パッケージ抽出・`-y`・backend引数境界
- uninstall command の常時backend実行・manifest削除
- manifest保存内容が同一の場合の無変更

**手動確認が必要**:

- APT実機での `saya install <package...>` / `saya install` / install引数透過 / `saya uninstall <package>`
- pacman backend全体

## YAGNI

file locking、pacmanオプションの個別解釈、バージョン固定パッケージの記録、独自エラー型、設定ファイルの複数プロファイル対応。
