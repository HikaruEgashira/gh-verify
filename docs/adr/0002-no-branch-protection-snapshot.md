# ADR-0002: Branch Protection APIのスナップショットを取得しない

## ステータス
採用

## コンテキスト
`required-status-checks` コントロールの初期実装では、GitHub Branch Protection API
(`GET /repos/{owner}/{repo}/branches/{branch}/protection`) を呼び出して
リポジトリの設定スナップショットを取得し、status check が設定されているかを検証していた。

この方法には以下の問題がある。

1. **権限の壁**: Branch Protection API は admin 権限がないと 404 を返す。
   大半のユースケース（CIからの実行、read-only トークン）で evidence 収集が失敗し、
   indeterminate → fail となる。
2. **スナップショットの意味論**: 「status check が設定されているか」は
   検証時点のリポジトリ設定であり、PR がマージされた時点の状態ではない。
   設定は事後に変更可能であり、監査証跡としての信頼性が低い。
3. **検証すべき対象の取り違え**: SDLC 検証の関心事は
   「この PR の check が実際に通ったか」であり、
   「リポジトリに check が設定されているか」ではない。

## 決定
Branch Protection API を呼び出さない。代わりに PR の HEAD コミットに対する
実際の check runs (`GET /repos/{owner}/{repo}/commits/{ref}/check-runs`) と
combined status (`GET /repos/{owner}/{repo}/commits/{ref}/status`) を取得し、
check が通ったかどうかを検証する。

- PR パス: HEAD コミットの check runs を取得して satisfied/violated を判定
- Release パス: check runs は PR 単位の関心事のため `NotApplicable`

## 結果
- admin 権限なしの read-only トークンで全コントロールが動作する
- 「設定されているか」ではなく「通ったか」という事実ベースの検証になる
- Branch Protection API 関連のコード (`repo_api.rs`) を全削除
