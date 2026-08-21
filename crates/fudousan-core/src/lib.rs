//! open-fudousan: 土地・物件検索システムのデータモデル(コアクレート)。
//!
//! # 免責事項(必読)
//!
//! - 本クレートが扱う`Property`は**すべてダミーデータ**であり、
//!   実在の不動産情報ではない(`dummy_data`モジュール参照)。
//! - 実在の不動産ポータルサイトのスクレイピング等は行っていない。
//! - 外部不動産APIとの実連携はスコープ外。
//!
//! # open-cg-cadとの連携
//!
//! `export`モジュールが、姉妹リポジトリ`open-cg-cad`
//! (`crates/cg-cad-core`)の`Site`/`ZoningRules`型が要求する入力形式
//! (ローカル平面座標・建ぺい率・容積率)への変換を提供する。

pub mod dummy_data;
pub mod export;
pub mod property;

pub use export::{export_for_cad, to_local_meters, CadExport, CadSiteExport, CadZoningExport};
pub use property::{LatLng, Property, SiteShape, ZoningCategory};
