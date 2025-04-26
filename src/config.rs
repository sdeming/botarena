//! Configuration constants for the robot arena game.

// Arena and movement
pub const UNIT_SIZE: f64 = 0.05; // 1 unit = 5% of arena width/height
pub const POWER_REGEN_RATE: f64 = 0.01; // Power units regenerated per cycle (1.0 per turn @ 100 cycles/turn)
pub const ARENA_WIDTH_UNITS: u32 = 20; // Default arena width in grid units
pub const ARENA_HEIGHT_UNITS: u32 = 20; // Default arena height in grid units
pub const OBSTACLE_DENSITY: f32 = 0.01; // Default density of obstacles (1%)
pub const SCAN_DISTANCE: f64 = 1.0; // Maximum distance for robot scanning (10 grid units)

// Rendering configuration
pub const WINDOW_WIDTH: i32 = 1000; // Increased width for UI panel
pub const WINDOW_HEIGHT: i32 = 800;
pub const UI_PANEL_WIDTH: i32 = 200; // Width of the side panel
pub const ARENA_WIDTH: i32 = WINDOW_WIDTH - UI_PANEL_WIDTH; // Width for the arena rendering
pub const ARENA_HEIGHT: i32 = WINDOW_HEIGHT; // Arena uses full height
pub const FRAME_RATE: u32 = 60; // Target frame rate

// Game rules
pub const MAX_TURNS: u32 = 1000; // Maximum turns before draw

// Scanner configuration
pub const DEFAULT_SCANNER_FOV: f64 = 45.0; // +/- 22.5 degrees from center
pub const DEFAULT_SCANNER_RANGE: f64 = 1.414; // Maximum arena diagonal (1.0 width + 1.0 height)

// Ranged weapon configuration
pub const DEFAULT_RANGED_DAMAGE: f64 = 10.0; // Base damage before power/distance scaling
pub const DEFAULT_PROJECTILE_SPEED: f64 = 0.2; // Units per cycle (20.0 units per turn at full power)
pub const PROJECTILE_SUB_STEPS: u32 = 1; // Number of steps for projectile collision checks per cycle

// Game rules
pub const CYCLES_PER_TURN: u32 = 100; // Default simulation cycles per turn
pub const DEFAULT_INITIAL_HEALTH: f64 = 100.0;
pub const DEFAULT_INITIAL_POWER: f64 = 1.0;

// Robot Physics/Movement Configuration
pub const MAX_DRIVE_UNITS_PER_TURN: f64 = 5.0;
pub const DRIVE_VELOCITY_FACTOR: f64 = UNIT_SIZE / CYCLES_PER_TURN as f64;
pub const MAX_ROTATION_PER_CYCLE: f64 = 180.0 / CYCLES_PER_TURN as f64; // Degrees/cycle (scaled automatically, e.g., 3.6 deg/cycle for 100 cycles/turn)

// VM configuration
pub const MAX_CALL_STACK_SIZE: usize = 10; // Maximum depth of the call stack for subroutines
