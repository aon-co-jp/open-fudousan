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

fn not_found() -> Response<BoxBody> {
    json_response(
        StatusCode::NOT_FOUND,
        &serde_json::json!({ "error": "not_found" }),
    )
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

async fn handle(
    state: Arc<AppState>,
    req: Request<Incoming>,
) -> Result<Response<BoxBody>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    // consume/drop body (all endpoints here are GET, no body expected)
    let _ = req.into_body().collect().await;

    if method != Method::GET {
        return Ok(json_response(
            StatusCode::METHOD_NOT_ALLOWED,
            &serde_json::json!({ "error": "method_not_allowed" }),
        ));
    }

    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();

    match segments.as_slice() {
        ["v1", "properties", "search"] => {
            let params = parse_query(&query);
            let area_filter = params
                .iter()
                .find(|(k, _)| k == "area")
                .map(|(_, v)| v.clone());
            let budget_max: Option<u64> = params
                .iter()
                .find(|(k, _)| k == "budget_max")
                .and_then(|(_, v)| v.parse().ok());

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

            Ok(json_response(
                StatusCode::OK,
                &serde_json::json!({
                    "count": results.len(),
                    "results": results,
                    "is_dummy_data": true,
                }),
            ))
        }
        ["v1", "properties", id] => {
            match state.properties.iter().find(|p| p.id == *id) {
                Some(p) => Ok(json_response(StatusCode::OK, p)),
                None => Ok(not_found()),
            }
        }
        ["v1", "properties", id, "export-for-cad"] => {
            match state.properties.iter().find(|p| p.id == *id) {
                Some(p) => {
                    let export = export_for_cad(p);
                    Ok(json_response(StatusCode::OK, &export))
                }
                None => Ok(not_found()),
            }
        }
        ["healthz"] => Ok(json_response(StatusCode::OK, &serde_json::json!({ "status": "ok" }))),
        _ => Ok(not_found()),
    }
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
