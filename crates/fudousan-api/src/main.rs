//! open-fudousan API サーバー。
//!
//! tokio/hyperを直接用いた自前実装(重量級フレームワーク・ORM不使用の
//! エコシステム方針に準拠、`open-raid-z/CLAUDE.md`参照)。
//!
//! # エンドポイント
//! - `GET /v1/properties/search?area=<部分一致>&budget_max=<円>` — ダミー
//!   物件の検索(全件`fudousan_core::dummy_data::sample_properties()`から)。
//! - `GET /v1/properties/:id` — 単一物件の詳細。
//! - `GET /v1/properties/:id/export-for-cad` — open-cg-cad向けエクスポート。
//!
//! **正直な開示**: 認証・ページネーション・実DB永続化は未実装(MVPの
//! スコープ外、`README.md`/`CLAUDE.md`のHANDOFF参照)。

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use fudousan_core::{dummy_data, export_for_cad, Property};
use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto::Builder as ConnBuilder;
use hyper_util::rt::TokioExecutor;
use tokio::net::TcpListener;

type BoxBody = Full<Bytes>;

struct AppState {
    properties: Vec<Property>,
}

fn json_response(status: StatusCode, value: &impl serde::Serialize) -> Response<BoxBody> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// クエリ文字列 `a=1&b=2` を単純パースする(URLデコードは`%20`程度の
/// 基本的なものに限定、依存クレート追加を避けるための簡易実装)。
fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            Some((simple_url_decode(k), simple_url_decode(v)))
        })
        .collect()
}

fn simple_url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let Ok(byte) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(byte);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn bad_request(field: &str, message: &str) -> Response<BoxBody> {
    json_response(
        StatusCode::BAD_REQUEST,
        &serde_json::json!({ "error": "bad_request", "field": field, "message": message }),
    )
}

/// メソッド・パス・クエリ文字列からルーティングを行い、レスポンスを組み立てる
/// 同期の純粋関数。hyperの`Incoming`ボディに依存しないため、単体テストで
/// 実際のHTTPサーバーを起動せずにルーティング・エラーハンドリングを検証できる。
fn route(method: &Method, path: &str, query: &str, state: &AppState) -> Response<BoxBody> {
    if method != Method::GET {
        return json_response(
            StatusCode::METHOD_NOT_ALLOWED,
            &serde_json::json!({
                "error": "method_not_allowed",
                "message": format!("this endpoint only supports GET, got {method}"),
            }),
        );
    }

    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    match segments.as_slice() {
        ["v1", "properties", "search"] => {
            let params = parse_query(query);
            let area_filter = params
                .iter()
                .find(|(k, _)| k == "area")
                .map(|(_, v)| v.clone());

            let budget_max_raw = params.iter().find(|(k, _)| k == "budget_max").map(|(_, v)| v.clone());
            let budget_max: Option<u64> = match budget_max_raw {
                None => None,
                Some(ref raw) if raw.is_empty() => None,
                Some(ref raw) => match raw.parse::<u64>() {
                    Ok(v) => Some(v),
                    Err(_) => {
                        return bad_request(
                            "budget_max",
                            "budget_max must be a non-negative integer (yen), e.g. budget_max=50000000",
                        );
                    }
                },
            };

            let results: Vec<&Property> = state
                .properties
                .iter()
                .filter(|p| {
                    area_filter
                        .as_ref()
                        .map(|a| p.address.contains(a.as_str()))
                        .unwrap_or(true)
                })
                .filter(|p| budget_max.map(|max| p.estimated_price_jpy <= max).unwrap_or(true))
                .collect();

            json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "count": results.len(),
                    "results": results,
                    "is_dummy_data": true,
                }),
            )
        }
        ["v1", "properties", id] => match state.properties.iter().find(|p| p.id == *id) {
            Some(p) => json_response(StatusCode::OK, p),
            None => not_found_property(id),
        },
        ["v1", "properties", id, "export-for-cad"] => {
            match state.properties.iter().find(|p| p.id == *id) {
                Some(p) => {
                    let export = export_for_cad(p);
                    json_response(StatusCode::OK, &export)
                }
                None => not_found_property(id),
            }
        }
        ["healthz"] => json_response(StatusCode::OK, &serde_json::json!({ "status": "ok" })),
        _ => json_response(
            StatusCode::NOT_FOUND,
            &serde_json::json!({
                "error": "not_found",
                "message": "unknown route; see GET /v1/properties/search, GET /v1/properties/:id, GET /v1/properties/:id/export-for-cad, GET /healthz",
            }),
        ),
    }
}

/// 物件IDが見つからない場合の404。どのIDを探して見つからなかったかを
/// レスポンスに含め、`{"error":"not_found"}`だけよりも原因を特定しやすくする。
fn not_found_property(id: &str) -> Response<BoxBody> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({
            "error": "not_found",
            "message": format!("no property with id '{id}'"),
        }),
    )
}

async fn handle(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    // consume/drop body (all endpoints here are GET, no body expected)
    let _ = req.into_body().collect().await;

    Ok(route(&method, &path, &query, &state))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AppState { properties: dummy_data::sample_properties() });

    let port: u16 = std::env::var("FUDOUSAN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;
    eprintln!("open-fudousan API listening on http://{addr} (dummy data only)");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let state = state.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req| handle(state.clone(), req));
            if let Err(err) = ConnBuilder::new(TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                eprintln!("connection error: {err}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        AppState { properties: dummy_data::sample_properties() }
    }

    /// レスポンスボディ(`Full<Bytes>`)をJSONへ変換する。`Full`は既に確定した
    /// バイト列を包むだけのbodyなので、`block_on`相当のブロッキング待機で
    /// 十分(このテストモジュールは`#[tokio::test]`のasyncコンテキスト内で
    /// 呼ぶため`.await`で安全に取り出せる)。
    async fn body_json(resp: Response<BoxBody>) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.expect("infallible").to_bytes();
        serde_json::from_slice(&bytes).expect("response body must be JSON")
    }

    #[test]
    fn parse_query_decodes_percent_and_plus() {
        let params = parse_query("area=%E6%9D%B1%E4%BA%AC&budget_max=100");
        assert_eq!(params[0], ("area".to_string(), "東京".to_string()));
        assert_eq!(params[1], ("budget_max".to_string(), "100".to_string()));

        let params = parse_query("q=a+b");
        assert_eq!(params[0], ("q".to_string(), "a b".to_string()));
    }

    #[test]
    fn parse_query_handles_empty_string() {
        assert!(parse_query("").is_empty());
    }

    #[test]
    fn healthz_returns_ok() {
        let state = test_state();
        let resp = route(&Method::GET, "/healthz", "", &state);
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn search_without_params_returns_all_dummy_properties() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/search", "", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let expected_count = state.properties.len();
        let json = body_json(resp).await;
        assert_eq!(json["count"], expected_count);
        assert_eq!(json["is_dummy_data"], true);
    }

    #[tokio::test]
    async fn search_filters_by_area_substring() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/search", "area=%E6%B8%8B%E8%B0%B7", &state); // "渋谷"
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["count"], 1);
    }

    #[tokio::test]
    async fn search_filters_by_budget_max() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/search", "budget_max=40000000", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        let count = json["count"].as_u64().unwrap();
        assert!(count > 0, "at least one dummy property is <= 40,000,000 yen");
        for p in json["results"].as_array().unwrap() {
            assert!(p["estimated_price_jpy"].as_u64().unwrap() <= 40_000_000);
        }
    }

    #[tokio::test]
    async fn search_with_invalid_budget_max_returns_400_with_helpful_message() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/search", "budget_max=not-a-number", &state);
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let json = body_json(resp).await;
        assert_eq!(json["field"], "budget_max");
        assert!(json["message"].as_str().unwrap().contains("non-negative integer"));
    }

    #[tokio::test]
    async fn get_property_by_id_returns_property() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/dummy-001", "", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["id"], "dummy-001");
    }

    #[tokio::test]
    async fn get_unknown_property_returns_404_with_id_in_message() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/does-not-exist", "", &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = body_json(resp).await;
        assert!(json["message"].as_str().unwrap().contains("does-not-exist"));
    }

    #[test]
    fn export_for_cad_unknown_property_returns_404() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/does-not-exist/export-for-cad", "", &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn export_for_cad_known_property_includes_dummy_flag() {
        let state = test_state();
        let resp = route(&Method::GET, "/v1/properties/dummy-001/export-for-cad", "", &state);
        assert_eq!(resp.status(), StatusCode::OK);
        let json = body_json(resp).await;
        assert_eq!(json["is_dummy_data"], true);
        assert_eq!(json["property_id"], "dummy-001");
    }

    #[tokio::test]
    async fn unknown_route_returns_404_with_endpoint_hint() {
        let state = test_state();
        let resp = route(&Method::GET, "/nonexistent", "", &state);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let json = body_json(resp).await;
        assert!(json["message"].as_str().unwrap().contains("/v1/properties/search"));
    }

    #[test]
    fn non_get_method_returns_405() {
        let state = test_state();
        let resp = route(&Method::POST, "/v1/properties/search", "", &state);
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
