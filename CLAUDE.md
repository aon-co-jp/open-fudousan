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

- **2026-08-23 CPU SIMD 適用可否の調査(結論: 適用対象外、コード変更なし)**:
  エコシステム共通の CPU 命令セット検出・ディスパッチ基盤
  [`open-cpu`](https://github.com/aon-co-jp/open-cpu) を
  複数命令セットの組み合わせ対応へ拡張したのに伴い、本リポジトリにも
  適用先があるかを調査した。**結論: 適用対象になる処理は無い。**

  理由(実コードを確認した上での判断):
  - 本リポジトリは物件検索 MVP であり、Rust コードは全体で 800 行未満、
    `fudousan-core`(データモデル+エクスポート)と `fudousan-api`
    (hyper 直叩きの HTTP)の 2 crate のみ。DB もテンプレートエンジンも無い。
  - 物件データは `dummy_data.rs` の **8 件のダミーレコード**で、検索は
    その 8 件に対する部分一致と予算比較。ループ長が 2 桁に満たない。
  - 唯一の浮動小数点演算は `export.rs` の `latlng_to_local_meters`
    (緯度経度→メートルの正距円筒近似)だが、対象は敷地ポリゴンの
    頂点数個であり、SIMD の起動コストの方が大きい。
  - 画像処理・ハッシュ・大量データ集計のいずれも存在しない。

  **無理に SIMD を入れることはしない**という判断を、机上の空論を
  避けるために明記しておく。`open-cpu` への依存も追加していない。
  - 次にすべきこと: 将来 HANDOFF で先送りされている `aruaru-db` への
    永続化が入り、物件件数が数万〜数十万規模の実データになった段階で
    初めて「検索・集計のベクトル化」が検討対象になる。その時点で
    本節を見直すこと。


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

- **2026-08-21 (使いやすさ・完成度向上)**:
  - `fudousan-api`にテストが1件も無かった問題を修正。`main.rs`の
    ルーティング処理(旧`handle()`内にベタ書き)を、hyperの`Incoming`
    ボディに依存しない同期の純粋関数`route(method, path, query, state)`
    へ切り出し、実サーバー起動なしで単体テスト可能にした。13件のテスト
    (クエリパース・検索フィルタ・404/400/405の各レスポンス内容)を追加。
  - UX改善: `budget_max`に数値でない値を渡した場合、従来は黙って無視して
    フィルタなしの全件検索結果を返していた(ユーザーが入力ミスに気付け
    ない)。`400 Bad Request` + 具体的なエラーメッセージ
    (`budget_max must be a non-negative integer (yen), e.g. budget_max=50000000`)
    を返すよう修正。
  - 存在しない物件IDへの404レスポンスに、探したID自体をメッセージへ含める
    ように修正(`{"error":"not_found"}`だけでは原因特定しづらかった)。
  - 未定義ルートへの404レスポンスに、利用可能なエンドポイント一覧を
    メッセージへ含めるように修正。
  - 405(メソッド不許可)のメッセージに実際に受け取ったメソッド名を含める
    ように修正。
  - `cargo build --release`(警告なし)・`cargo test --release`
    (`fudousan-core` 6件 + `fudousan-api` 13件、全19件成功)・実サーバー
    起動+`curl`による3種のエラーレスポンス実HTTP確認、済み。
  - スコープ外(未着手): aruaru-dbへの永続化、認証、ページネーション、
    open-cg-cadとの実プロセス間連携(いずれも従来のHANDOFFで見送り済みの
    項目のまま、今回のセッションでは対応していない)。
