# saya

APT/yayの薄いラッパー。chezmoi(dotfiles)、mise(devtools)に対し、OSパッケージの「意図記録」と「再現」だけ担う。

- **意図記録**: `saya install foo bar`でインストールに成功したパッケージをマニフェストへ記録する。
- **状態適用**: `saya install`(引数なし)で`present`の不足分を入れ、`absent`の導入済みパッケージを消す。
- **明示的な削除**: `saya uninstall foo bar`でアンインストールし、削除意図を`absent`へ記録する。

マニフェストは実行ユーザーの`~/.config/saya/packages.toml`に通常の設定ファイルとして`0644`で保存する。APT環境で`sudo`経由ならrootではなく元ユーザー側に保存する。保存内容が同一なら内容を書き換えず、差分は同じディレクトリの一意な一時ファイルを経由して置き換える。

詳細設計は[SPECIFICATION.md](./SPECIFICATION.md)参照。

## インストール

```sh
curl -fsSL https://raw.githubusercontent.com/Stasshe/saya/main/install.sh | sh
```

x86_64 / aarch64 Linux対応。`SAYA_VERSION=v0.2.0`のように特定バージョン指定、`SAYA_INSTALL_DIR`で配置先変更可能。

ソースからビルドする場合:

```sh
cargo install --git https://github.com/Stasshe/saya
```

## 使い方

```sh
saya -v                    # saya本体のバージョンを表示する(--versionも可)
saya self-update           # 最新のGitHub Releaseからsaya本体を更新する
saya update                # apt-get update / yay -Sy を実行する
saya upgrade               # apt-get upgrade / yay -Syu を実行する
saya install neovim        # 検出したbackendでインストールし、成功したら記録する
saya install -y openssh-server
                           # apt/yayと同じ位置の-yも受理する
saya install adb fastboot  # 複数パッケージをまとめてインストールし、成功したら記録する
saya install neovim -- --config /path/to/pacman.conf
                           # -- 以降をapt-get / yayのinstallへそのまま渡す
saya install               # presentの不足分を入れ、absentの導入済みパッケージを消す

saya status                # マニフェストとインストール状態の差分確認
saya uninstall neovim git  # アンインストールし、absentへ記録する
```

## マニフェスト

`~/.config/saya/packages.toml`に、apt/yayそれぞれのパッケージ名をそのまま並べる。

```toml
schema_version = 5

[apt]
present = ["neovim", "build-essential"]
absent = ["nano"]

[yay]
present = ["neovim", "base-devel"]
absent = ["nano"]
```

`present`は導入するパッケージ、`absent`は明示的に削除状態を維持するパッケージ。同じ名前を両方には書けない。どちらからも名前を消すと、そのパッケージは管理対象外になる。

schema 4から更新する場合は`schema_version = 5`へ変更し、従来の`apt`/`yay`配列を各backendの`present`へ移す。自動移行はしない。

- apt/yay間でのパッケージ名の対応付けはしない。`saya install <name>`/`saya uninstall <name>`は今動いているOSのbackendだけを更新する。他方のOSにも適用したい場合は、そちらの環境で改めて実行する。
- Arch系では公式リポジトリとAURの両方をyayで扱う。`/usr/bin/yay`が必要で、sayaはsudoを付けずに実行する。
- 同じアプリでもOSごとにパッケージ名が異なることが多い(例: apt=`build-essential` / yay=`base-devel`)。この場合は各OSでそのOSのパッケージ名を`install`すればよい。
- `uninstall`は複数パッケージをまとめて処理し、成功後に`present`から`absent`へ移す。APTでは`apt-get remove --purge`後に`apt-get autoremove --purge`、yayでは`yay -Rns`を使う。
- installは常に非対話で実行する。`-y`はapt/yayに慣れた操作との互換用で、省略時も挙動は同じ。
- Arch系では`saya`を一般ユーザーとして実行する。`sudo saya ...`はAURビルドをrootで実行しないため拒否する。
- `saya install`(引数なし)/`saya status`は今動いているOS側の`present`/`absent`だけを見る。

## リリース手順(開発者向け)

GitHub Actionsの`release`をworkflow_dispatchで実行(patch/minor/major選択)。
`Cargo.toml`バージョン自動算出→更新・コミット・タグpush→x86_64/aarch64のmuslバイナリビルド→GitHub Releasesへ添付、を1本のワークフローで行う。
