# saya

OS標準パッケージマネージャ(apt/pacman)の薄いラッパー。chezmoi(dotfiles)、mise(devtools)に対し、OSパッケージの「意図記録」と「再現」だけ担う。

- **意図記録**: `saya install foo bar`でインストールに成功したパッケージをマニフェストへ記録する。
- **一方向適用**: `saya install`(引数なし)でマニフェストにあって未インストールのものだけインストールする。
- **明示的な削除**: `saya uninstall foo`でアンインストールし、マニフェストからも削除する。

マニフェストは実行ユーザーの`~/.config/saya/packages.toml`に保存する。`sudo`経由で実行した場合もrootではなく元ユーザー側に保存する。保存内容が同一ならファイルを書き換えない。

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
saya self-update           # 最新のGitHub Releaseからsaya本体を更新する
saya update                # apt-get update / pacman -Sy を実行する
saya upgrade               # apt-get upgrade / pacman -Syu を実行する
saya install neovim        # 検出したbackendでインストールし、成功したら記録する
saya install adb fastboot  # 複数パッケージをまとめてインストールし、成功したら記録する
saya install neovim -- -C /path/to/pacman.conf
                           # -- 以降をapt-get / pacmanのinstallへそのまま渡す
saya install               # マニフェストにあって未インストールのものを入れる(引数なし)

saya status                # マニフェストとインストール状態の差分確認
saya uninstall neovim      # アンインストールし、マニフェストから削除する
```

## マニフェスト

`~/.config/saya/packages.toml`に、apt/pacmanそれぞれのパッケージ名をそのまま並べる。

```toml
schema_version = 3
apt = [
    "neovim",
    "build-essential",
]
pacman = [
    "neovim",
    "base-devel",
]
```

- apt/pacman間でのパッケージ名の対応付けはしない。`saya install <name>`は今動いているOSのbackend(apt or pacman)を判定し、その配列にだけ名前を追記する。他方のOSにも入れたい場合は、そちらの環境で改めて`saya install <name>`を実行する。
- 同じアプリでもOSごとにパッケージ名が異なることが多い(例: apt=`build-essential` / pacman=`base-devel`)。この場合は各OSでそのOSのパッケージ名を`install`すればよい。
- `saya install`(引数なし)/`saya status`は今動いているOS側の配列だけを見る。

## リリース手順(開発者向け)

GitHub Actionsの`release`をworkflow_dispatchで実行(patch/minor/major選択)。
`Cargo.toml`バージョン自動算出→更新・コミット・タグpush→x86_64/aarch64のmuslバイナリビルド→GitHub Releasesへ添付、を1本のワークフローで行う。
