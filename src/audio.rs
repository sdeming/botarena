use log::warn;
use macroquad::audio::{Sound, load_sound, play_sound_once};

#[derive(Default)]
pub struct AudioManager {
    fire_sound: Option<Sound>,
    hit_sound: Option<Sound>,
    death_sound: Option<Sound>,
}

impl AudioManager {
    pub fn new() -> Self {
        Default::default()
    }

    // Load all required sound assets
    pub async fn load_assets(&mut self) {
        self.fire_sound = load_sound("assets/fire1.ogg")
            .await
            .map_err(|e| {
                warn!("Failed to load fire sound 'assets/fire1.ogg': {}", e);
                e
            })
            .ok();

        self.hit_sound = load_sound("assets/boom1.ogg")
            .await
            .map_err(|e| {
                warn!("Failed to load hit sound 'assets/boom1.ogg': {}", e);
                e
            })
            .ok();

        self.death_sound = load_sound("assets/death1.ogg")
            .await
            .map_err(|e| {
                warn!("Failed to load death sound 'assets/death1.ogg': {}", e);
                e
            })
            .ok();
    }

    // Play the fire sound if loaded
    pub fn play_fire(&self) {
        if let Some(ref sound) = self.fire_sound {
            play_sound_once(sound);
        }
    }

    // Play the hit sound if loaded
    pub fn play_hit(&self) {
        if let Some(ref sound) = self.hit_sound {
            play_sound_once(sound);
        }
    }

    // Play the death sound if loaded
    pub fn play_death(&self) {
        if let Some(ref sound) = self.death_sound {
            play_sound_once(sound);
        }
    }
}
