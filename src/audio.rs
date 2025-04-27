use crate::assets::get_asset_bytes;
use log::warn;
use macroquad::audio::load_sound_from_bytes;
use macroquad::audio::{Sound, play_sound_once};

#[derive(Default)]
pub struct AudioManager {
    fire_sound: Option<Sound>,
    bothit_sound: Option<Sound>,
    death_sound: Option<Sound>,
    wallhit_sound: Option<Sound>,
}

impl AudioManager {
    pub fn new() -> Self {
        Default::default()
    }

    // Load all required sound assets
    pub async fn load_assets(&mut self) {
        self.fire_sound = match get_asset_bytes("fire1.ogg") {
            Some(bytes) => load_sound_from_bytes(bytes.as_ref()).await.ok(),
            None => {
                warn!("Embedded sound fire1.ogg not found");
                None
            }
        };

        self.bothit_sound = match get_asset_bytes("bothit1.ogg") {
            Some(bytes) => load_sound_from_bytes(bytes.as_ref()).await.ok(),
            None => {
                warn!("Embedded sound bothit1.ogg not found");
                None
            }
        };

        self.death_sound = match get_asset_bytes("death1.ogg") {
            Some(bytes) => load_sound_from_bytes(bytes.as_ref()).await.ok(),
            None => {
                warn!("Embedded sound death1.ogg not found");
                None
            }
        };

        self.wallhit_sound = match get_asset_bytes("wallhit1.ogg") {
            Some(bytes) => load_sound_from_bytes(bytes.as_ref()).await.ok(),
            None => {
                warn!("Embedded sound wallhit1.ogg not found");
                None
            }
        };
    }

    // Play the fire sound if loaded
    pub fn play_fire(&self) {
        if let Some(ref sound) = self.fire_sound {
            play_sound_once(sound);
        }
    }

    // Play the hit sound if loaded
    pub fn play_bothit(&self) {
        if let Some(ref sound) = self.bothit_sound {
            play_sound_once(sound);
        }
    }

    // Play the death sound if loaded
    pub fn play_death(&self) {
        if let Some(ref sound) = self.death_sound {
            play_sound_once(sound);
        }
    }

    pub fn play_wallhit(&self) {
        if let Some(ref sound) = self.wallhit_sound {
            play_sound_once(sound);
        }
    }
}
