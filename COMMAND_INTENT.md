# Command intent

## Background

パッケージ追加とmanifest再現に別の動詞が必要だが、独自語彙は操作を迷わせる。pnpm等で定着した`add`と`install`の区別を使えば、manifestを変更する操作か再現する操作かがコマンド名だけで判別できる。

## Intent

`add`は指定パッケージをインストールし、成功後にmanifestへ記録する。`install`はmanifestを変更せず不足パッケージをインストールする。重複する`apply`は持たない。`forget`は一方向同期を守り、manifestから削除してもアンインストールしない。
