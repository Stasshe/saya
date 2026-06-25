# saya

OS標準パッケージマネージャ(apt/pacman)の薄いラッパー。chezmoi(dotfiles)、mise(devtools)に対し、OSパッケージの「意図記録」と「再現」だけ担う。

- **意図記録**: `saya install foo`でインストールに成功した直接指定パッケージ名だけマニフェストへ記録する。
- **一方向適用**: マニフェストにあって未インストールのものだけインストール。マニフェストから消えてもアンインストールしない。

マニフェストは実行ユーザーの`~/.config/saya/packages.toml`に保存する。`sudo`経由で実行した場合もrootではなく元ユーザー側に保存する。

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
saya install neovim        # apt/pacmanをsudo経由で呼び出し、成功したら記録する

saya status                # マニフェストとインストール状態の差分確認
saya apply                 # マニフェストにあって未インストールのものを入れる
```

## リリース手順(開発者向け)

GitHub Actionsの`release`をworkflow_dispatchで実行(patch/minor/major選択)。
`Cargo.toml`バージョン自動算出→更新・コミット・タグpush→x86_64/aarch64のmuslバイナリビルド→GitHub Releasesへ添付、を1本のワークフローで行う。
