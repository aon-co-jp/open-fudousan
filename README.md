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

## API 方針: REST 全廃・gRPC / Connect へ (2026-11-04)

### 日本語
- **REST は全廃する。新しい API は REST で作らない。** ブラウザや `curl` から呼ぶ API は **Connect の単純呼び出し**(`POST /<パッケージ>.<Service>/<Method>`、`content-type: application/json`、本文は JSON)、サービス間は **gRPC / gRPC-web**(RPoem の管理API `open_runo.admin.v1.Admin` と同じ流儀)。ストリーミングは、必要になってから。
- **既存の REST は、触るときに順に移す。** 移したら旧 REST の経路は消す。移行が終わるまで残るが、**新規には追加しない**。
- **JSON は RS-JSON に統一**する(読み込みは `from_str_secure` / `parse_secure` でサイズ・深さ・重複キーの上限つき、書き出しは `rust_json::fast`)。ブラウザの WASM に `prost` は入れない(サイズが増えるため)。
- **例外(自分たちの API ではなく、相手の規格が決めているもの)**: ACME、OAuth/OIDC のコールバック、第三者(GitHub など)からの Webhook、ヘルスチェック、静的ファイル配信、OTLP など。これらは規格どおりの HTTP を受ける。
- 参考実装: `RPoem`(`open_runo.admin.v1.Admin`)、`open-easy-web`(`server/src/connect.rs`、`proto/open_easy_web/admin/v1/admin.proto`)。
- ※ このリポジトリのコードが既に方針どおりかどうかは、各リポジトリの現状による。この節は方針の記録であり、コードの変更を意味しない。

### English
- **REST is being abolished. New APIs are not built as REST.** APIs called from browsers or `curl` use **Connect unary calls** (`POST /<package>.<Service>/<Method>`, `content-type: application/json`, JSON body); service-to-service calls use **gRPC / gRPC-web** (the same style as RPoem's admin API `open_runo.admin.v1.Admin`). Streaming only when it is actually needed.
- **Existing REST endpoints are migrated one by one, when they are touched.** The old REST route is removed once migrated. Until migration is finished they remain, but **no new ones are added**.
- **JSON is unified on RS-JSON**: reading with `from_str_secure` / `parse_secure` (size, depth and duplicate-key limits), writing with `rust_json::fast`. `prost` is not put into browser WASM (it would grow the binary).
- **Exceptions (shapes dictated by someone else's standard, not our own API)**: ACME, OAuth/OIDC callbacks, webhooks from third parties (e.g. GitHub), health checks, static file serving, OTLP, and so on. These accept plain HTTP as the standard requires.
- Reference implementations: `RPoem` (`open_runo.admin.v1.Admin`) and `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Note: whether the code in a given repository already follows this policy depends on that repository. This section records the policy; it does not mean the code has been changed.

### Deutsch (KI-Übersetzung, noch nicht von Muttersprachlern geprüft)
- **REST wird abgeschafft. Neue APIs werden nicht als REST gebaut.** APIs, die aus dem Browser oder mit `curl` aufgerufen werden, nutzen **Connect-Unary-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body); zwischen Diensten gilt **gRPC / gRPC-web** (wie bei der Admin-API von RPoem, `open_runo.admin.v1.Admin`). Streaming erst, wenn es wirklich nötig ist.
- **Bestehende REST-Endpunkte werden einzeln migriert, sobald man sie anfasst.** Nach der Migration wird die alte REST-Route entfernt. Bis dahin bleiben sie bestehen, es werden aber **keine neuen hinzugefügt**.
- **JSON wird auf RS-JSON vereinheitlicht**: Lesen mit `from_str_secure` / `parse_secure` (Grenzen für Größe, Tiefe und doppelte Schlüssel), Schreiben mit `rust_json::fast`. `prost` kommt nicht in Browser-WASM (die Datei würde größer).
- **Ausnahmen (Formate, die ein fremder Standard vorgibt, nicht unsere eigene API)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Dritten (z. B. GitHub), Health-Checks, statische Dateien, OTLP usw. Diese nehmen schlichtes HTTP entgegen, wie es der Standard verlangt.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines Repositorys diese Richtlinie schon befolgt, hängt vom jeweiligen Repository ab. Dieser Abschnitt hält die Richtlinie fest; er bedeutet nicht, dass der Code bereits geändert wurde.

### Русский (машинный перевод, носителями языка ещё не проверен)
- **REST полностью отменяется. Новые API не создаются как REST.** API, вызываемые из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело — JSON); между сервисами — **gRPC / gRPC-web** (так же, как административный API RPoem, `open_runo.admin.v1.Admin`). Потоковая передача — только когда она действительно нужна.
- **Существующие REST-эндпоинты переносятся по одному, когда до них доходят руки.** После переноса старый REST-маршрут удаляется. До завершения переноса они остаются, но **новые не добавляются**.
- **JSON унифицируется на RS-JSON**: чтение через `from_str_secure` / `parse_secure` (ограничения на размер, глубину и повторяющиеся ключи), запись через `rust_json::fast`. `prost` не добавляется в браузерный WASM (размер вырос бы).
- **Исключения (форматы, заданные чужим стандартом, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и т. п. Они принимают обычный HTTP, как требует стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Этот раздел фиксирует политику и не означает, что код уже изменён.
