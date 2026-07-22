# Backend intent

## Background

Archではyayが公式リポジトリとAURを同じ操作で扱い、インストール状態もpacmanのdatabaseへ集約する。pacmanとyayを別backendにすると同一packageを二重管理し、利用者に選択を要求する。AUR buildはroot実行できない。

## Intent

Debian系はAPT、Arch系はyayへ一意に決定する。Archの公式packageとAUR packageは区別せず`yay`配列へ記録する。yayは一般ユーザーとして直接起動し、権限が必要な処理はyay自身へ委ねる。yay未導入とroot実行はerrorにする。
