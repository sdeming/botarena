mod arena;
mod audio;
mod config;
mod game;
mod logging;
mod particles;
mod render;
mod robot;
mod types;
mod utils;
mod vm;

use crate::config::{ARENA_WIDTH, UI_PANEL_WIDTH, WINDOW_HEIGHT};
use clap::Parser;
use log::{LevelFilter, error, info};
use macroquad::prelude::*;
use std::process;

use crate::audio::AudioManager;
use crate::game::Game;
use crate::logging::init_logger;
use crate::render::Renderer;

// Command line arguments structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Paths to the robot program files (up to 4).
    #[arg(required = true, num_args = 1..=4)]
    robot_files: Vec<String>,

    /// Maximum number of turns for the simulation.
    #[arg(short, long, default_value_t = 1000)]
    max_turns: u32,

    /// Log level (off, error, warn, info, debug, trace).
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Optional comma-separated list of targets for debug/trace logging.
    #[arg(long)]
    debug_filter: Option<String>,

    /// Whether to place obstacles in the arena
    #[arg(long)]
    no_obstacles: bool,

    /// Disable sound effects
    #[arg(long)]
    no_audio: bool,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Bot Arena".to_owned(),
        window_width: (ARENA_WIDTH + UI_PANEL_WIDTH) as i32,
        window_height: WINDOW_HEIGHT as i32,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let args = Args::parse();

    // Parse log level string
    let log_level_filter = match args.log_level.to_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => {
            eprintln!(
                "Warning: Invalid log level '{}'. Defaulting to 'info'.",
                args.log_level
            );
            LevelFilter::Info
        }
    };

    // Setup logger with level and optional filter
    if let Err(e) = init_logger(log_level_filter, args.debug_filter) {
        eprintln!("Failed to set up logging: {}", e);
        process::exit(1);
    }

    info!("Bot Arena starting...");

    // Create Renderer and load fonts
    let mut renderer = Renderer::new();
    renderer.load_title_font().await; // Load title font
    renderer.load_ui_font().await; // Load UI font
    renderer.init_glow_resources();
    renderer.init_scanner_material();

    // Create AudioManager
    let mut audio_manager = AudioManager::new();
    // Load sounds only if --no-audio is NOT specified
    if !args.no_audio {
        audio_manager.load_assets().await;
    }

    // Create Game instance (passing potentially empty audio_manager)
    let mut game = match Game::new(&args.robot_files, args.max_turns, audio_manager) {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to initialize game: {}", e);
            process::exit(1);
        }
    };

    if !args.no_obstacles {
        game.arena.place_obstacles();
    }

    // Run the game loop
    if let Err(e) = game.run(&mut renderer).await {
        error!("Game loop error: {}", e);
        process::exit(1);
    }

    info!("Bot Arena finished.");
}
