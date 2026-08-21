# PORTING

現時点(2026-08-21新規作成)では移植元・移植先の対応関係は無い
(新規リポジトリとして設計・実装したもの)。

- APIエンドポイント形状(`GET /v1/xxx?param=value`)は`RS-EC`
  (`aon-co-jp/RS-EC`)等、エコシステム内の他Rust APIリポジトリの
  URL設計パターンを踏襲。ただし`RS-EC`はPoemフレームワークに依存する
  のに対し、本リポジトリは`open-raid-z/CLAUDE.md`の方針
  (DB非依存・1バイナリ完結・テンプレートエンジン不使用)により
  `hyper`直接実装とした点が異なる。
- データモデル(`Site`のローカル平面座標表現等)は`open-cg-cad`
  (`crates/cg-cad-core/src/site.rs`)の既存実装に合わせて設計した
  (詳細はREADME.md「open-cg-cadとの連携」参照)。
