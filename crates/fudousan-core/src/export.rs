//! open-cg-cad向けエクスポート形式。
//!
//! open-cg-cad (`crates/cg-cad-core/src/site.rs`, `zoning.rs`) が要求する
//! 入力形式は以下の通り(open-cg-cad側のドキュメントコメントより確認済み):
//! - `Site { vertices: Vec<(f64, f64)> }` — メートル単位のローカル平面座標
//!   (GIS的な測地系からの変換は呼び出し側=本クレートの責務、との明記あり)。
//! - `ZoningRules { building_coverage_ratio: f64, floor_area_ratio: f64 }`
//!   (0.0〜1.0の建ぺい率、0.0超の容積率)。
//!
//! 本モジュールは`Property`(緯度経度ベース)をこの形式へ変換する。

use serde::{Deserialize, Serialize};

use crate::property::{LatLng, Property, SiteShape};

/// open-cg-cad `Site::polygon` にそのまま渡せる形式
/// (`crates/cg-cad-core/src/site.rs`の`Site { vertices: Vec<(f64,f64)> }`と
/// フィールド互換)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadSiteExport {
    /// ローカル平面座標(メートル単位)の頂点列。
    pub vertices: Vec<(f64, f64)>,
}

/// open-cg-cad `ZoningRules::new`にそのまま渡せる形式。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CadZoningExport {
    pub building_coverage_ratio: f64,
    pub floor_area_ratio: f64,
}

/// `GET /v1/properties/:id/export-for-cad` のレスポンス本体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadExport {
    pub property_id: String,
    pub site: CadSiteExport,
    pub zoning: CadZoningExport,
    /// 敷地面積(参考値、open-cg-cad側でも`Site::area_m2()`で再計算可能)。
    pub site_area_m2: f64,
    /// このエクスポートがダミーデータ由来であることを明示するフラグ
    /// (open-cg-cad側で誤って実データ扱いされないための正直な開示)。
    pub is_dummy_data: bool,
}

/// 緯度経度の頂点列を、1点目を原点とするローカル平面座標(メートル単位)へ
/// 変換する。
///
/// **近似の限界(正直な開示)**: 正しい測地学的投影(UTM等)は行わず、
/// 単純な等長円筒図法(equirectangular)近似を用いる——1点目の緯度を
/// 基準に「緯度1度=111,320m」「経度1度=111,320m×cos(緯度)」で換算する
/// だけの単純な接平面近似。数百m四方程度の狭い敷地であれば誤差は
/// 実用上無視できるが、広域(km単位)や極域に近い座標では誤差が拡大する。
/// より厳密な投影が必要な場合はGISライブラリ(proj等)の導入を検討すること
/// (現時点では車輪の再発明を避けるため未導入、将来課題)。
pub fn to_local_meters(shape: &SiteShape) -> Vec<(f64, f64)> {
    const METERS_PER_DEGREE_LAT: f64 = 111_320.0;

    let Some(origin) = shape.vertices.first().copied() else {
        return Vec::new();
    };
    let meters_per_degree_lng = METERS_PER_DEGREE_LAT * origin.lat.to_radians().cos();

    shape
        .vertices
        .iter()
        .map(|v| {
            let x = (v.lng - origin.lng) * meters_per_degree_lng;
            let y = (v.lat - origin.lat) * METERS_PER_DEGREE_LAT;
            (x, y)
        })
        .collect()
}

/// 単一頂点のヘルパー(テスト・単純変換用)。
pub fn latlng_to_local_meters(origin: LatLng, point: LatLng) -> (f64, f64) {
    const METERS_PER_DEGREE_LAT: f64 = 111_320.0;
    let meters_per_degree_lng = METERS_PER_DEGREE_LAT * origin.lat.to_radians().cos();
    let x = (point.lng - origin.lng) * meters_per_degree_lng;
    let y = (point.lat - origin.lat) * METERS_PER_DEGREE_LAT;
    (x, y)
}

/// `Property`をopen-cg-cad向けエクスポート形式へ変換する。
pub fn export_for_cad(property: &Property) -> CadExport {
    let vertices = to_local_meters(&property.shape);
    let site_area_m2 = property.area_m2();
    CadExport {
        property_id: property.id.clone(),
        site: CadSiteExport { vertices },
        zoning: CadZoningExport {
            building_coverage_ratio: property.building_coverage_ratio,
            floor_area_ratio: property.floor_area_ratio,
        },
        site_area_m2,
        is_dummy_data: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::ZoningCategory;

    fn sample_property() -> Property {
        Property {
            id: "dummy-001".to_string(),
            address: "東京都渋谷区ダミー町1-2-3(架空の住所)".to_string(),
            location: LatLng { lat: 35.6595, lng: 139.7005 },
            shape: SiteShape {
                vertices: vec![
                    LatLng { lat: 35.6595, lng: 139.7005 },
                    LatLng { lat: 35.6595, lng: 139.7010 },
                    LatLng { lat: 35.6600, lng: 139.7010 },
                    LatLng { lat: 35.6600, lng: 139.7005 },
                ],
            },
            zoning: ZoningCategory::Type1Residential,
            building_coverage_ratio: 0.6,
            floor_area_ratio: 2.0,
            estimated_price_jpy: 80_000_000,
        }
    }

    #[test]
    fn origin_vertex_maps_to_zero_zero() {
        let p = sample_property();
        let local = to_local_meters(&p.shape);
        assert_eq!(local[0], (0.0, 0.0));
    }

    #[test]
    fn export_carries_zoning_and_dummy_flag() {
        let p = sample_property();
        let export = export_for_cad(&p);
        assert_eq!(export.zoning.building_coverage_ratio, 0.6);
        assert_eq!(export.zoning.floor_area_ratio, 2.0);
        assert!(export.is_dummy_data);
        assert_eq!(export.site.vertices.len(), 4);
        assert!(export.site_area_m2 > 0.0);
    }

    #[test]
    fn roughly_rectangular_site_area_is_plausible() {
        // 約 0.0005度 x 0.0005度 の矩形 ≒ 約55.5m x 45.6m(緯度35.66度付近)
        // ≒ 約2500m2程度のオーダーになるはず(厳密一致は求めない、桁の妥当性のみ確認)。
        let p = sample_property();
        let area = p.area_m2();
        assert!(area > 1000.0 && area < 5000.0, "area_m2 = {area}");
    }
}
