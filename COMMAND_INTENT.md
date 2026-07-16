# Command intent

## Background

パッケージ追加とmanifest再現は同じinstall操作であり、独自動詞を増やすと操作を迷わせる。OS package manager固有オプションをsayaが解釈すると、将来の追加に追従が必要になり、値とパッケージ名の区別も曖昧になる。

## Intent

`install`はパッケージ指定時に全指定パッケージをインストールして成功後にmanifestへ記録し、未指定時はmanifestの不足分を再現する。backend固有引数は`--`以降という明示境界で透過し、sayaは解釈も記録もしない。重複する追加・適用コマンドは持たない。
