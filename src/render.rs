use crate::arena::*;
use crate::config::{ARENA_WIDTH, ARENA_HEIGHT, UNIT_SIZE, UI_PANEL_WIDTH, WINDOW_WIDTH, WINDOW_HEIGHT};
use crate::particles::ParticleSystem;
use crate::robot::Robot;
use crate::types::*;
use crate::utils;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams, TextureFormat, TextureParams, FilterMode};
use macroquad::prelude::*;

const BRIGHTNESS_THRESHOLD: f32 = 0.05;
const BLUR_PASSES: usize = 2; // Keep blur passes low for now
const GLOW_INTENSITY: f32 = 1.5; // Factor to multiply glow brightness

// Conversion helpers
fn point_to_vec2(p: Point, arena_screen_width: i32, arena_screen_height: i32) -> Vec2 {
    Vec2::new(
        (p.x * arena_screen_width as f64) as f32,
        (p.y * arena_screen_height as f64) as f32,
    )
}

fn color_from_rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba(r, g, b, a)
}

// Add a helper function at the top of the file
fn faded_color(mut color: Color, alpha: f32) -> Color {
    color.a *= alpha;
    color
}

// Add a helper to brighten a color
fn brighten_color(color: Color, amount: f32) -> Color {
    Color::new(
        (color.r + amount).min(1.0),
        (color.g + amount).min(1.0),
        (color.b + amount).min(1.0),
        color.a,
    )
}

// Handles rendering the simulation state using macroquad
pub struct Renderer {
    material: Option<Material>,
    scene_rt: Option<RenderTarget>,
    bright_rt: Option<RenderTarget>,
    blur_rt1: Option<RenderTarget>,
    blur_rt2: Option<RenderTarget>,
    brightness_material: Option<Material>,
    h_blur_material: Option<Material>,
    v_blur_material: Option<Material>,
    additive_material: Option<Material>, // Material for final additive blend
    scanner_material: Option<Material>, // Material for scanner visualization
    title_font: Option<Font>, // Font for UI title
    ui_font: Option<Font>, // Font for general UI elements
}

impl Renderer {
    pub fn new() -> Self {
        Renderer {
            material: None,
            scene_rt: None,
            bright_rt: None,
            blur_rt1: None,
            blur_rt2: None,
            brightness_material: None,
            h_blur_material: None,
            v_blur_material: None,
            additive_material: None,
            scanner_material: None,
            title_font: None, // Initialize title_font as None
            ui_font: None, // Initialize ui_font as None
        }
    }

    pub fn init_material(&mut self) {
        let material = load_material(
            ShaderSource::Glsl {
                vertex: "#version 100
attribute vec3 position;
attribute vec2 texcoord; // We don't use texcoord here, but need it for macroquad's default mesh
varying vec4 frag_color; // Pass color through
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    // Assign a default color or pass vertex color if available
    // Since we're tinting everything drawn with this shader,
    // the actual input color doesn't matter as much,
    // but let's just use white.
    frag_color = vec4(1.0, 1.0, 1.0, 1.0);
}",
                fragment: "#version 100
precision mediump float;
varying vec4 frag_color; // Receive color from vertex shader
void main() {
    // Apply red tint to the incoming fragment color
    gl_FragColor = frag_color * vec4(1.0, 0.3, 0.3, 1.0); // Stronger red tint
}",
            },
            // Note: No MaterialParams needed if not using textures/uniforms beyond default Model/Projection
             MaterialParams::default() // Use default params
        ).unwrap();
        self.material = Some(material);
    }

    pub fn init_scanner_material(&mut self) {
        let vertex_shader = "#version 100
            attribute vec3 position;
            attribute vec4 color0;
            varying lowp vec4 frag_color;
            uniform mat4 Model;
            uniform mat4 Projection;
            void main() {
                gl_Position = Projection * Model * vec4(position, 1.0);
                frag_color = color0 / 255.0;
            }";

        let fragment_shader = "#version 100
            precision lowp float;
            varying lowp vec4 frag_color;
            void main() {
                gl_FragColor = frag_color;
            }";

        let pipeline_params = PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::One, // Use premultiplied alpha source factor
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha), // Correct standard alpha destination factor
            )),
            ..Default::default()
        };

        self.scanner_material = Some(
            load_material(
                ShaderSource::Glsl {
                    vertex: vertex_shader,
                    fragment: fragment_shader,
                },
                MaterialParams {
                    pipeline_params,
                    ..Default::default()
                },
            )
            .expect("Failed to load scanner material"),
        );
    }

    // Load the custom title font
    pub async fn load_title_font(&mut self) {
        match load_ttf_font("assets/title.ttf").await {
            Ok(font) => self.title_font = Some(font),
            Err(e) => log::error!("Failed to load font assets/title.ttf: {}", e),
        }
    }

    // Load the custom UI font
    pub async fn load_ui_font(&mut self) {
        match load_ttf_font("assets/default.ttf").await {
            Ok(font) => self.ui_font = Some(font),
            Err(e) => log::error!("Failed to load UI font assets/default.ttf: {}", e),
        }
    }

    // Initialize materials and render targets for the glow effect
    pub fn init_glow_resources(&mut self) {
        // Use miniquad::render_target to create RenderTargets
        self.scene_rt = Some(render_target(ARENA_WIDTH as u32, ARENA_HEIGHT as u32));
        self.bright_rt = Some(render_target(ARENA_WIDTH as u32, ARENA_HEIGHT as u32));
        self.blur_rt1 = Some(render_target(ARENA_WIDTH as u32, ARENA_HEIGHT as u32));
        self.blur_rt2 = Some(render_target(ARENA_WIDTH as u32, ARENA_HEIGHT as u32));

        // Use imported miniquad types
        let _texture_params = TextureParams {
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        };
        // Set filter on the textures using the imported FilterMode
        self.scene_rt.as_mut().unwrap().texture.set_filter(FilterMode::Linear);
        self.bright_rt.as_mut().unwrap().texture.set_filter(FilterMode::Linear);
        self.blur_rt1.as_mut().unwrap().texture.set_filter(FilterMode::Linear);
        self.blur_rt2.as_mut().unwrap().texture.set_filter(FilterMode::Linear);

        let post_process_vertex_shader = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    uv = texcoord;
}";

        let brightness_fragment_shader = "#version 100
precision mediump float;
varying vec2 uv;
uniform sampler2D InputTexture;
uniform float Threshold;

void main() {
    vec4 color = texture2D(InputTexture, uv);
    float brightness = dot(color.rgb, vec3(0.2126, 0.7152, 0.0722));
    // Restore thresholding to get bright parts
    vec4 final_color = step(Threshold, brightness) * color;
    gl_FragColor = final_color;
}";

        // Simplified Box Blur for now (Gaussian is better but more complex)
        let blur_fragment_shader = "#version 100
precision mediump float;
varying vec2 uv;
uniform sampler2D InputTexture;
uniform vec2 BlurDir; // (1.0/texture_width, 0.0) or (0.0, 1.0/texture_height)

void main() {
    vec4 color = vec4(0.0);
    vec2 texel_size = vec2(BlurDir.x, BlurDir.y);
    color += texture2D(InputTexture, uv - 4.0 * texel_size) * 0.05;
    color += texture2D(InputTexture, uv - 3.0 * texel_size) * 0.09;
    color += texture2D(InputTexture, uv - 2.0 * texel_size) * 0.12;
    color += texture2D(InputTexture, uv - 1.0 * texel_size) * 0.15;
    color += texture2D(InputTexture, uv) * 0.18;
    color += texture2D(InputTexture, uv + 1.0 * texel_size) * 0.15;
    color += texture2D(InputTexture, uv + 2.0 * texel_size) * 0.12;
    color += texture2D(InputTexture, uv + 3.0 * texel_size) * 0.09;
    color += texture2D(InputTexture, uv + 4.0 * texel_size) * 0.05;
    gl_FragColor = color;
}";

        let brightness_params = MaterialParams {
            textures: vec!["InputTexture".to_string()],
            uniforms: vec![UniformDesc::new("Threshold", UniformType::Float1)],
            pipeline_params: PipelineParams {
                color_blend: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let h_blur_params = MaterialParams {
            textures: vec!["InputTexture".to_string()],
            uniforms: vec![UniformDesc::new("BlurDir", UniformType::Float2)],
            pipeline_params: PipelineParams { color_blend: None, ..Default::default() },
            ..Default::default()
        };
        let v_blur_params = MaterialParams {
            textures: vec!["InputTexture".to_string()],
            uniforms: vec![UniformDesc::new("BlurDir", UniformType::Float2)],
            pipeline_params: PipelineParams { color_blend: None, ..Default::default() },
            ..Default::default()
        };

        self.brightness_material = Some(load_material(
            ShaderSource::Glsl {
                vertex: post_process_vertex_shader,
                fragment: brightness_fragment_shader,
            },
            brightness_params,
        ).unwrap());

        // Horizontal Blur Material
        self.h_blur_material = Some(load_material(
            ShaderSource::Glsl {
                vertex: post_process_vertex_shader,
                fragment: &blur_fragment_shader,
            },
            h_blur_params,
        ).unwrap());

        // Vertical Blur Material
        self.v_blur_material = Some(load_material(
            ShaderSource::Glsl {
                vertex: post_process_vertex_shader,
                fragment: &blur_fragment_shader,
            },
            v_blur_params,
        ).unwrap());

        // Define minimal passthrough fragment shader WITH intensity uniform
        let passthrough_fragment_shader = "#version 100
precision mediump float;
varying vec2 uv;
uniform sampler2D InputTexture;
uniform float GlowIntensity; // Add intensity uniform
void main() {
    vec4 glow_color = texture2D(InputTexture, uv);
    gl_FragColor = glow_color * GlowIntensity; // Multiply by intensity
}";

        // Create material for additive blending step
        let additive_blend_state = BlendState::new(
            Equation::Add,
            BlendFactor::One,
            BlendFactor::One
        );
        let additive_pipeline_params = PipelineParams {
            color_blend: Some(additive_blend_state),
            ..Default::default()
        };
        self.additive_material = Some(load_material(
            ShaderSource::Glsl { // Use GLSL source
                vertex: post_process_vertex_shader, // Can reuse the same vertex shader
                fragment: passthrough_fragment_shader, // Use minimal fragment shader
            },
            MaterialParams {
                textures: vec!["InputTexture".to_string()], // Still needs texture input
                uniforms: vec![UniformDesc::new("GlowIntensity", UniformType::Float1)], // Add uniform desc
                pipeline_params: additive_pipeline_params,
                ..Default::default()
            }
        ).unwrap());
    }

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
        // --- Bypass Glow Effect - Draw directly to screen --- 
        set_default_camera(); // Ensure we are drawing to the screen
        clear_background(BLACK); // Clear the main screen

        let alpha = (time_accumulator / cycle_duration).clamp(0.0, 1.0);
        // Draw arena elements directly
        Self::draw_arena_boundaries(arena, ARENA_WIDTH, ARENA_HEIGHT);
        Self::draw_obstacles(arena, ARENA_WIDTH, ARENA_HEIGHT);
        for robot in robots {
            // Note: draw_robot now needs &mut self if we were to use materials internally
            // Since we are calling it on Self, we pass self implicitly.
            // If draw_robot was not part of Renderer impl, we would need &mut renderer.
            self.draw_robot(robot, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64);
        }
        Self::draw_projectiles(arena, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64);
        Self::draw_particles(particle_system, ARENA_WIDTH, ARENA_HEIGHT, alpha);

        /* --- Glow Effect Code - Temporarily Disabled --- 
        // Ensure all RTs and materials are initialized (should be done in main, but double-check)
        if self.scene_rt.is_none() {
             self.init_glow_resources();
        }

        // --- Pass 1: Draw Scene to Render Target --- 
        let scene_rt = self.scene_rt.as_ref().unwrap();
        set_camera(&Camera2D {
            render_target: Some(scene_rt.clone()),
            zoom: vec2(1.0 / ARENA_WIDTH as f32 * 2.0, 1.0 / ARENA_HEIGHT as f32 * 2.0),
            target: vec2(ARENA_WIDTH as f32 / 2.0, ARENA_HEIGHT as f32 / 2.0),
            ..Default::default()
        });
        clear_background(BLACK); // Clear the scene RT

        let alpha = (time_accumulator / cycle_duration).clamp(0.0, 1.0);
        // Draw arena elements normally (no special material here)
        Self::draw_arena_boundaries(arena, ARENA_WIDTH, ARENA_HEIGHT);
        Self::draw_obstacles(arena, ARENA_WIDTH, ARENA_HEIGHT);
        for robot in robots {
            self.draw_robot(robot, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64); 
        }
        Self::draw_projectiles(arena, ARENA_WIDTH, ARENA_HEIGHT, alpha as f64);
        Self::draw_particles(particle_system, ARENA_WIDTH, ARENA_HEIGHT, alpha);

        set_default_camera(); // Reset camera after drawing to RT

        // --- Pass 2: Extract Bright Pixels --- 
        let bright_rt = self.bright_rt.as_ref().unwrap();
        let scene_texture = &self.scene_rt.as_ref().unwrap().texture;
        let brightness_material = self.brightness_material.as_ref().unwrap();
        brightness_material.set_uniform("Threshold", BRIGHTNESS_THRESHOLD);
        brightness_material.set_texture("InputTexture", scene_texture.clone());

        set_camera(&Camera2D {
            render_target: Some(bright_rt.clone()),
            zoom: vec2(1.0 / ARENA_WIDTH as f32 * 2.0, 1.0 / ARENA_HEIGHT as f32 * 2.0),
            target: vec2(ARENA_WIDTH as f32 / 2.0, ARENA_HEIGHT as f32 / 2.0),
            ..Default::default()
        });
        clear_background(BLACK);
        gl_use_material(brightness_material);
        draw_texture_ex(scene_texture, 0.0, 0.0, WHITE, DrawTextureParams { ..Default::default() });
        gl_use_default_material();
        set_default_camera();

        // --- Pass 3: Blur Bright Pixels (Ping-Pong) --- 
        let h_blur_material = self.h_blur_material.as_ref().unwrap();
        let v_blur_material = self.v_blur_material.as_ref().unwrap();
        let blur_rt1 = self.blur_rt1.as_ref().unwrap();
        let blur_rt2 = self.blur_rt2.as_ref().unwrap();

        let blur_dir_h = vec2(1.0 / ARENA_WIDTH as f32, 0.0);
        let blur_dir_v = vec2(0.0, 1.0 / ARENA_HEIGHT as f32);

        let mut current_source_rt = bright_rt; // Start with the bright pass result
        let mut current_target_rt = blur_rt1;
        let mut next_target_rt = blur_rt2;

        for _i in 0..BLUR_PASSES {
            // --- Horizontal Blur --- 
            let source_texture_h = &current_source_rt.texture;
            set_camera(&Camera2D { 
                render_target: Some(current_target_rt.clone()),
                zoom: vec2(1.0 / ARENA_WIDTH as f32 * 2.0, 1.0 / ARENA_HEIGHT as f32 * 2.0),
                target: vec2(ARENA_WIDTH as f32 / 2.0, ARENA_HEIGHT as f32 / 2.0),
                ..Default::default() 
            });
            clear_background(BLACK);
            h_blur_material.set_texture("InputTexture", source_texture_h.clone());
            h_blur_material.set_uniform("BlurDir", blur_dir_h);
            gl_use_material(h_blur_material);
            draw_rectangle(0.0, 0.0, ARENA_WIDTH as f32, ARENA_HEIGHT as f32, WHITE);
            gl_use_default_material();
            set_default_camera();
            // Swap textures for next pass
            std::mem::swap(&mut current_source_rt, &mut current_target_rt);
            std::mem::swap(&mut current_target_rt, &mut next_target_rt);

            // --- Vertical Blur --- 
            let source_texture_v = &current_source_rt.texture;
            set_camera(&Camera2D { 
                render_target: Some(current_target_rt.clone()),
                zoom: vec2(1.0 / ARENA_WIDTH as f32 * 2.0, 1.0 / ARENA_HEIGHT as f32 * 2.0),
                target: vec2(ARENA_WIDTH as f32 / 2.0, ARENA_HEIGHT as f32 / 2.0),
                 ..Default::default() 
            });
            clear_background(BLACK);
            v_blur_material.set_texture("InputTexture", source_texture_v.clone());
            v_blur_material.set_uniform("BlurDir", blur_dir_v);
            gl_use_material(v_blur_material);
            draw_rectangle(0.0, 0.0, ARENA_WIDTH as f32, ARENA_HEIGHT as f32, WHITE);
            gl_use_default_material();
            set_default_camera();
            // Swap textures for next pass (or final result)
            std::mem::swap(&mut current_source_rt, &mut current_target_rt);
            std::mem::swap(&mut current_target_rt, &mut next_target_rt);
        }
        // After the loop, current_source_rt holds the final blurred texture
        let final_glow_rt = current_source_rt;

        // --- Final Composite: Draw Scene + Additive Glow to Screen --- 
        clear_background(BLACK); // Clear the main screen

        // 1. Draw the original scene - NO flip needed now
        draw_texture_ex(&scene_rt.texture, 0.0, 0.0, WHITE, DrawTextureParams { ..Default::default() });

        // 2. Draw the final blurred glow texture using the additive material and draw_rectangle
        let additive_material = self.additive_material.as_ref().unwrap();
        let glow_texture = &final_glow_rt.texture;
        additive_material.set_texture("InputTexture", glow_texture.clone()); // Bind glow tex to material
        additive_material.set_uniform("GlowIntensity", GLOW_INTENSITY); // Set intensity
        gl_use_material(additive_material); // This applies the additive blend pipeline
        // Draw rectangle, the material's passthrough shader will sample the glow texture
        draw_rectangle(0.0, 0.0, ARENA_WIDTH as f32, ARENA_HEIGHT as f32, WHITE);
        gl_use_default_material(); // Reset to default material/pipeline
        */

        // --- Draw UI (unaffected by glow) --- 
        self.draw_ui_panel(
            robots,
            current_turn,
            max_turns,
            current_cycle,
            cycles_per_turn,
        );
        // Draw FPS counter using UI font
        let fps_text = format!("FPS: {}", get_fps());
        let fps_params = TextParams {
            font: self.ui_font.as_ref(),
            font_size: 18,
            color: WHITE,
            ..Default::default()
        };
        draw_text_ex(&fps_text, 10.0, 20.0, fps_params); // Adjusted Y position slightly for clarity
        
        if let Some(msg) = announcement {
            self.draw_announcement(msg);
        }
    }

    fn draw_arena_boundaries(_arena: &Arena, arena_screen_width: i32, arena_screen_height: i32) {
        draw_rectangle_lines(
            1.0,
            1.0,
            (arena_screen_width - 2) as f32,
            (arena_screen_height - 2) as f32,
            2.0,
            GRAY,
        );
    }

    fn draw_obstacles(arena: &Arena, arena_screen_width: i32, arena_screen_height: i32) {
        let obstacle_screen_size = (arena.unit_size * arena_screen_width.min(arena_screen_height) as f64) as f32;
        let half_size = obstacle_screen_size / 2.0;
        for obstacle in &arena.obstacles {
            let screen_pos = point_to_vec2(obstacle.position, arena_screen_width, arena_screen_height);
            draw_rectangle(
                screen_pos.x - half_size,
                screen_pos.y - half_size,
                obstacle_screen_size,
                obstacle_screen_size,
                DARKGRAY,
            );
        }
    }

    fn draw_robot(&self, robot: &Robot, arena_screen_width: i32, arena_screen_height: i32, alpha: f64) {
        let robot_screen_size = (UNIT_SIZE * arena_screen_width.min(arena_screen_height) as f64) as f32;
        let radius = robot_screen_size / 2.0;
        // Interpolate state
        let interp_pos = utils::lerp_point(robot.prev_position, robot.position, alpha);
        let interp_drive_deg = utils::angle_lerp(robot.prev_drive_direction, robot.drive.direction, alpha);
        let interp_turret_deg = utils::angle_lerp(robot.prev_turret_direction, robot.turret.direction, alpha);
        let center_pos = point_to_vec2(interp_pos, arena_screen_width, arena_screen_height);
        // Use the same color logic as the UI card
        let body_color = match robot.id {
            1 => Color::from_rgba(40, 80, 140, 255),
            2 => Color::from_rgba(140, 40, 40, 255),
            3 => Color::from_rgba(40, 100, 40, 255),
            4 => Color::from_rgba(140, 120, 20, 255),
            _ => Color::from_rgba(100, 50, 100, 255),
        };
        let body_outline_color = brighten_color(body_color, 0.5);
        // Compute target directions
        let target_drive_deg = (robot.drive.direction + robot.drive.pending_rotation).rem_euclid(360.0) as f32;
        let target_turret_deg = (robot.turret.direction + robot.turret.pending_rotation).rem_euclid(360.0) as f32;
        // Define ghost colors and thickness
        let ghost_fill_color = faded_color(DARKGRAY, 0.2); // Adjusted background alpha
        let ghost_outline_color = brighten_color(DARKGRAY, 0.2); // Darker outline based on DARKGRAY
        let ghost_line_thickness = 0.5; // Thinner outline
        // Draw ghost (target) drive direction (always draw)
        Self::draw_triangle_at_angle(center_pos, radius * 2.0, target_drive_deg, ghost_fill_color, ghost_outline_color, false, ghost_line_thickness, false, BLACK);
        // Draw ghost (target) turret direction (always draw)
        let turret_rad = target_turret_deg.to_radians();
        let turret_end = center_pos + Vec2::new(turret_rad.cos(), turret_rad.sin()) * radius * 2.0 * 0.8;
        draw_line(center_pos.x, center_pos.y, turret_end.x, turret_end.y, 2.0, ghost_fill_color); // Use brighter background color
        // Draw robot body as triangle (interpolated)
        Self::draw_triangle_at_angle(center_pos, radius, interp_drive_deg as f32, faded_color(body_color, 1.0), body_outline_color, true, 1.0, true, WHITE);
        // Draw turret as a line (interpolated)
        let turret_rad = interp_turret_deg.to_radians() as f32;
        let turret_end = center_pos + Vec2::new(turret_rad.cos(), turret_rad.sin()) * radius * 0.8;
        draw_line(center_pos.x, center_pos.y, turret_end.x, turret_end.y, 2.0, faded_color(LIGHTGRAY, 1.0));

        // Draw scanner area using a custom mesh and material
        if let Some(scanner_material) = &self.scanner_material {
            let scanner_range = (robot.turret.scanner.range * arena_screen_width.min(arena_screen_height) as f64) as f32;
            let scanner_fov_deg = robot.turret.scanner.fov as f32;
            let start_angle_deg = interp_turret_deg as f32 - scanner_fov_deg / 2.0;
            let base_scanner_color = faded_color(body_color, 0.15); // Set alpha to 0.15

            // Premultiply color for the chosen blend mode
            let scanner_color = Color::new(
                base_scanner_color.r * base_scanner_color.a,
                base_scanner_color.g * base_scanner_color.a,
                base_scanner_color.b * base_scanner_color.a,
                base_scanner_color.a,
            );

            let num_segments = 20; // Number of segments for the arc
            let mut vertices: Vec<Vertex> = Vec::with_capacity(num_segments + 2);
            let mut indices: Vec<u16> = Vec::with_capacity(num_segments * 3);

            // Center vertex
            vertices.push(Vertex::new(center_pos.x, center_pos.y, 0.0, 0.0, 0.0, scanner_color));

            // Arc vertices
            for i in 0..=num_segments {
                let t = i as f32 / num_segments as f32;
                let angle_deg = start_angle_deg + t * scanner_fov_deg;
                let angle_rad = angle_deg.to_radians();
                let point_on_arc = center_pos + Vec2::new(angle_rad.cos(), angle_rad.sin()) * scanner_range;
                vertices.push(Vertex::new(point_on_arc.x, point_on_arc.y, 0.0, 0.0, 0.0, scanner_color));
            }

            // Triangle fan indices (0, 1, 2), (0, 2, 3), ...
            for i in 1..=num_segments {
                indices.push(0); // Center vertex
                indices.push(i as u16);
                indices.push(i as u16 + 1);
            }

            let mesh = Mesh {
                vertices,
                indices,
                texture: None, // No texture needed
            };

            gl_use_material(scanner_material); // Use custom blend mode material
            draw_mesh(&mesh); // Draw the custom mesh (Correct function name)
            gl_use_default_material(); // Switch back to default material/blend state
        }
    }

    fn draw_triangle_at_angle(
        center_pos: Vec2,
        radius: f32,
        angle_deg: f32,
        color: Color,
        outline_color: Color,
        with_outline: bool,
        line_thickness: f32,
        draw_tip_indicator: bool,
        indicator_color: Color
    ) {
        let angle_rad = angle_deg.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let p1_base = Vec2::new(radius, 0.0);
        let p2_base = Vec2::new(-radius * 0.5, radius * 0.866);
        let p3_base = Vec2::new(-radius * 0.5, -radius * 0.866);
        let rotate = |p: Vec2| -> Vec2 {
            Vec2::new(
                p.x * cos_a - p.y * sin_a + center_pos.x,
                p.x * sin_a + p.y * cos_a + center_pos.y,
            )
        };
        let p1 = rotate(p1_base);
        let p2 = rotate(p2_base);
        let p3 = rotate(p3_base);
        draw_triangle(p1, p2, p3, color);

        if draw_tip_indicator {
            const INDICATOR_SIZE_FRACTION: f32 = 0.25;
            let indicator_p2 = p1.lerp(p2, INDICATOR_SIZE_FRACTION);
            let indicator_p3 = p1.lerp(p3, INDICATOR_SIZE_FRACTION);
            draw_triangle(p1, indicator_p2, indicator_p3, indicator_color);
        }

        if with_outline {
            draw_line(p1.x, p1.y, p2.x, p2.y, line_thickness, outline_color);
            draw_line(p2.x, p2.y, p3.x, p3.y, line_thickness, outline_color);
            draw_line(p3.x, p3.y, p1.x, p1.y, line_thickness, outline_color);
        }
    }

    fn draw_projectiles(arena: &Arena, arena_screen_width: i32, arena_screen_height: i32, alpha: f64) {
        for projectile in &arena.projectiles {
            let interp_pos = utils::lerp_point(projectile.prev_position, projectile.position, alpha);
            let screen_pos = point_to_vec2(interp_pos, arena_screen_width, arena_screen_height);
            draw_circle(screen_pos.x, screen_pos.y, 2.0, WHITE);
        }
    }

    fn draw_particles(particle_system: &ParticleSystem, arena_screen_width: i32, arena_screen_height: i32, alpha: f32) {
        for particle in &particle_system.particles {
            let interp_x = utils::lerp(particle.prev_position.x, particle.position.x, alpha);
            let interp_y = utils::lerp(particle.prev_position.y, particle.position.y, alpha);
            let screen_pos = point_to_vec2(crate::types::Point { x: interp_x as f64, y: interp_y as f64 }, arena_screen_width, arena_screen_height);
            let color = Color::from_rgba(
                (particle.color.r * 255.0) as u8,
                (particle.color.g * 255.0) as u8,
                (particle.color.b * 255.0) as u8,
                (particle.color.a * 255.0) as u8,
            );
            draw_circle(screen_pos.x, screen_pos.y, 2.0, color);
        }
    }

    fn draw_ui_panel(&self, robots: &[Robot], current_turn: u32, max_turns: u32, current_cycle: u32, cycles_per_turn: u32) {
        let panel_x = ARENA_WIDTH as f32;
        let panel_width = UI_PANEL_WIDTH as f32;
        let padding = 10.0; // General padding for horizontal spacing and between elements
        let top_margin = 16.0; // Specific margin for the top
        let font_size = 22.0; // Base size for title (unused directly here)
        let small_font_size = 14.0; // Reduced size for UI elements
        let mut y = top_margin;

        // Keep default font for most things
        let default_params = TextParams {
            font_size: font_size as u16,
            color: WHITE,
            ..Default::default()
        };
        let small_params = TextParams {
            font_size: small_font_size as u16,
            font: self.ui_font.as_ref(), // Use UI font
            ..default_params
        };
        let small_gray_params = TextParams {
            font: self.ui_font.as_ref(), // Use UI font
            color: LIGHTGRAY,
            ..small_params
        };
        let small_white_params = TextParams {
            font: self.ui_font.as_ref(), // Use UI font
            color: WHITE,
            ..small_params
        };
        let small_value_params = TextParams {
            font_size: (small_font_size - 2.0) as u16,
            font: self.ui_font.as_ref(), // Use UI font
            color: WHITE,
            ..small_params
        };

        // Panel drop shadow
        draw_rectangle(panel_x + 6.0, 8.0, panel_width, WINDOW_HEIGHT as f32 - 16.0, Color::from_rgba(0, 0, 0, 60));
        // Panel background
        draw_rectangle(panel_x, 0.0, panel_width, WINDOW_HEIGHT as f32, faded_color(Color::from_rgba(32, 36, 48, 255), 1.0));
        
        // Title - Use custom font here only
        let title_params = TextParams {
            font: self.title_font.as_ref(), // Use custom font
            font_size: font_size as u16,
            color: GOLD,
            ..Default::default()
        };
        draw_text_ex("BOT ARENA", panel_x + padding, y + 12.0, title_params); // Use title_params + 5.0px offset
        y += font_size + padding * 0.5;
        
        // Turn meter - Use default font params
        let bar_x = panel_x + padding;
        let bar_width = panel_width - 2.0 * padding;
        let bar_height = 12.0;
        let turn_ratio = current_turn as f32 / max_turns as f32;
        draw_text_ex("TURN", bar_x, y, small_white_params.clone());
        let turn_label_y = y;
        let turn_bar_y = turn_label_y + small_font_size - 14.0 + 1.0;
        draw_rectangle(bar_x, turn_bar_y, bar_width, bar_height, Color::from_rgba(44, 48, 60, 255));
        draw_rectangle(bar_x, turn_bar_y, bar_width * turn_ratio, bar_height, GREEN);
        let turn_text = format!("{}/{}", current_turn, max_turns);
        let turn_text_dims = measure_text(&turn_text, self.ui_font.as_ref(), small_value_params.font_size, 1.0);
        draw_text_ex(&turn_text, bar_x + bar_width - turn_text_dims.width - 2.0, turn_bar_y + bar_height * 0.7 + 1.0, small_value_params.clone());
        y = turn_bar_y + bar_height + 8.0;
        // Cycle meter - Use default font params
        let cycle_ratio = current_cycle as f32 / cycles_per_turn as f32;
        let cycle_label_y = y + 4.0;
        draw_text_ex("CYCLE", bar_x, cycle_label_y, small_white_params.clone());
        let cycle_bar_y = cycle_label_y + small_font_size - 14.0 + 2.0;
        draw_rectangle(bar_x, cycle_bar_y, bar_width, bar_height, Color::from_rgba(44, 48, 60, 255));
        draw_rectangle(bar_x, cycle_bar_y, bar_width * cycle_ratio, bar_height, SKYBLUE);
        let cycle_text = format!("{}/{}", current_cycle, cycles_per_turn);
        let cycle_text_dims = measure_text(&cycle_text, self.ui_font.as_ref(), small_value_params.font_size, 1.0);
        draw_text_ex(&cycle_text, bar_x + bar_width - cycle_text_dims.width - 2.0, cycle_bar_y + bar_height * 0.7 + 1.0, small_value_params.clone());
        y = cycle_bar_y + bar_height + padding * 0.5;
        // Robot cards - Use default font params
        let card_height = 124.0;
        let card_spacing = padding; // Use general padding for card spacing
        for robot in robots {
            let card_y = y;
            let robot_color = match robot.id {
                1 => faded_color(Color::from_rgba(40, 80, 140, 255), 1.0),
                2 => faded_color(Color::from_rgba(140, 40, 40, 255), 1.0),
                3 => faded_color(Color::from_rgba(40, 100, 40, 255), 1.0),
                4 => faded_color(Color::from_rgba(140, 120, 20, 255), 1.0),
                _ => faded_color(Color::from_rgba(100, 50, 100, 255), 1.0),
            };
            // Card drop shadow
            draw_rectangle(panel_x + padding + 3.0, card_y + 3.0, panel_width - 2.0 * padding, card_height, Color::from_rgba(0, 0, 0, 40));
            // Card background
            draw_rectangle(panel_x + padding, card_y, panel_width - 2.0 * padding, card_height, faded_color(Color::from_rgba(40, 44, 60, 255), 1.0));
            draw_rectangle_lines(panel_x + padding, card_y, panel_width - 2.0 * padding, card_height, 2.0, robot_color);
            // Header bar
            let header_height = 28.0;
            draw_rectangle(panel_x + padding, card_y, panel_width - 2.0 * padding, header_height, robot_color);
            // Vertically center robot ID and status in header
            let header_center_y = card_y + header_height / 2.0 + small_font_size / 2.2 - 2.0;
            let name_text = format!("ROBOT {}", robot.id);
            draw_text_ex(&name_text, panel_x + padding * 2.0, header_center_y, small_white_params.clone());
            let status_text = format!("{:?}", robot.status);
            let status_text_dims = measure_text(&status_text, self.ui_font.as_ref(), small_white_params.font_size, 1.0);
            draw_text_ex(&status_text, panel_x + panel_width - padding * 2.0 - status_text_dims.width, header_center_y, small_white_params.clone());
            
            // Define layout constants
            let card_inner_padding_x = padding * 2.0;
            let card_bar_width = panel_width - 2.0 * card_inner_padding_x;
            let bar_height = 12.0;
            let label_to_bar_y_offset = 2.0; // How far below label start the bar starts
            let bar_value_pad_x = 4.0; // Right padding for value in bar
            let value_font_size = small_value_params.font_size as f32;
            let value_in_bar_y_offset = bar_height * 0.85; // Offset for value text within bar height
            let row_v_spacing = 15.0; // Added 2px more vertical space between rows
            let label_to_instr_y_offset = small_font_size + 2.0; // Space between INSTR label and its value

            // Start drawing below the header (uses row_v_spacing)
            let mut current_y = card_y + header_height + row_v_spacing;

            // --- Health --- 
            let health_label_y = current_y;
            draw_text_ex("HEALTH", panel_x + card_inner_padding_x, health_label_y, small_gray_params.clone());
            
            let health_bar_y = health_label_y + label_to_bar_y_offset;
            let health_ratio = (robot.health / 100.0).clamp(0.0, 1.0) as f32;
            draw_rectangle(panel_x + card_inner_padding_x, health_bar_y, card_bar_width, bar_height, Color::from_rgba(54, 58, 70, 255));
            draw_rectangle(panel_x + card_inner_padding_x, health_bar_y, card_bar_width * health_ratio, bar_height, RED);
            
            let health_val = format!("{:.1}", robot.health);
            let health_val_dims = measure_text(&health_val, self.ui_font.as_ref(), value_font_size as u16, 1.0);
            draw_text_ex(&health_val, panel_x + card_inner_padding_x + card_bar_width - health_val_dims.width - bar_value_pad_x, health_bar_y + value_in_bar_y_offset, small_value_params.clone());
            
            // Update current_y to below the health bar + spacing
            current_y = health_bar_y + bar_height + row_v_spacing;

            // --- Power --- 
            let power_label_y = current_y;
            draw_text_ex("POWER", panel_x + card_inner_padding_x, power_label_y, small_gray_params.clone());
            
            let power_bar_y = power_label_y + label_to_bar_y_offset;
            let power_ratio = (robot.power / 1.0).clamp(0.0, 1.0) as f32;
            draw_rectangle(panel_x + card_inner_padding_x, power_bar_y, card_bar_width, bar_height, Color::from_rgba(54, 58, 70, 255));
            draw_rectangle(panel_x + card_inner_padding_x, power_bar_y, card_bar_width * power_ratio, 12.0, Color::from_rgba(40, 80, 180, 255));
            
            let power_val = format!("{:.2}", robot.power);
            let power_val_dims = measure_text(&power_val, self.ui_font.as_ref(), value_font_size as u16, 1.0);
            draw_text_ex(&power_val, panel_x + card_inner_padding_x + card_bar_width - power_val_dims.width - bar_value_pad_x, power_bar_y + value_in_bar_y_offset, small_value_params.clone());

            // Update current_y to below the power bar + spacing
            current_y = power_bar_y + bar_height + row_v_spacing;

            // --- Current Instruction --- 
            let instr_label_y = current_y;
            draw_text_ex("INSTR", panel_x + card_inner_padding_x, instr_label_y, small_gray_params.clone());
            
            let instr_str = robot.get_current_instruction_string();
            let instr_val_y = instr_label_y + label_to_instr_y_offset; 
            draw_text_ex(&instr_str, panel_x + card_inner_padding_x, instr_val_y, small_white_params.clone());
            
            // Update main y for next card
            y += card_height + card_spacing;
        }
    }

    fn draw_announcement(&self, msg: &str) {
        let rect_width = 500.0;
        let rect_height = 120.0;
        let x = (WINDOW_WIDTH as f32 / 2.0) - (rect_width / 2.0);
        let y = (WINDOW_HEIGHT as f32 / 2.0) - (rect_height / 2.0);
        draw_rectangle(x, y, rect_width, rect_height, faded_color(Color::from_rgba(0, 0, 0, 180), 1.0));
        
        // Use ui_font for announcement text
        let font_size_announcement = 32.0;
        let announcement_params = TextParams {
            font: self.ui_font.as_ref(),
            font_size: font_size_announcement as u16,
            color: WHITE,
            ..Default::default()
        };
        let text_dims = measure_text(msg, self.ui_font.as_ref(), announcement_params.font_size, 1.0);
        let text_x = x + (rect_width - text_dims.width) / 2.0;
        let text_y = y + (rect_height - font_size_announcement) / 2.0 + font_size_announcement * 0.7; // Adjust Y for better centering
        draw_text_ex(msg, text_x, text_y, announcement_params);
        
        // Use ui_font for hint text
        let hint = "Press ESC to exit";
        let hint_size = 18.0;
        let hint_params = TextParams {
            font: self.ui_font.as_ref(),
            font_size: hint_size as u16,
            color: LIGHTGRAY,
            ..Default::default()
        };
        let hint_dims = measure_text(hint, self.ui_font.as_ref(), hint_params.font_size, 1.0);
        let hint_x = x + (rect_width - hint_dims.width) / 2.0;
        draw_text_ex(hint, hint_x, y + rect_height - hint_size - 10.0, hint_params);
    }

    pub fn window_should_close() -> bool {
        is_key_down(KeyCode::Escape) || is_quit_requested()
    }

    pub fn is_key_down(key: KeyCode) -> bool {
        is_key_down(key)
    }
}
