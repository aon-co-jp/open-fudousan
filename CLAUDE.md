# 開発方針＆開発環境ルール(open-fudousan)

土地・物件検索システム(Rust、MVP)。**最大の目的は姉妹リポジトリ
`F:\runo\open-cg-cad`(AI工務店/CAD自動生成システム、並行開発中)へ、
検索した敷地情報をスムーズに引き継ぐこと**。

エコシステム共通の開発ルール(Rust中心、実機検証必須、正直な開示徹底、
新規フォルダ作成前の相談ルール、DB非依存・1バイナリ完結・テンプレート
エンジン不使用等)は
[`open-raid-z`](https://github.com/aon-co-jp/open-raid-z)の`CLAUDE.md`が
**正本**であり、詳細をここに複製せずそちらを参照する。

## 構成

```
open-fudousan/
├── Cargo.toml (workspace)
├── crates/
│   ├── fudousan-core/   # データモデル(Property)+ open-cg-cad向けエクスポート
│   └── fudousan-api/    # tokio/hyper直接実装のHTTP API
```

## open-cg-cadとの連携方針

`open-cg-cad`の`crates/cg-cad-core/src/site.rs`・`zoning.rs`(2026-08-21
時点で実装済み)が要求する入力形式を事前調査した上で設計した:

- `Site { vertices: Vec<(f64, f64)> }` — ローカル平面座標(メートル単位)。
  GIS的な測地系からの変換はopen-cg-cad側のスコープ外であり、呼び出し側
  (=本リポジトリ)の責務と明記されている。
- `ZoningRules { building_coverage_ratio, floor_area_ratio, ... }` —
  建ぺい率(0.0〜1.0)・容積率(0.0超)。

`fudousan-core::export::export_for_cad()`が`Property`(緯度経度ベース)
からこれらの形式への変換を行う。緯度経度→ローカル平面座標の変換は、
1点目を原点とする単純な等長円筒図法近似
(`export.rs::to_local_meters`のドキュメントコメントに限界を明記)。

## HANDOFF

- **2026-08-21 新規作成・段階0〜3実装・段階4見送り**:
  - **段階0(リポジトリ初期化)**: `CLAUDE.md`/`README.md`/
    `README-English.md`/`PORTING.md`を新規作成。Rustワークスペース
    (`Cargo.toml` + `crates/fudousan-core` + `crates/fudousan-api`)を
    構築。
  - **段階1(データモデル設計)**: `Property`型
    (`crates/fudousan-core/src/property.rs`)を設計。フィールド:
    `address`(所在地)・`location: LatLng`(緯度経度)・
    `shape: SiteShape`(敷地形状、頂点列)・`zoning: ZoningCategory`
    (用途地域、5区分の簡易列挙)・`building_coverage_ratio`(建ぺい率)・
    `floor_area_ratio`(容積率)・`estimated_price_jpy`(想定価格)。
    serdeでJSON化。`area_m2()`は`export::to_local_meters`と同じ投影を
    経てシューレース公式で算出。
  - **段階2(検索API実装)**: `crates/fudousan-api`(hyper直接実装、
    Poem等のフレームワーク非依存)で`GET /v1/properties/search`
    (`area`部分一致・`budget_max`上限フィルタ)・
    `GET /v1/properties/:id`・`GET /healthz`を実装。ダミーデータ8件
    (`dummy_data::sample_properties()`、全て架空の住所・座標・価格)。
    実サーバーを起動し`curl`/PowerShellの`Invoke-WebRequest`で実HTTP
    検証済み(4エンドポイント全て、404ケース含む)。
  - **段階3(open-cg-cadへのデータ引き継ぎ実証)**:
    `GET /v1/properties/:id/export-for-cad`を実装。open-cg-cadの
    `Site`/`ZoningRules`型が要求する形式(ローカル平面座標メートル・
    建ぺい率・容積率)へ変換し、`is_dummy_data: true`を必ず含める。
    open-cg-cad側は現時点でCADコアロジック(`crates/cg-cad-core`)の
    みでHTTP API等の実行可能な統合エンドポイントを持たないため、
    実際のプロセス間連携(HTTP経由の呼び出し)は行っていない——
    **本リポジトリ側のエクスポート機能の実装・単体検証(3件のテスト、
    面積の桁妥当性チェック含む)・実HTTPレスポンス確認までにとどめた**。
  - **段階4(aruaru-dbへの永続化)は見送り**。理由: `aruaru-db`の
    `README.md`/`CLAUDE.md`を確認したところ、PostgreSQL互換の
    ACID+Git-on-SQLハイブリッド構成であることは把握できたが、
    今回のセッション内で外部Rustクライアントからの具体的な接続手順
    (crate選定・接続文字列形式・認証方式)を確立するには追加調査が
    必要と判断。現状は`sample_properties()`によるインメモリ・
    プロセス内データのみで、プロセス再起動でデータは失われる
    (永続化なし)。次回セッションで着手する場合は、`aruaru-db`側の
    クライアント接続実装例(Rustから接続した既存リポジトリがあれば
    それを参照)を先に調査すること。
  - **検証結果**: `cargo build --release`成功。`cargo test --release`で
    `fudousan-core`の単体テスト6件全て成功。実サーバー起動+実HTTP
    リクエスト(`/healthz`, `/v1/properties/search?area=...&budget_max=...`,
    `/v1/properties/dummy-001`, `/v1/properties/dummy-001/export-for-cad`,
    存在しないID→404)を確認済み。
  - 次にすべきこと: (1) aruaru-dbへの実接続調査・実装、(2) open-cg-cad側に
    HTTP API層が実装されたら、実プロセス間連携(open-fudousanの
    エクスポートJSONをopen-cg-cadへ実際にPOSTして結果を受け取る)を
    実証する、(3) ページネーション・認証等の実運用に向けた機能追加。
