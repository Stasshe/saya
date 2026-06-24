# saya

OS標準パッケージマネージャ(apt/pacman)の薄いラッパー。chezmoi(dotfiles)、mise(devtools)に対し、OSパッケージの「意図記録」と「再現」だけ担う。

- **意図記録**: `sudo apt install foo`の使用感変えず、直接指定パッケージ名だけ自動でマニフェストへ記録(依存解決分は記録しない)。
- **一方向適用**: マニフェストにあって未インストールのものだけインストール。マニフェストから消えてもアンインストールしない。

詳細設計は[SPECIFICATION.md](./SPECIFICATION.md)参照。

## インストール

```sh
curl -fsSL https://raw.githubusercontent.com/Stasshe/saya/main/install.sh | sh
```

x86_64 / aarch64 Linux対応。`SAYA_VERSION=v0.1.0`で特定バージョン指定、`SAYA_INSTALL_DIR`で配置先変更可能。

ソースからビルドする場合:

```sh
cargo install --git https://github.com/Stasshe/saya
```

## 使い方

```sh
sudo saya capture enable   # apt/apt-get/pacmanのshim設置
sudo apt install neovim    # 通常通り使うだけで自動記録される

sudo saya status           # マニフェストとインストール状態の差分確認
sudo saya apply            # マニフェストにあって未インストールのものを入れる
sudo saya doctor           # shim整合性・PATH順序の検証
```

## リリース手順(開発者向け)

GitHub Actionsの`release`をworkflow_dispatchで実行(patch/minor/major選択)。
`Cargo.toml`バージョン自動算出→更新・コミット・タグpush→x86_64/aarch64のmuslバイナリビルド→GitHub Releasesへ添付、を1本のワークフローで行う。
