use chrono::Local;
use log::{LevelFilter, Metadata, Record, SetLoggerError};
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::OnceLock;

// Custom logger structure
#[derive(Debug)]
struct BotArenaLogger {
    level: LevelFilter,
    debug_filters: Option<HashSet<String>>,
}

// Implement the log::Log trait for our custom logger
impl log::Log for BotArenaLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Check if the record's level is enabled
        if metadata.level() <= self.level {
            // If we have debug filters, check if the target matches any filter
            if let Some(filters) = &self.debug_filters {
                if metadata.level() == log::Level::Debug || metadata.level() == log::Level::Trace {
                    return filters.contains(metadata.target())
                        || filters.iter().any(|f| metadata.target().starts_with(f));
                }
            }
            return true;
        }
        false
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level_color = match record.level() {
                log::Level::Error => "\x1B[31m", // Red
                log::Level::Warn => "\x1B[33m",  // Yellow
                log::Level::Info => "\x1B[32m",  // Green
                log::Level::Debug => "\x1B[36m", // Cyan
                log::Level::Trace => "\x1B[35m", // Magenta
            };

            let reset = "\x1B[0m";
            let now = Local::now();
            let timestamp = now.format("%H:%M:%S%.3f");

            // Extract metadata fields
            let mut robot_id: Option<u32> = None;
            let turn: Option<u32> = None;
            let mut cycle: Option<u32> = None;

            // Check if target has robot_id format (robot_N)
            if let Some(id_str) = record.target().strip_prefix("robot_") {
                if let Ok(id) = id_str.parse::<u32>() {
                    robot_id = Some(id);
                }
            }

            // Look for robot ID, turn, and cycle patterns in the message
            let message = record.args().to_string();

            // Look for "Robot N" pattern
            if robot_id.is_none() {
                if let Some(robot_idx) = message.find("Robot ") {
                    if let Some(end_idx) =
                        message[robot_idx + 6..].find(|c: char| !c.is_ascii_digit())
                    {
                        if let Ok(id) =
                            message[robot_idx + 6..robot_idx + 6 + end_idx].parse::<u32>()
                        {
                            robot_id = Some(id);
                        }
                    }
                }
            }

            // Look for Cycle N pattern
            if let Some(cycle_idx) = message.find("Cycle ") {
                if let Some(end_idx) = message[cycle_idx + 6..].find(|c: char| !c.is_ascii_digit())
                {
                    if let Ok(c) = message[cycle_idx + 6..cycle_idx + 6 + end_idx].parse::<u32>() {
                        cycle = Some(c);
                    }
                }
            }

            // Create context prefix with available information
            let mut context = String::new();
            if let Some(id) = robot_id {
                context.push_str(&format!("[R{:02}]", id));
            }
            if let Some(t) = turn {
                context.push_str(&format!("[T{:03}]", t));
            }
            if let Some(c) = cycle {
                context.push_str(&format!("[C{:02}]", c));
            }

            if !context.is_empty() {
                context.push(' ');
            }

            // Standard output format with context
            let mut output = format!(
                "{timestamp} {level_color}{level:5}{reset} {context}{target}: {message}",
                timestamp = timestamp,
                level_color = level_color,
                level = record.level(),
                reset = reset,
                context = context,
                target = record.target(),
                message = record.args()
            );

            // Add module path if available and different from target
            if let Some(module_path) = record.module_path() {
                if module_path != record.target() {
                    output.push_str(&format!(" [{}]", module_path));
                }
            }

            let mut stdout = io::stdout();
            writeln!(stdout, "{}", output).expect("Failed to write to stdout");
            stdout.flush().expect("Failed to flush stdout");
        }
    }

    fn flush(&self) {
        io::stdout().flush().expect("Failed to flush stdout");
    }
}

// Use OnceLock instead of unsafe static mut
static LOGGER: OnceLock<BotArenaLogger> = OnceLock::new();

// Initialize the logger with optional debug filters
pub fn init_logger(level: LevelFilter, debug_filter: Option<String>) -> Result<(), SetLoggerError> {
    let debug_filters = debug_filter.map(|filter_str| {
        filter_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<HashSet<String>>()
    });

    // Initialize the logger if it hasn't been initialized yet
    if LOGGER.get().is_none() {
        let logger = BotArenaLogger {
            level,
            debug_filters,
        };

        // Try to set the logger
        LOGGER.set(logger).expect("Failed to set logger");
    }

    // Set the logger
    log::set_logger(LOGGER.get().unwrap()).map(|()| log::set_max_level(level))
}

// Helper macros for specific debug topics
#[macro_export]
macro_rules! debug_vm {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "vm", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "vm", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "vm", "{}", format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! debug_robot {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "robot", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "robot", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "robot", "{}", format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! debug_drive {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "drive", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "drive", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "drive", "{}", format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! debug_weapon {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "weapon", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "weapon", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "weapon", "{}", format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! debug_scan {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "scan", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "scan", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "scan", "{}", format_args!($($arg)*))
    }
}

#[macro_export]
macro_rules! debug_instructions {
    ($robot_id:expr, $turn:expr, $cycle:expr, $($arg:tt)*) => {
        log::debug!(target: "instructions", "[R{:02}][T{:03}][C{:02}] {}", $robot_id, $turn, $cycle, format_args!($($arg)*))
    };
    ($robot_id:expr, $($arg:tt)*) => {
        log::debug!(target: "instructions", "[R{:02}] {}", $robot_id, format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        log::debug!(target: "instructions", "{}", format_args!($($arg)*))
    }
}

// Robot ID-specific logging functions have been removed as they are not used in the codebase
