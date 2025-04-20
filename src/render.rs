use crate::arena::*;
use crate::config::*;
use crate::particles::ParticleSystem; // <-- Import ParticleSystem
use crate::robot::Robot; // Import Robot
use crate::robot::RobotStatus; // Import RobotStatus enum
use crate::types::*;
use crate::utils;
use log::info; // Add log import
use raylib::prelude::*;
use std::os::raw::{c_char, c_int, c_void};

// Handles rendering the simulation state using Raylib
pub struct Renderer {
    rl: RaylibHandle,
    thread: RaylibThread,
}

// Add Drop implementation to handle cleanup with proper logging
impl Drop for Renderer {
    fn drop(&mut self) {
        // Add our own properly formatted logs for cleanup
        info!("Unloading Raylib resources (textures, shaders, and window)");
    }
}

// Foreign C types for Raylib callback
#[allow(dead_code)]
unsafe extern "C" fn silent_log_callback(
    _log_level: c_int,
    _text: *const c_char,
    _args: *mut c_void,
) {
    // Intentionally empty - this silences all Raylib logging
}

impl Renderer {
    // Creates a new Renderer instance and initializes Raylib
    pub fn new() -> Self {
        // Disable Raylib logging by setting trace log level to NONE (0)
        let (mut rl, thread) = raylib::init()
            .size(WINDOW_WIDTH, WINDOW_HEIGHT)
            .title("Bot Arena")
            .vsync()
            .msaa_4x()
            .log_level(raylib::consts::TraceLogLevel::LOG_NONE)
            .build();

        rl.set_target_fps(FRAME_RATE);

        Renderer { rl, thread }
    }

    // Main drawing loop - accepts time info for interpolation
    pub fn draw_frame(
        &mut self,
        arena: &Arena,
        robots: &[Robot],
        particle_system: &ParticleSystem,
        current_turn: u32,
        max_turns: u32,
        current_cycle: u32,
        cycles_per_turn: u32,
        time_accumulator: f32,
        cycle_duration: f32,
        announcement: Option<&str>,
    ) {
        let mut d = self.rl.begin_drawing(&self.thread);
        d.clear_background(Color::BLACK);
        let alpha = (time_accumulator / cycle_duration).clamp(0.0, 1.0);
        Self::draw_arena_boundaries(&mut d, arena, ARENA_WIDTH, ARENA_HEIGHT);
        Self::draw_obstacles(&mut d, arena, ARENA_WIDTH, ARENA_HEIGHT);
        for robot in robots {
            Self::draw_robot(&mut d, robot, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64);
        }
        Self::draw_projectiles(&mut d, arena, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64);
        Self::draw_particles(&mut d, particle_system, ARENA_WIDTH, ARENA_HEIGHT, alpha);
        Self::draw_ui_panel(
            &mut d,
            robots,
            current_turn,
            max_turns,
            current_cycle,
            cycles_per_turn,
        );
        d.draw_fps(10, 10);
        if let Some(msg) = announcement {
            Self::draw_announcement(&mut d, msg);
        }
    }

    // Checks if the window should close
    pub fn window_should_close(&self) -> bool {
        self.rl.window_should_close()
    }

    // Converts arena world coordinates (0.0-1.0) to screen coordinates within the arena panel
    fn world_to_screen(point: Point, arena_screen_width: i32, arena_screen_height: i32) -> Vector2 {
        Vector2 {
            x: (point.x * arena_screen_width as f64) as f32,
            y: (point.y * arena_screen_height as f64) as f32,
        }
    }

    // Converts arena world size (0.0-1.0) to screen size within the arena panel
    fn world_size_to_screen(size: f64, arena_screen_width: i32, arena_screen_height: i32) -> f32 {
        // Use the minimum dimension of the arena panel for scaling to maintain aspect ratio
        (size * arena_screen_width.min(arena_screen_height) as f64) as f32
    }

    // Draws the arena boundaries within the designated area
    fn draw_arena_boundaries(
        d: &mut RaylibDrawHandle,
        _arena: &Arena,
        arena_screen_width: i32,
        arena_screen_height: i32,
    ) {
        d.draw_rectangle_lines(
            1,
            1,
            arena_screen_width - 2,
            arena_screen_height - 2,
            Color::GRAY,
        );
    }

    // Draws the obstacles within the arena area
    fn draw_obstacles(
        d: &mut RaylibDrawHandle,
        arena: &Arena,
        arena_screen_width: i32,
        arena_screen_height: i32,
    ) {
        let obstacle_screen_size =
            Self::world_size_to_screen(arena.unit_size, arena_screen_width, arena_screen_height);
        let half_size = obstacle_screen_size / 2.0;

        for obstacle in &arena.obstacles {
            let screen_pos =
                Self::world_to_screen(obstacle.position, arena_screen_width, arena_screen_height);
            d.draw_rectangle(
                (screen_pos.x - half_size) as i32,
                (screen_pos.y - half_size) as i32,
                obstacle_screen_size as i32,
                obstacle_screen_size as i32,
                Color::DARKGRAY,
            );
        }
    }

    // Draws a single robot within the arena area, using interpolation
    fn draw_robot(
        d: &mut RaylibDrawHandle,
        robot: &Robot,
        arena_screen_width: i32,
        arena_screen_height: i32,
        alpha: f64, // Interpolation factor (0.0 to 1.0)
    ) {
        let robot_screen_size =
            Self::world_size_to_screen(UNIT_SIZE, arena_screen_width, arena_screen_height);
        let radius = robot_screen_size / 2.0;

        // --- Interpolate State ---
        let interp_pos = utils::lerp_point(robot.prev_position, robot.position, alpha);
        let interp_drive_deg =
            utils::angle_lerp(robot.prev_drive_direction, robot.drive.direction, alpha);
        let interp_turret_deg =
            utils::angle_lerp(robot.prev_turret_direction, robot.turret.direction, alpha);

        // Convert interpolated world position to screen position
        let center_pos = Self::world_to_screen(interp_pos, arena_screen_width, arena_screen_height);

        let body_color = match robot.id {
            1 => Color::BLUE,
            2 => Color::RED,
            3 => Color::GREEN,
            4 => Color::YELLOW,
            _ => Color::PURPLE,
        };

        // 1. Draw ghost for drive intended direction (if there's pending rotation)
        if robot.drive.pending_rotation.abs() > 0.1 {
            // Calculate target drive direction
            let target_drive_deg = (robot.drive.direction + robot.drive.pending_rotation) % 360.0;

            // Draw ghost triangle with reduced alpha
            let ghost_color = Color {
                r: body_color.r,
                g: body_color.g,
                b: body_color.b,
                a: 180, // Increased opacity from 80 to 180
            };

            Self::draw_triangle_at_angle(
                d,
                center_pos,
                radius * 0.9, // Slightly smaller
                target_drive_deg as f32,
                ghost_color,
                true, // Added outline to make it more visible
            );

            // Add an explicit line showing rotation direction
            let angle_rad = robot.drive.direction.to_radians() as f32;
            let target_angle_rad = target_drive_deg.to_radians() as f32;
            let arc_radius = radius * 1.2;
            let arc_center = center_pos;

            // Draw an arc to show rotation direction
            let steps = 10;
            let mut prev_point = Vector2 {
                x: arc_center.x + angle_rad.cos() * arc_radius,
                y: arc_center.y + angle_rad.sin() * arc_radius,
            };

            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let intermediate_angle = angle_rad * (1.0 - t) + target_angle_rad * t;
                let current_point = Vector2 {
                    x: arc_center.x + intermediate_angle.cos() * arc_radius,
                    y: arc_center.y + intermediate_angle.sin() * arc_radius,
                };

                d.draw_line_v(
                    prev_point,
                    current_point,
                    Color {
                        r: 255,
                        g: 255,
                        b: 0,
                        a: 200,
                    },
                );
                prev_point = current_point;
            }
        }

        // 2. Draw ghost for turret intended direction (if there's pending rotation)
        if robot.turret.pending_rotation.abs() > 0.1 {
            // Calculate target turret direction
            let target_turret_deg =
                (robot.turret.direction + robot.turret.pending_rotation) % 360.0;

            // Draw ghosted turret line with solid line first (more visible)
            let turret_rad = target_turret_deg.to_radians() as f32;
            let turret_end_x = center_pos.x + turret_rad.cos() * radius * 0.8;
            let turret_end_y = center_pos.y + turret_rad.sin() * radius * 0.8;

            // Draw ghost turret line with more visible color
            d.draw_line_v(
                center_pos,
                Vector2 {
                    x: turret_end_x,
                    y: turret_end_y,
                },
                Color {
                    r: 255,
                    g: 200,
                    b: 0,
                    a: 200,
                }, // Bright yellow with higher opacity
            );

            // Draw a small circle at the end of the ghost turret line
            d.draw_circle_v(
                Vector2 {
                    x: turret_end_x,
                    y: turret_end_y,
                },
                3.0,
                Color {
                    r: 255,
                    g: 200,
                    b: 0,
                    a: 200,
                },
            );
        }

        // --- Draw actual robot body ---
        Self::draw_triangle_at_angle(
            d,
            center_pos,
            radius,
            interp_drive_deg as f32,
            body_color,
            true, // With outline
        );

        // --- Draw actual turret Direction Indicator ---
        let turret_rad = interp_turret_deg.to_radians() as f32;
        let turret_end_x = center_pos.x + turret_rad.cos() * radius * 0.8;
        let turret_end_y = center_pos.y + turret_rad.sin() * radius * 0.8;
        d.draw_line_v(
            center_pos,
            Vector2 {
                x: turret_end_x,
                y: turret_end_y,
            },
            Color::LIGHTGRAY,
        );
    }

    // Helper method to draw a triangle rotated to a specific angle
    fn draw_triangle_at_angle(
        d: &mut RaylibDrawHandle,
        center_pos: Vector2,
        radius: f32,
        angle_deg: f32,
        color: Color,
        with_outline: bool,
    ) {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let p1_base = Vector2::new(radius, 0.0);
        let p2_base = Vector2::new(-radius * 0.5, radius * 0.866);
        let p3_base = Vector2::new(-radius * 0.5, -radius * 0.866);

        let rotate = |p: Vector2| -> Vector2 {
            Vector2 {
                x: p.x * cos_a - p.y * sin_a + center_pos.x,
                y: p.x * sin_a + p.y * cos_a + center_pos.y,
            }
        };

        let p1 = rotate(p1_base);
        let p2 = rotate(p2_base);
        let p3 = rotate(p3_base);

        d.draw_triangle(p1, p3, p2, color);

        if with_outline {
            d.draw_triangle_lines(p1, p2, p3, Color::WHITE);
        }
    }

    // Draws all active projectiles within the arena area, using interpolation
    fn draw_projectiles(
        d: &mut RaylibDrawHandle,
        arena: &Arena,
        arena_screen_width: i32,
        arena_screen_height: i32,
        alpha: f64, // Interpolation factor
    ) {
        for projectile in &arena.projectiles {
            // Interpolate position
            let interp_pos =
                utils::lerp_point(projectile.prev_position, projectile.position, alpha);
            // Convert interpolated world position to screen position
            let screen_pos =
                Self::world_to_screen(interp_pos, arena_screen_width, arena_screen_height);
            d.draw_circle_v(screen_pos, 2.0, Color::WHITE);
        }
    }

    // Draws the active particles within the arena area, passing alpha for interpolation
    fn draw_particles(
        d: &mut RaylibDrawHandle,
        particle_system: &ParticleSystem,
        arena_screen_width: i32,
        arena_screen_height: i32,
        alpha: f32, // Interpolation factor
    ) {
        // Pass alpha to the particle system's draw method
        particle_system.draw(d, arena_screen_width, arena_screen_height, alpha);
    }

    // --- UI Panel Drawing ---
    fn draw_ui_panel(
        d: &mut RaylibDrawHandle,
        robots: &[Robot],
        current_turn: u32,
        max_turns: u32,
        current_cycle: u32,
        cycles_per_turn: u32,
    ) {
        let panel_x = ARENA_WIDTH; // Start UI panel after the arena
        let panel_width = UI_PANEL_WIDTH;

        // Constants for UI layout
        let padding = 12;
        let small_padding = 6;
        let xs_padding = 1;
        let card_padding = 10;
        let font_size_large = 18;
        let font_size_normal = 14;
        let font_size_small = 12;
        let bar_height = 8;
        let card_spacing = 16;
        let card_radius = 0.2;
        let bar_radius = 0.25;

        // Draw panel background with a dark color
        d.draw_rectangle(
            panel_x,
            0,
            panel_width,
            WINDOW_HEIGHT,
            Color::new(18, 18, 26, 255),
        );

        // Draw subtle grid pattern in the background for a more technical look
        let grid_size = 20;
        let grid_color = Color::new(30, 30, 40, 255);
        for i in 0..(WINDOW_HEIGHT / grid_size) {
            d.draw_line(
                panel_x,
                i * grid_size,
                panel_x + panel_width,
                i * grid_size,
                grid_color,
            );
        }
        for i in 0..(panel_width / grid_size) {
            d.draw_line(
                panel_x + i * grid_size,
                0,
                panel_x + i * grid_size,
                WINDOW_HEIGHT,
                grid_color,
            );
        }

        // Draw dividing line with gradient
        d.draw_line(
            panel_x,
            0,
            panel_x,
            WINDOW_HEIGHT,
            Color::new(60, 60, 80, 255),
        );
        d.draw_line(
            panel_x + 1,
            0,
            panel_x + 1,
            WINDOW_HEIGHT,
            Color::new(40, 40, 60, 255),
        );

        let mut current_y = padding;

        // --- Header Section with title ---
        let title_bg_rect = Rectangle::new(
            (panel_x + padding) as f32,
            current_y as f32,
            (panel_width - padding * 2) as f32,
            40.0,
        );
        d.draw_rectangle_rounded(title_bg_rect, 0.2, 6, Color::new(30, 30, 45, 255));

        // Draw App Title
        d.draw_text(
            "BOT ARENA",
            panel_x + panel_width / 2 - d.measure_text("BOT ARENA", font_size_large + 2) / 2,
            current_y + 10,
            font_size_large + 2,
            Color::WHITE,
        );

        current_y += 50;

        // --- Turn/Cycle Info with improved styling ---
        let status_rect = Rectangle::new(
            (panel_x + padding) as f32,
            current_y as f32,
            (panel_width - padding * 2) as f32,
            60.0,
        );
        d.draw_rectangle_rounded(status_rect, 0.2, 6, Color::new(30, 30, 45, 255));

        // Turn counter with visual indicator
        let turn_text = format!("TURN: {} / {}", current_turn, max_turns);
        let turn_ratio = current_turn as f32 / max_turns as f32;
        d.draw_text(
            &turn_text,
            panel_x + panel_width / 2 - d.measure_text(&turn_text, font_size_small) / 2,
            current_y + 8,
            font_size_small,
            Color::WHITE,
        );

        // Progress bar for turn
        let turn_bar_width = panel_width - padding * 4;
        d.draw_rectangle_rounded(
            Rectangle::new(
                (panel_x + padding * 2) as f32,
                (current_y + 20) as f32,
                turn_bar_width as f32,
                8.0,
            ),
            bar_radius,
            4,
            Color::new(40, 40, 60, 255),
        );

        if turn_ratio > 0.0 {
            d.draw_rectangle_rounded(
                Rectangle::new(
                    (panel_x + padding * 2) as f32,
                    (current_y + 20) as f32,
                    turn_bar_width as f32 * turn_ratio,
                    8.0,
                ),
                bar_radius,
                4,
                Color::new(120, 140, 240, 255),
            );
        }

        current_y += 25;

        let cycle_text = format!("CYCLE: {} / {}", current_cycle, cycles_per_turn);
        let cycle_ratio = current_cycle as f32 / cycles_per_turn as f32;
        d.draw_text(
            &cycle_text,
            panel_x + panel_width / 2 - d.measure_text(&cycle_text, font_size_small) / 2,
            current_y + 8,
            font_size_small,
            Color::WHITE,
        );

        // Progress bar for cycle
        let cycle_bar_width = panel_width - padding * 4;
        d.draw_rectangle_rounded(
            Rectangle::new(
                (panel_x + padding * 2) as f32,
                (current_y + 20) as f32,
                cycle_bar_width as f32,
                8.0,
            ),
            bar_radius,
            4,
            Color::new(40, 40, 60, 255),
        );

        if cycle_ratio > 0.0 {
            d.draw_rectangle_rounded(
                Rectangle::new(
                    (panel_x + padding * 2) as f32,
                    (current_y + 20) as f32,
                    cycle_bar_width as f32 * cycle_ratio,
                    8.0,
                ),
                bar_radius,
                4,
                Color::new(120, 140, 240, 255),
            );
        }

        current_y += 40;

        // --- Robot Info Cards ---
        for robot in robots {
            let card_height = 130; // Fixed card height

            // Select robot color based on ID
            let robot_color = match robot.id {
                1 => Color::new(70, 130, 230, 255),  // Blue
                2 => Color::new(230, 70, 70, 255),   // Red
                3 => Color::new(70, 190, 70, 255),   // Green
                4 => Color::new(230, 200, 30, 255),  // Yellow
                _ => Color::new(190, 100, 190, 255), // Purple
            };

            // Card background
            let card_bg_rect = Rectangle::new(
                (panel_x + padding) as f32,
                current_y as f32,
                (panel_width - padding * 2) as f32,
                card_height as f32,
            );
            d.draw_rectangle_rounded(card_bg_rect, card_radius, 6, Color::new(30, 30, 45, 255));

            // Card header with robot ID and status
            let header_height = 20;

            // Draw header with robot color
            d.draw_rectangle(
                panel_x + padding,
                current_y,
                panel_width - padding * 2,
                header_height,
                robot_color,
            );

            // Draw rounded corners for the top of the header
            // We can't use rounded corners for specific sides, so we'll just draw over the sharp corners
            // with small circles at the top corners
            let corner_radius = 6.0;
            d.draw_circle(
                panel_x + padding + corner_radius as i32,
                current_y + corner_radius as i32,
                corner_radius,
                robot_color,
            );
            d.draw_circle(
                panel_x + panel_width - padding - corner_radius as i32,
                current_y + corner_radius as i32,
                corner_radius,
                robot_color,
            );

            // Robot status indicator
            let status_color = match robot.status {
                RobotStatus::Active => Color::GREEN,
                RobotStatus::Stunned(_) => Color::YELLOW,
                RobotStatus::Destroyed => Color::RED,
                _ => Color::DARKGRAY,
            };

            let name_text = format!("ROBOT {}", robot.id);
            d.draw_text(
                &name_text,
                panel_x + padding + card_padding,
                current_y + 4,
                font_size_normal,
                Color::WHITE,
            );

            // Status indicator
            let status_text = format!("{:?}", robot.status);
            d.draw_circle(
                panel_x + panel_width - padding - card_padding - 10,
                current_y + header_height / 2,
                5.0,
                status_color,
            );
            d.draw_text(
                &status_text,
                panel_x + panel_width
                    - padding
                    - card_padding
                    - 20
                    - d.measure_text(&status_text, font_size_normal),
                current_y + 4,
                font_size_normal,
                Color::WHITE,
            );

            // Content area starts after header
            let content_y = current_y + header_height + small_padding;
            let content_width = panel_width - padding * 2 - card_padding * 2;

            // --- Health Bar ---
            let label_x = panel_x + padding + card_padding;
            let bar_x = label_x;
            let bar_width = content_width;

            // Draw health label and value on the same line
            d.draw_text(
                "HEALTH",
                label_x,
                content_y,
                font_size_small,
                Color::LIGHTGRAY,
            );

            let health_value_text = format!("{:.1}", robot.health);
            d.draw_text(
                &health_value_text,
                panel_x + panel_width
                    - padding
                    - card_padding
                    - d.measure_text(&health_value_text, font_size_small),
                content_y,
                font_size_small,
                Color::WHITE,
            );

            // Health bar (below text)
            let health_percentage = (robot.health / 100.0).clamp(0.0, 1.0);
            let health_bar_fill_width = (bar_width as f64 * health_percentage) as i32;

            // Background
            d.draw_rectangle_rounded(
                Rectangle::new(
                    bar_x as f32,
                    (content_y + font_size_small + xs_padding) as f32,
                    bar_width as f32,
                    bar_height as f32,
                ),
                bar_radius,
                4,
                Color::new(40, 90, 40, 100),
            );

            // Foreground (if health > 0)
            if health_bar_fill_width > 0 {
                d.draw_rectangle_rounded(
                    Rectangle::new(
                        bar_x as f32,
                        (content_y + font_size_small + xs_padding) as f32,
                        health_bar_fill_width as f32,
                        bar_height as f32,
                    ),
                    bar_radius,
                    4,
                    Color::new(40, 180, 40, 255),
                );
            }

            // --- Power Bar ---
            let power_y = content_y + font_size_small + xs_padding + bar_height + 14;

            // Draw power label and value on the same line
            d.draw_text("POWER", label_x, power_y, font_size_small, Color::LIGHTGRAY);

            let power_value_text = format!("{:.2}", robot.power);
            d.draw_text(
                &power_value_text,
                panel_x + panel_width
                    - padding
                    - card_padding
                    - d.measure_text(&power_value_text, font_size_small),
                power_y,
                font_size_small,
                Color::WHITE,
            );

            // Power bar (below text)
            let power_percentage = robot.power.clamp(0.0, 1.0);
            let power_bar_fill_width = (bar_width as f64 * power_percentage) as i32;

            // Background
            d.draw_rectangle_rounded(
                Rectangle::new(
                    bar_x as f32,
                    (power_y + font_size_small + xs_padding) as f32,
                    bar_width as f32,
                    bar_height as f32,
                ),
                bar_radius,
                4,
                Color::new(40, 40, 90, 100),
            );

            // Foreground (if power > 0)
            if power_bar_fill_width > 0 {
                d.draw_rectangle_rounded(
                    Rectangle::new(
                        bar_x as f32,
                        (power_y + font_size_small + xs_padding) as f32,
                        power_bar_fill_width as f32,
                        bar_height as f32,
                    ),
                    bar_radius,
                    4,
                    Color::new(60, 120, 230, 255),
                );
            }

            // --- Instruction Info ---
            let instr_y = power_y + font_size_small + xs_padding + bar_height + 14;

            // Draw IP label and value on the same line
            d.draw_text("IP", label_x, instr_y, font_size_small, Color::LIGHTGRAY);

            let ip_text = format!("{}", robot.vm_state.ip);
            d.draw_text(
                &ip_text,
                panel_x + panel_width
                    - padding
                    - card_padding
                    - d.measure_text(&ip_text, font_size_small),
                instr_y,
                font_size_small,
                Color::new(240, 200, 60, 255), // Gold color
            );

            // Instruction
            let instr_display_y = instr_y + font_size_small + xs_padding;
            let instruction_text = robot.get_current_instruction_string();

            // Instruction background
            d.draw_rectangle(
                label_x,
                instr_display_y,
                content_width,
                font_size_small + xs_padding * 2,
                Color::new(30, 30, 35, 255),
            );

            // Instruction text
            d.draw_text(
                &instruction_text,
                label_x + xs_padding,
                instr_display_y + xs_padding,
                font_size_small,
                Color::WHITE,
            );

            // Move to next card position
            current_y += card_height + card_spacing;
        }
    }

    // Optional: Draws the grid lines for debugging within the arena area
    #[allow(dead_code)]
    fn draw_grid(
        d: &mut RaylibDrawHandle,
        arena: &Arena,
        arena_screen_width: i32,
        arena_screen_height: i32,
    ) {
        let screen_width_f = arena_screen_width as f32;
        let screen_height_f = arena_screen_height as f32;
        let unit_screen_width = screen_width_f / arena.grid_width as f32;
        let unit_screen_height = screen_height_f / arena.grid_height as f32;

        for i in 1..arena.grid_width {
            d.draw_line(
                (i as f32 * unit_screen_width) as i32,
                0,
                (i as f32 * unit_screen_width) as i32,
                arena_screen_height,
                Color::DARKGREEN,
            );
        }
        for i in 1..arena.grid_height {
            d.draw_line(
                0,
                (i as f32 * unit_screen_height) as i32,
                arena_screen_width,
                (i as f32 * unit_screen_height) as i32,
                Color::DARKGREEN,
            );
        }
    }

    // Get the time elapsed since the last frame
    pub fn get_frame_time(&self) -> f32 {
        self.rl.get_frame_time()
    }

    pub fn draw_announcement(d: &mut RaylibDrawHandle, message: &str) {
        let overlay_color = Color::new(0, 0, 0, 180);
        let rect_width = 500;
        let rect_height = 120;
        let x = (WINDOW_WIDTH / 2) - (rect_width / 2);
        let y = (WINDOW_HEIGHT / 2) - (rect_height / 2);
        d.draw_rectangle(x, y, rect_width, rect_height, overlay_color);
        let font_size = 36;
        let text_width = d.measure_text(message, font_size);
        let text_x = x + (rect_width - text_width) / 2;
        let text_y = y + (rect_height - font_size) / 2;
        d.draw_text(message, text_x, text_y, font_size, Color::WHITE);
        let hint = "Press ESC to exit";
        let hint_size = 20;
        let hint_width = d.measure_text(hint, hint_size);
        let hint_x = x + (rect_width - hint_width) / 2;
        d.draw_text(hint, hint_x, y + rect_height - hint_size - 10, hint_size, Color::LIGHTGRAY);
    }

    pub fn is_key_down(&self, key: KeyboardKey) -> bool {
        self.rl.is_key_down(key)
    }
}

// Default implementation for Renderer
impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
