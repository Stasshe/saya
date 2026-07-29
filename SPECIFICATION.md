# saya: APT/yayの薄いラッパー

## Context

chezmoi(dotfiles管理)、mise(開発ツール管理)に対し、OSパッケージ管理(APT/yay)の「意図記録」と「再現」だけを担う小さいツール。

sayaは大規模統合パッケージマネージャではなく、APT/yayに以下を足す。

- **意図記録**: `saya install foo bar`でインストールに成功したパッケージをマニフェストへ記録する。
- **状態適用**: `saya install`(引数なし)で`present`の不足分を入れ、`absent`の導入済みパッケージを消す。
- **明示的な削除**: `saya uninstall foo bar`でアンインストールし、削除意図を`absent`へ記録する。

## 設計判断

- **明示的な install/uninstall**: apt/yay の shim や自動キャプチャは作らない。パッケージ追加・削除は `saya install <package>` / `saya uninstall <package>` で明示する。
- **削除意図の永続化**: manifest未記載は管理対象外とし、OS標準パッケージを削除対象にしない。明示的にuninstallした名前だけを`absent`へ残し、別環境でも削除状態を再現する。不要になった意図はmanifestから手動で消す。
- **明示的なschema移行**: 起動時には移行しない。`saya migrate`だけが直前のschemaを現在のschemaへ一段階移行する。複数世代の互換処理は保持しない。
- **install/addの統合**: 当初`saya add <package>`(install+記録)と`saya install`(一括反映)を別コマンドにしていたが、名前が近く役割も「install系」で揃うため統合した。npm(`npm install`=lockfile一括、`npm install <pkg...>`=追加)と同じ、引数の有無で挙動を切り替える形にした。`saya uninstall`は対称に見えるが「一括アンインストール」という概念が元々ないため、名前は常に必須。
- **backend引数の境界**: `saya install <package...> -- <arg...>`の`--`以降は、解釈・保存せず検出中backendのinstallへ渡す。値を取るオプションとパッケージ名を推測で区別せず、任意のapt-get/yayオプションを扱える明示境界とする。
- **非対話install**: installは常にAPTへ`-y`、yayへ`--noconfirm`を渡す。`saya install -y <package...>`も同じ非対話操作として受理する。
- **manifest保存先**: 設定ファイルは実行ユーザーの `~/.config/saya/packages.toml` に保存する。APT環境で`sudo`経由なら`SUDO_UID`から元ユーザーを解決し、rootではなく元ユーザー側のhomeに保存する。
- **権限方針**: APTは`/usr/bin/sudo`経由、yayは一般ユーザーとして直接実行する。Arch系でroot実行されたsayaは処理を拒否する。
- **root書き込み回避**: rootでユーザーhome配下へmanifestを書かない。manifest保存前に元ユーザーへ権限を落としてから書き込む。
- **Arch backend**: yayで公式リポジトリとAURを一括管理する。pacmanとのbackend分割はしない。`/usr/bin/yay`未導入時はエラーにする。
- **マニフェスト書き込み**: `0644`で保存する。保存内容が既存ファイルと同一なら内容を書き換えず、権限のみ補正する。差分があれば同じディレクトリに一意なtmpファイルを排他的に作成し、`0644`へ変更して書き込みを同期してからrenameする。file lockingは作らない。
- **backend間の論理名共有を撤廃**: Ubuntu/Arch間で共有できるpackage名が少ないため、apt/yayごとに独立した`present`/`absent`を持つ。package名は検出中backendへそのまま記録する。schema_versionは5とし、通常の読み込みに旧形式との後方互換は持たせない。

## アーキテクチャ概要

単一バイナリの通常CLIのみ。

```text
saya -v / --version        -> print the saya binary version
saya self-update           -> update this binary from GitHub Releases
saya update                -> update package manager metadata
saya upgrade               -> upgrade installed packages through detected backend
saya install                -> apply present/absent entries
saya install <package...>   -> install through detected backend, then record
saya install -y <package...> -> accept the familiar non-interactive form
saya install <package...> -- <arg...> -> pass native install arguments through
saya status                 -> show install status
saya migrate                -> migrate the previous manifest schema
saya uninstall <package...> -> uninstall through detected backend, then record absent
saya import --manual        -> list or import manually-installed packages
```

Backend traitでAPT/yay差分を吸収する。trait抽象化はOS backendを差し替える箇所に限定する。

外部コマンド呼び出しは絶対パス固定で、shellを介さず `std::process::Command` を使う。

## ファイル構成

```text
saya/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── cli.rs
    ├── cli/
    │   └── tests.rs
    ├── manifest.rs
    ├── privilege.rs
    ├── backend/
    │   ├── mod.rs
    │   ├── apt.rs
    │   └── yay.rs
    └── commands/
        ├── mod.rs
        ├── install.rs
        ├── migrate.rs
        ├── uninstall.rs
        ├── status.rs
        └── import.rs
```

## モジュール詳細

### manifest.rs

```rust
pub struct Manifest {
    pub schema_version: u32,
    pub apt: PackageSet,
    pub yay: PackageSet,
}
pub struct PackageSet {
    pub present: Vec<String>,
    pub absent: Vec<String>,
}
```

- `schema_version` はマニフェストファイル形式のバージョンで、saya本体のリリースバージョンではない。
- `apt`/`yay` はそれぞれ独立したbackend状態。`present`は導入意図、`absent`は削除意図。同じ名前を両方へ入れられず、どちらにもなければ管理対象外。
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
pub enum BackendKind { Apt, Yay }
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

- `detect_backend()` は `/etc/os-release` の `ID`/`ID_LIKE` からDebian系ならAPT、Arch系ならyayを選ぶ。
- APTの変更操作は`/usr/bin/sudo`経由で呼ぶ。yayは一般ユーザーとして直接呼ぶ。
- Arch系では`/usr/bin/yay`の存在を要求し、root実行を拒否する。

### commands/install.rs

`saya install`(パッケージ引数なし)は検出中backendの`present`で未導入の名前をまとめて `install()` へ渡し、`absent`で導入済みの名前をまとめて`uninstall()`へ渡す。`saya install <name...>`は全指定パッケージを一度にインストールし、成功後に`present`へ記録して同名を`absent`から除く。状態が変わらなければmanifestは保存しない。installは常に非対話で、`-y`指定も受理する。`--`以降の引数はinstall時だけbackendへ順序と値を変えずに渡し、manifestには記録しない。

### commands/uninstall.rs

`saya uninstall <name...>` はmanifestへの記録や事前のインストール判定にかかわらず、全指定パッケージを検出中backendで一度にアンインストールする。成功後に`present`から除いて`absent`へ記録する。APT backendは対象を`apt-get remove --purge`で削除後、`apt-get autoremove --purge`で不要な依存パッケージも削除する。yay backendは`yay -Rns`を使う。

### commands/migrate.rs

`saya migrate`は現在schemaの直前だけを読み込み、現在形式へ変換して同じmanifestへ保存する。schema 4から5では従来のapt/yay配列を各`present`へ移し、`absent`を空にする。対象外schemaと不正な旧形式は書き換えずエラーにする。

### commands/status.rs

検出中backendの`present`/`absent`と現在のインストール状態を表示する。変更はしない。

### commands/import.rs

`list_manually_installed` から`present`/`absent`のどちらにもない未管理分を一覧する。`--edit` の場合だけエディタで候補を編集して`present`へ取り込む。

## 検証方法

**自動(cargo test、root不要)**:

- manifest の load/save、present/absent排他、既存記録判定
- 直前schemaからのmanifest移行・対象外schemaの拒否
- distro backend 判定
- apt manual list パース
- privilege の passwd lookup
- install command(複数指定/manifest適用)の成功時記録・導入/削除対象抽出・`-y`・backend引数境界
- uninstall command の複数指定・常時backend実行・absent記録
- manifest保存内容が同一の場合の無変更

**手動確認が必要**:

- APT実機での `saya install <package...>` / present・absent適用 / install引数透過 / `saya uninstall <package...>`
- yayのinstall/update/upgrade/uninstallによる実機変更

## YAGNI

file locking、yayオプションの個別解釈、バージョン固定パッケージの記録、独自エラー型、設定ファイルの複数プロファイル対応。
