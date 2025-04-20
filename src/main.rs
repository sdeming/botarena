mod arena;
mod config;
mod game;
mod logging;
mod particles;
mod render;
mod robot;
mod types;
mod utils;
pub mod vm;

use clap::Parser; // <-- Add clap import
use log::{LevelFilter, info};
use macroquad::prelude::*;
use crate::config::{ARENA_WIDTH, UI_PANEL_WIDTH, WINDOW_HEIGHT};

// --- Command Line Arguments ---
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Robot assembly file(s) to load (1 to 4).
    #[arg(required = true, num_args = 1..=4)]
    robot_files: Vec<String>,

    /// Maximum number of turns to simulate.
    #[arg(long, default_value_t = config::MAX_TURNS)]
    turns: u32,

    /// Debug filter to specify log topics (e.g., "vm,drive,robot,weapon,scan,instructions")
    /// Available topics: vm, robot, drive, weapon, scan, instructions
    #[arg(long)]
    debug_filter: Option<String>,

    /// Log level (off, error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Bot Arena".to_owned(),
        window_width: (ARENA_WIDTH + UI_PANEL_WIDTH) as i32,
        window_height: WINDOW_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize the logger
    let log_level = match args.log_level.to_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    // Setup logger with debug filters if provided
    if let Err(e) = logging::init_logger(log_level, args.debug_filter) {
        eprintln!("Warning: Failed to initialize logger: {}", e);
    }

    info!("Initializing Bot Arena...");

    // Create and initialize the game
    let mut game = game::Game::new(&args.robot_files, args.turns).expect("Failed to create game");

    // Initialize the renderer
    info!("Initializing macroquad rendering system");
    let mut renderer = render::Renderer::new();
    info!("Renderer initialized.");

    // Run the game loop
    game.run(&mut renderer).await.expect("Game loop failed");
}
