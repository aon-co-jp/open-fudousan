# open-fudousan

土地・物件検索システム(Rust製、MVP)。**最大の目的は姉妹リポジトリ
[open-cg-cad](https://github.com/aon-co-jp/open-cg-cad)(AI工務店/CAD自動
生成システム)へ、検索した敷地情報をスムーズに引き継ぐこと**である。

## 重要な注記(必読)

- **本リポジトリが扱う物件データは全てダミーデータである**
  (`crates/fudousan-core/src/dummy_data.rs`、8件)。実在の不動産情報では
  ない。
- 実在の不動産ポータルサイトのスクレイピング等は行っていない(スコープ外)。
- 外部不動産APIとの実連携はスコープ外。
- 開発方針・エコシステム共通ルールは
  [`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`が
  正本(本リポジトリの`CLAUDE.md`参照)。

## アーキテクチャ

```
open-fudousan/
├── crates/
│   ├── fudousan-core/   # データモデル(Property等)+ open-cg-cad向けエクスポート変換
│   └── fudousan-api/    # tokio/hyper直接実装のHTTP API(フレームワーク非依存)
```

- **DB非依存・1バイナリ完結・テンプレートエンジン不使用**の方針
  (エコシステム共通、`open-raid-z/CLAUDE.md`参照)に従い、`fudousan-api`は
  Webフレームワーク(Poem等)を使わず`hyper`を直接叩く自前実装。
- `fudousan-core::export`が、`open-cg-cad`の`Site`/`ZoningRules`型が要求
  する入力形式(ローカル平面座標メートル単位・建ぺい率・容積率)への
  変換を提供する(詳細は「open-cg-cadとの連携」参照)。

## ビルド・実行

```sh
cargo build --release
cargo test --release
./target/release/fudousan-api   # デフォルトで http://127.0.0.1:8080 を待受(FUDOUSAN_PORT環境変数で変更可)
```

## API

- `GET /v1/properties/search?area=<住所部分一致>&budget_max=<円>&limit=<1〜200、既定50>&offset=<既定0>` —
  ダミー物件の検索(ページネーション対応。`count`は条件に合う総件数、`returned`は今回返した件数)。
  English: dummy-property search with `limit`/`offset` pagination (`count` = total matches).
- `GET /v1/properties/:id` — 単一物件の詳細。
- `GET /v1/properties/:id/export-for-cad` — open-cg-cad向けエクスポート
  (下記参照)。
- `GET /healthz` — ヘルスチェック。

### 実HTTP検証例

```sh
curl "http://127.0.0.1:8080/v1/properties/search?budget_max=100000000"
curl "http://127.0.0.1:8080/v1/properties/dummy-001/export-for-cad"
```

## open-cg-cadとの連携

`open-cg-cad`の`crates/cg-cad-core/src/site.rs`は
`Site { vertices: Vec<(f64, f64)> }`(ローカル平面座標、メートル単位)を、
`zoning.rs`は`ZoningRules { building_coverage_ratio, floor_area_ratio }`を
要求する(いずれもopen-cg-cad側のソースコードのドキュメントコメントで
確認済み)。

`open-fudousan`側の`Property`は緯度経度(`LatLng`)で敷地形状を保持して
いるため、`export-for-cad`エンドポイントは以下を行う:

1. 緯度経度の頂点列を、1点目を原点とする単純な等長円筒図法近似で
   ローカル平面座標(メートル)へ変換する
   (`fudousan-core/src/export.rs::to_local_meters`)。
   **限界の正直な開示**: 正式な測地投影(UTM等)ではない簡易近似であり、
   数百m四方程度の狭い敷地では実用上十分だが、広域では誤差が拡大する。
2. `building_coverage_ratio`・`floor_area_ratio`をそのまま
   `CadZoningExport`として渡す(open-cg-cadの`ZoningRules::new`にそのまま
   渡せる形)。
3. レスポンスに`is_dummy_data: true`を必ず含め、ダミーデータであることを
   明示する。

## エラーレスポンス

- 不正な`budget_max`(数値でない等)を渡した場合は`400 Bad Request`で
  `{"error":"bad_request","field":"budget_max","message":"..."}`を返す
  (以前は不正値を黙って無視して全件検索と同じ結果を返していたため、
  ユーザーが入力ミスに気付けなかった問題を修正)。
- 存在しない物件IDの場合の404レスポンスには、探したIDそのものを
  `message`に含める(`no property with id 'xxx'`)。
- 未定義ルートの404レスポンスには、利用可能なエンドポイント一覧を
  `message`に含める。

## 検証結果

- `cargo build --release` — 成功(警告なし)。
- `cargo test --release` — `fudousan-core`の単体テスト6件、
  `fudousan-api`の単体テスト13件、全て成功(面積計算・エクスポート
  変換・ダミーデータの一意性・APIルーティング/検索フィルタ/
  エラーレスポンスの内容等)。`fudousan-api`側は元々テストゼロ
  だったため、ルーティング処理を`route()`という同期の純粋関数へ
  切り出し、実サーバーを起動せずに単体テスト可能にした。
- 実サーバー起動 + `curl`/`Invoke-WebRequest`による実HTTP検証 — 4エンドポイント
  (`/healthz`, `/v1/properties/search`, `/v1/properties/:id`,
  `/v1/properties/:id/export-for-cad`)全てで期待通りのJSONレスポンスを
  確認済み(存在しないIDへの404、不正な`budget_max`への400も確認済み)。

## 見送った部分

- **段階4(aruaru-dbへの永続化)**: 見送り。理由は`CLAUDE.md`のHANDOFF
  参照(調査した範囲でaruaru-db側の外部クライアント接続手順の確立に
  追加調査が必要と判断したため、今回のスコープでは未実装)。
- open-cg-cad側は現時点で敷地・法規制チェックまでの実装(`crates/cg-cad-core`)
  であり、HTTP API等の実行可能な統合エンドポイントを持たないため、実際の
  プロセス間連携(HTTP経由でのopen-cg-cad呼び出し等)は行っていない。
  本リポジトリはopen-cg-cadが要求する**データ形式**での出力実装・単体
  検証までにとどめている。


## ライセンス / License

MIT(`LICENSE`参照)。データは全てダミーで、実在の不動産情報ではありません。 / MIT (see `LICENSE`). All data is dummy data, not real estate listings.
