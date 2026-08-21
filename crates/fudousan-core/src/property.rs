//! 土地・物件情報のデータモデル。
//!
//! **重要な注記**: 本クレートが扱う`Property`は全てダミーデータである
//! (`dummy_data`モジュール参照)。実在の不動産ポータルサイトとの連携・
//! スクレイピングは行わない(スコープ外)。

use serde::{Deserialize, Serialize};

/// 緯度経度(WGS84を想定、実際の測地系変換は行わない簡易表現)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

/// 敷地形状。矩形は4頂点の多角形として表現する(polygonのシュガー扱い)。
/// 頂点は緯度経度で保持し、`export`モジュールでopen-cg-cad向けの
/// ローカル平面座標(メートル単位)へ変換する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteShape {
    /// 敷地境界の頂点列(緯度経度、最低3点)。
    pub vertices: Vec<LatLng>,
}

/// 用途地域(法規制上の区分)。日本の都市計画法上の代表的な区分を
/// 簡易的に列挙するのみで、全区分を網羅しない(MVPのため)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoningCategory {
    /// 第一種低層住居専用地域
    Type1LowRiseResidential,
    /// 第一種住居地域
    Type1Residential,
    /// 近隣商業地域
    NeighborhoodCommercial,
    /// 商業地域
    Commercial,
    /// 準工業地域
    QuasiIndustrial,
}

impl ZoningCategory {
    pub fn label_ja(&self) -> &'static str {
        match self {
            ZoningCategory::Type1LowRiseResidential => "第一種低層住居専用地域",
            ZoningCategory::Type1Residential => "第一種住居地域",
            ZoningCategory::NeighborhoodCommercial => "近隣商業地域",
            ZoningCategory::Commercial => "商業地域",
            ZoningCategory::QuasiIndustrial => "準工業地域",
        }
    }
}

/// 土地/物件情報。**全フィールドはダミーデータ生成専用**
/// (`dummy_data::sample_properties()`)であり、実在の物件を表さない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    pub id: String,
    /// 所在地(住所、ダミー表記)。
    pub address: String,
    /// 所在地の代表点(緯度経度)。
    pub location: LatLng,
    /// 敷地形状(境界多角形、緯度経度)。
    pub shape: SiteShape,
    /// 用途地域。
    pub zoning: ZoningCategory,
    /// 建ぺい率(0.0〜1.0)。
    pub building_coverage_ratio: f64,
    /// 容積率(0.0以上)。
    pub floor_area_ratio: f64,
    /// 想定価格(円、ダミー)。
    pub estimated_price_jpy: u64,
}

impl Property {
    /// 敷地面積(平方メートル)。緯度経度からのローカル平面座標変換
    /// (`export::to_local_meters`と同じ簡易投影)を経てシューレース公式で
    /// 算出する。厳密な測地面積計算(測地線・楕円体を考慮したもの)では
    /// ない——小規模な敷地(数百m四方程度)であれば実用上十分な近似。
    pub fn area_m2(&self) -> f64 {
        let local = crate::export::to_local_meters(&self.shape);
        let n = local.len();
        if n < 3 {
            return 0.0;
        }
        let mut sum = 0.0;
        for i in 0..n {
            let (x0, y0) = local[i];
            let (x1, y1) = local[(i + 1) % n];
            sum += x0 * y1 - x1 * y0;
        }
        (sum / 2.0).abs()
    }
}
