use rust_embed::RustEmbed;
use std::borrow::Cow;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Asset;

pub fn get_asset_bytes(name: &str) -> Option<Cow<'static, [u8]>> {
    Asset::get(name).map(|f| f.data)
}
