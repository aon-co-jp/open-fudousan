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

- **2026-10-18 v0.1.0公開**: `limit`/`offset`ページネーション追加(テスト21件成功・実HTTP確認)、LICENSE(MIT)追加、リポジトリを公開。aruaru-db永続化・認証・open-cg-cadとの実プロセス間連携は引き続き未着手。

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

### Deutsch (KI-Übersetzung, redaktionell überarbeitet; noch nicht von Muttersprachlern geprüft)
- **REST wird vollständig abgeschafft; neue APIs entstehen nicht mehr als REST.** APIs, die aus dem Browser oder per `curl` aufgerufen werden, verwenden **unäre Connect-Aufrufe** (`POST /<Paket>.<Service>/<Methode>`, `content-type: application/json`, JSON-Body). Zwischen Diensten kommen **gRPC bzw. gRPC-web** zum Einsatz, im selben Stil wie die Admin-API von RPoem (`open_runo.admin.v1.Admin`). Streaming wird erst ergänzt, wenn es tatsächlich gebraucht wird.
- **Bestehende REST-Endpunkte werden schrittweise umgestellt, und zwar immer dann, wenn man ohnehin an ihnen arbeitet.** Sobald ein Endpunkt umgestellt ist, wird die alte REST-Route entfernt. Bis zum Abschluss der Migration bleiben die übrigen bestehen, es werden aber **keine neuen mehr hinzugefügt**.
- **JSON wird einheitlich über RS-JSON verarbeitet**: Gelesen wird mit `from_str_secure` bzw. `parse_secure` (mit Obergrenzen für Größe, Verschachtelungstiefe und doppelte Schlüssel), geschrieben mit `rust_json::fast`. `prost` wird nicht in das Browser-WASM aufgenommen, da es die Datei vergrößern würde.
- **Ausnahmen (Schnittstellen, deren Form ein fremder Standard vorgibt und die nicht zu unserer eigenen API gehören)**: ACME, OAuth/OIDC-Callbacks, Webhooks von Drittanbietern (z. B. GitHub), Health-Checks, die Auslieferung statischer Dateien, OTLP und Ähnliches. Diese nehmen, wie vom jeweiligen Standard verlangt, gewöhnliches HTTP entgegen.
- Referenzimplementierungen: `RPoem` (`open_runo.admin.v1.Admin`) und `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Hinweis: Ob der Code eines einzelnen Repositorys diese Richtlinie bereits erfüllt, hängt vom jeweiligen Repository ab. Dieser Abschnitt dokumentiert lediglich die Richtlinie; er besagt nicht, dass der Code schon angepasst wurde.

### Русский (перевод выполнен с помощью ИИ и отредактирован; носителями языка пока не проверен)
- **От REST мы полностью отказываемся; новые API больше не создаются в стиле REST.** API, к которым обращаются из браузера или через `curl`, используют **унарные вызовы Connect** (`POST /<пакет>.<Service>/<Method>`, `content-type: application/json`, тело запроса — JSON). Для взаимодействия между сервисами применяются **gRPC и gRPC-web** — в том же стиле, что и административный API RPoem (`open_runo.admin.v1.Admin`). Потоковая передача данных добавляется лишь тогда, когда в ней действительно возникает необходимость.
- **Существующие REST-эндпоинты переводятся поэтапно — по мере того, как с ними приходится работать.** После перевода старый REST-маршрут удаляется. Пока миграция не завершена, оставшиеся эндпоинты сохраняются, но **новые не добавляются**.
- **Работа с JSON унифицирована на основе RS-JSON**: чтение — через `from_str_secure` или `parse_secure` (с ограничениями на размер, глубину вложенности и повторяющиеся ключи), запись — через `rust_json::fast`. `prost` в браузерный WASM не включается, поскольку это увеличило бы размер файла.
- **Исключения (интерфейсы, форму которых определяет чужой стандарт, а не наш собственный API)**: ACME, обратные вызовы OAuth/OIDC, вебхуки сторонних сервисов (например, GitHub), проверки работоспособности, раздача статических файлов, OTLP и тому подобное. Такие интерфейсы принимают обычный HTTP, как того требует соответствующий стандарт.
- Эталонные реализации: `RPoem` (`open_runo.admin.v1.Admin`) и `open-easy-web` (`server/src/connect.rs`, `proto/open_easy_web/admin/v1/admin.proto`).
- Примечание: соответствует ли код конкретного репозитория этой политике, зависит от самого репозитория. Данный раздел лишь фиксирует политику и не означает, что код уже изменён.

## API 方針の補足: 用語の整理と使い分け (2026-11-04)

### 日本語
**1つに統一するものではなく、別の層の話です。** 「REST の代わり」になるのは Connect・gRPC・GraphQL だけで、WASM は画面を作る技術です。

| 用語 | 何か | REST の代わりか |
|---|---|---|
| REST | URL と HTTP メソッドで操作を表す API の作り方 | 全廃する側 |
| **Connect** | `POST /サービス名/メソッド名` に JSON を送る形式。`curl` でもブラウザの `fetch` でも呼べる | はい(**採用**) |
| **gRPC** | 同じ「サービス/メソッド」の考え方を、バイナリ(protobuf)と HTTP/2 で行う形式。速く、ストリーミングも得意 | はい(**サービス同士に採用**) |
| gRPC-web | gRPC のブラウザ向けの変形版 | はい(必要なら) |
| GraphQL | 欲しい項目を問い合わせる形式 | 一部だけ(**問い合わせ(読み取り)専用**) |
| WASM | Rust をブラウザの中で動かす仕組み | いいえ(画面を作る技術) |

- **ブラウザの画面や `curl` から呼ぶ API は Connect**(POST、JSON を RS-JSON で読み書き)。ブラウザの WASM は素の gRPC を話せず、Connect なら `fetch` のままで、`prost` を足さずに済む(画面のサイズも増えない)。
- **サービス同士の通信(サーバー間、重い処理、ストリーミング)は gRPC。** 多くは Connect で足りるので、ストリーミングが必要になった所だけ gRPC に上げる。
- **GraphQL は、まとめて問い合わせたいときの読み取り専用。** 書き込み(操作)には使わない。
- **WASM は画面を作る技術として、そのまま使う。** 画面から呼ぶ API が Connect になっただけ。
- 外部の規格が形を決めるもの(ACME、OAuth のコールバック、第三者の Webhook、ヘルスチェック、静的ファイル、OTLP)は、規格どおりの HTTP のまま。
- **迷ったら Connect。**

### English
**These are not alternatives to unify; they sit at different layers.** Only Connect, gRPC and GraphQL can replace REST; WASM is a way to build the UI.

| Term | What it is | Replaces REST? |
|---|---|---|
| REST | API style that expresses operations through URLs and HTTP methods | The thing being abolished |
| **Connect** | `POST /Service/Method` with a JSON body; callable from `curl` and from a browser `fetch` | Yes (**adopted**) |
| **gRPC** | The same service/method idea over binary (protobuf) and HTTP/2; fast, good at streaming | Yes (**for service-to-service**) |
| gRPC-web | A browser-friendly variant of gRPC | Yes (if needed) |
| GraphQL | A query language: ask for exactly the fields you want | Partly (**read-only queries**) |
| WASM | Runs Rust inside the browser | No (a UI technology) |

- **APIs called from browser screens or `curl` use Connect** (POST, JSON read and written with RS-JSON). Browser WASM cannot speak plain gRPC; with Connect it stays a `fetch`, and `prost` is not needed (the binary does not grow).
- **Service-to-service calls (server to server, heavy work, streaming) use gRPC.** Connect is enough in most places; move a call to gRPC only where streaming is actually needed.
- **GraphQL is read-only**, for queries that gather several pieces of data. It is not used for writes (operations).
- **WASM stays as the way to build the UI.** The API it calls simply became Connect.
- Interfaces whose shape an outside standard dictates (ACME, OAuth callbacks, third-party webhooks, health checks, static files, OTLP) stay plain HTTP.
- **When in doubt, use Connect.**

### Deutsch (KI-Übersetzung, sprachlich überarbeitet; noch nicht von Muttersprachlern geprüft)
**Es geht nicht darum, eine einzige Technik auszuwählen – die Begriffe liegen auf unterschiedlichen Ebenen.** Als Ersatz für REST kommen nur Connect, gRPC und GraphQL in Frage; WASM dagegen ist eine Technik, mit der die Benutzeroberfläche gebaut wird.

| Begriff | Was ist das? | Ersatz für REST? |
|---|---|---|
| REST | Ein API-Stil, bei dem Operationen über URLs und HTTP-Methoden ausgedrückt werden | wird abgeschafft |
| **Connect** | `POST /Dienst/Methode` mit JSON im Body; aufrufbar mit `curl` ebenso wie mit `fetch` im Browser | ja (**gewählt**) |
| **gRPC** | Dasselbe Prinzip aus Dienst und Methode, aber über binäres protobuf und HTTP/2; schnell und gut für Streaming geeignet | ja (**zwischen Diensten**) |
| gRPC-web | Eine für den Browser angepasste Variante von gRPC | ja (bei Bedarf) |
| GraphQL | Abfragesprache, mit der man genau die benötigten Felder anfordert | teilweise (**nur für lesende Abfragen**) |
| WASM | Führt Rust-Code direkt im Browser aus | nein (Technik für die Oberfläche) |

- **APIs, die von Browseroberflächen oder per `curl` aufgerufen werden, verwenden Connect** (POST; JSON wird mit RS-JSON gelesen und geschrieben). WASM im Browser kann kein reines gRPC sprechen. Mit Connect bleibt es bei einem gewöhnlichen `fetch`, und `prost` ist nicht nötig – die WASM-Datei wird dadurch nicht größer.
- **Für die Kommunikation zwischen Diensten (Server zu Server, rechenintensive Aufgaben, Streaming) wird gRPC eingesetzt.** In den meisten Fällen reicht Connect aus; auf gRPC wird nur dort umgestellt, wo Streaming tatsächlich gebraucht wird.
- **GraphQL dient ausschließlich lesenden Abfragen**, wenn mehrere Informationen gebündelt abgefragt werden sollen. Für Schreibzugriffe (Operationen) wird es nicht verwendet.
- **WASM bleibt die Technik für die Oberfläche.** Geändert hat sich lediglich, dass die aufgerufene API nun auf Connect beruht.
- Schnittstellen, deren Form durch einen externen Standard festgelegt ist (ACME, OAuth-Callbacks, Webhooks von Drittanbietern, Health-Checks, statische Dateien, OTLP), bleiben gewöhnliches HTTP.
- **Im Zweifel: Connect.**

### Русский (перевод выполнен с помощью ИИ и литературно отредактирован; носителями языка пока не проверен)
**Это не варианты, из которых нужно выбрать один, а технологии разных уровней.** Заменить REST могут только Connect, gRPC и GraphQL; WASM же — это способ создания пользовательского интерфейса.

| Термин | Что это | Заменяет REST? |
|---|---|---|
| REST | Стиль построения API, при котором операции выражаются через URL и HTTP-методы | отменяется |
| **Connect** | `POST /Сервис/Метод` с телом в формате JSON; вызывается как из `curl`, так и из браузерного `fetch` | да (**выбран**) |
| **gRPC** | Тот же принцип «сервис — метод», но поверх двоичного формата protobuf и HTTP/2; работает быстро и хорошо подходит для потоковой передачи | да (**для связи между сервисами**) |
| gRPC-web | Адаптированный для браузера вариант gRPC | да (при необходимости) |
| GraphQL | Язык запросов: клиент запрашивает ровно те поля, которые ему нужны | частично (**только запросы на чтение**) |
| WASM | Позволяет выполнять код на Rust прямо в браузере | нет (технология для интерфейса) |

- **API, которые вызываются из браузерных интерфейсов или через `curl`, строятся на Connect** (POST; JSON читается и записывается через RS-JSON). Браузерный WASM не умеет работать с «чистым» gRPC. С Connect достаточно обычного `fetch`, а `prost` не требуется — размер WASM-файла не увеличивается.
- **Для обмена между сервисами (сервер — сервер, ресурсоёмкие задачи, потоковая передача) применяется gRPC.** В большинстве случаев достаточно Connect; на gRPC переходят только там, где потоковая передача действительно необходима.
- **GraphQL используется только для чтения**, когда нужно получить сразу несколько связанных данных. Для записи (операций) он не применяется.
- **WASM по-прежнему остаётся технологией для интерфейса.** Изменилось лишь то, что вызываемый им API теперь построен на Connect.
- Интерфейсы, форма которых задана внешним стандартом (ACME, обратные вызовы OAuth, вебхуки сторонних сервисов, проверки работоспособности, статические файлы, OTLP), остаются обычным HTTP.
- **Если есть сомнения — выбирайте Connect.**
