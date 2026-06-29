# Manifest write intent

## Background

manifestはchezmoi等の外部ツールでも管理される。内容が同じなのにatomic renameすると、外部管理上は変更として扱われ、反復セットアップで不要な競合確認が生じる。

## Intent

保存前にシリアライズ結果と既存ファイルを比較し、同一ならファイルを変更しない。差分がある場合だけatomic writeする。package managerの実行意味はmanifest保存最適化から分離し、`add`は同一エントリでも指定パッケージをbackendへ渡す。
