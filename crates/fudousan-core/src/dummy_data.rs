//! ダミーの土地/物件データ(5〜10件)。
//!
//! **正直な開示**: ここに列挙する住所・座標・価格等は全て架空であり、
//! 実在の土地・不動産情報とは一切関係がない。実在の不動産ポータル
//! サイトのスクレイピング等は行っていない(本プロジェクトのスコープ外)。

use crate::property::{LatLng, Property, SiteShape, ZoningCategory};

/// 矩形敷地のヘルパー(南西角を`sw`、幅・奥行きを度単位で指定)。
/// 度単位での簡易生成であり、`export::to_local_meters`と組み合わせて
/// メートル単位の面積へ変換する想定。
fn rect_shape(sw: LatLng, width_deg_lng: f64, depth_deg_lat: f64) -> SiteShape {
    SiteShape {
        vertices: vec![
            sw,
            LatLng { lat: sw.lat, lng: sw.lng + width_deg_lng },
            LatLng { lat: sw.lat + depth_deg_lat, lng: sw.lng + width_deg_lng },
            LatLng { lat: sw.lat + depth_deg_lat, lng: sw.lng },
        ],
    }
}

/// ダミー物件データ一式(8件)を返す。
pub fn sample_properties() -> Vec<Property> {
    vec![
        Property {
            id: "dummy-001".to_string(),
            address: "東京都渋谷区ダミー町1-2-3(架空の住所)".to_string(),
            location: LatLng { lat: 35.6595, lng: 139.7005 },
            shape: rect_shape(LatLng { lat: 35.6595, lng: 139.7005 }, 0.0005, 0.0005),
            zoning: ZoningCategory::Type1Residential,
            building_coverage_ratio: 0.6,
            floor_area_ratio: 2.0,
            estimated_price_jpy: 80_000_000,
        },
        Property {
            id: "dummy-002".to_string(),
            address: "東京都世田谷区サンプル台4-5-6(架空の住所)".to_string(),
            location: LatLng { lat: 35.6460, lng: 139.6530 },
            shape: rect_shape(LatLng { lat: 35.6460, lng: 139.6530 }, 0.0008, 0.0006),
            zoning: ZoningCategory::Type1LowRiseResidential,
            building_coverage_ratio: 0.5,
            floor_area_ratio: 1.0,
            estimated_price_jpy: 65_000_000,
        },
        Property {
            id: "dummy-003".to_string(),
            address: "神奈川県横浜市模擬区7-8-9(架空の住所)".to_string(),
            location: LatLng { lat: 35.4437, lng: 139.6380 },
            shape: rect_shape(LatLng { lat: 35.4437, lng: 139.6380 }, 0.0010, 0.0008),
            zoning: ZoningCategory::Type1Residential,
            building_coverage_ratio: 0.6,
            floor_area_ratio: 2.0,
            estimated_price_jpy: 55_000_000,
        },
        Property {
            id: "dummy-004".to_string(),
            address: "大阪府大阪市テスト区10-11-12(架空の住所)".to_string(),
            location: LatLng { lat: 34.6937, lng: 135.5023 },
            shape: rect_shape(LatLng { lat: 34.6937, lng: 135.5023 }, 0.0012, 0.0010),
            zoning: ZoningCategory::NeighborhoodCommercial,
            building_coverage_ratio: 0.8,
            floor_area_ratio: 3.0,
            estimated_price_jpy: 120_000_000,
        },
        Property {
            id: "dummy-005".to_string(),
            address: "愛知県名古屋市見本区13-14-15(架空の住所)".to_string(),
            location: LatLng { lat: 35.1815, lng: 136.9066 },
            shape: rect_shape(LatLng { lat: 35.1815, lng: 136.9066 }, 0.0009, 0.0009),
            zoning: ZoningCategory::Type1Residential,
            building_coverage_ratio: 0.6,
            floor_area_ratio: 2.0,
            estimated_price_jpy: 48_000_000,
        },
        Property {
            id: "dummy-006".to_string(),
            address: "福岡県福岡市ダミー区16-17-18(架空の住所)".to_string(),
            location: LatLng { lat: 33.5904, lng: 130.4017 },
            shape: rect_shape(LatLng { lat: 33.5904, lng: 130.4017 }, 0.0007, 0.0007),
            zoning: ZoningCategory::Type1LowRiseResidential,
            building_coverage_ratio: 0.4,
            floor_area_ratio: 0.8,
            estimated_price_jpy: 32_000_000,
        },
        Property {
            id: "dummy-007".to_string(),
            address: "東京都新宿区サンプル町19-20-21(架空の住所)".to_string(),
            location: LatLng { lat: 35.6938, lng: 139.7036 },
            shape: rect_shape(LatLng { lat: 35.6938, lng: 139.7036 }, 0.0015, 0.0012),
            zoning: ZoningCategory::Commercial,
            building_coverage_ratio: 0.8,
            floor_area_ratio: 4.0,
            estimated_price_jpy: 250_000_000,
        },
        Property {
            id: "dummy-008".to_string(),
            address: "埼玉県さいたま市模擬区22-23-24(架空の住所)".to_string(),
            location: LatLng { lat: 35.8617, lng: 139.6455 },
            shape: rect_shape(LatLng { lat: 35.8617, lng: 139.6455 }, 0.0011, 0.0009),
            zoning: ZoningCategory::QuasiIndustrial,
            building_coverage_ratio: 0.6,
            floor_area_ratio: 2.0,
            estimated_price_jpy: 38_000_000,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_properties_has_at_least_five_entries() {
        let props = sample_properties();
        assert!(props.len() >= 5);
        assert!(props.len() <= 10);
    }

    #[test]
    fn all_ids_are_unique() {
        let props = sample_properties();
        let mut ids: Vec<_> = props.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), props.len());
    }

    #[test]
    fn all_properties_have_valid_ratios() {
        for p in sample_properties() {
            assert!(p.building_coverage_ratio > 0.0 && p.building_coverage_ratio <= 1.0);
            assert!(p.floor_area_ratio > 0.0);
        }
    }
}
