# Command intent

## Background

パッケージ追加とmanifest再現は同じinstall操作であり、独自動詞を増やすと操作を迷わせる。OS package manager固有オプションをsayaが解釈すると、将来の追加に追従が必要になり、値とパッケージ名の区別も曖昧になる。一方、非対話指定は日常的なinstall操作の一部で、APTの`-y`がそのまま使えることを利用者が期待する。

## Intent

`install`はパッケージ指定時に全指定パッケージをインストールして成功後にmanifestへ記録し、未指定時はmanifestの不足分を再現する。installは常に非対話で実行し、共通表現として`-y`も受理する。それ以外のbackend固有引数は`--`以降という明示境界で透過し、sayaは解釈も記録もしない。重複する追加・適用コマンドは持たない。
