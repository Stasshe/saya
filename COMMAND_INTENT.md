# Command intent

## Background

パッケージ追加とmanifest再現は同じinstall操作であり、独自動詞を増やすと操作を迷わせる。未記載packageを削除するとOS標準packageまで対象になるが、uninstallした名前を記録しなければ別環境で削除意図を再現できない。schema互換を通常の読み込みへ蓄積するとmanifest実装が旧形式へ拘束される。OS package manager固有オプションをsayaが解釈すると、将来の追加に追従が必要になり、値とパッケージ名の区別も曖昧になる。

## Intent

manifestはpackageを`present`、`absent`、未記載の管理対象外に分ける。`install <name...>`は導入成功後に`present`へ移し、`uninstall <name...>`は削除成功後に`absent`へ移す。引数なし`install`は両状態を適用する。削除意図は自動消去せず、管理をやめる場合だけ利用者がmanifestから消す。schema移行は明示的な`migrate`で直前の一世代だけ扱い、自動移行と多世代互換は持たない。installは常に非対話で実行し、共通表現として`-y`も受理する。それ以外のbackend固有引数は`--`以降という明示境界で透過し、sayaは解釈も記録もしない。
