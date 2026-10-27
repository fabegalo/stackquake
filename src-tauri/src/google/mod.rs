//! Conectores das fontes de dados. Cada fonte é um módulo isolado; para
//! adicionar outra, crie um módulo aqui e chame-o em `sync.rs`.

pub mod crashlytics;
pub mod firebase;
pub mod play;
