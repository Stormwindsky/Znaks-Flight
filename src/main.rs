use macroquad::prelude::*;
use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};
use resvg::usvg::{Options, Tree};
use resvg::tiny_skia::{Pixmap, Transform};

#[derive(PartialEq, Clone, Copy)]
enum GameMode {
    Classic,
    SwapedRule,
}

#[derive(PartialEq)]
enum GameState {
    MainMenu,
    ModeSelect,
    Playing,
    Dying,
    GameOver,
    Credits,
}

struct Pipe {
    x: f32,
    gap_y: f32,
    gap_height: f32,
    width: f32,
    passed: bool,
    top_hit: bool,
    bottom_hit: bool,
    top_collapsing: bool,
    bottom_collapsing: bool,
    top_collapse_y: f32,
    bottom_collapse_y: f32,
    top_shake_offset: f32,
    bottom_shake_offset: f32,
}

fn load_svg_texture(path: &str, target_width: u32, target_height: u32) -> Option<Texture2D> {
    let svg_data = std::fs::read(path).ok()?;
    let opt = Options::default();
    let fontdb = resvg::usvg::fontdb::Database::new();
    let tree = Tree::from_data(&svg_data, &opt, &fontdb).ok()?;

    let mut pixmap = Pixmap::new(target_width, target_height)?;

    let scale_x = target_width as f32 / tree.size().width();
    let scale_y = target_height as f32 / tree.size().height();
    let transform = Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = Image {
        bytes: pixmap.data().to_vec(),
        width: target_width as u16,
        height: target_height as u16,
    };

    Some(Texture2D::from_image(&image))
}

fn draw_button(text: &str, x: f32, y: f32, width: f32, height: f32, font: Option<&Font>) -> bool {
    let mouse_pos = mouse_position();
    let rect = Rect::new(x, y, width, height);
    let hovered = rect.contains(vec2(mouse_pos.0, mouse_pos.1));

    let bg_color = if hovered { DARKBLUE } else { BLUE };
    draw_rectangle(x, y, width, height, bg_color);
    draw_rectangle_lines(x, y, width, height, 3.0, WHITE);

    let text_params = TextParams {
        font,
        font_size: 22,
        color: WHITE,
        ..Default::default()
    };
    let text_dim = measure_text(text, font, 22, 1.0);
    draw_text_ex(
        text,
        x + (width - text_dim.width) / 2.0,
        y + (height + text_dim.height) / 2.0,
        text_params,
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

#[macroquad::main("Znak's Flight")]
async fn main() {
    let font = load_ttf_font("font.otf").await.ok();

    let tex_normal = load_svg_texture("img/Normal_Character.svg", 40, 40);
    let tex_game_over = load_svg_texture("img/GameOver_Character.svg", 40, 40);
    let tex_castle_down = load_svg_texture("img/Castle_Down.svg", 120, 600);
    let tex_castle_up = load_svg_texture("img/Castle_Up.svg", 120, 600);

    let tex_logo = load_texture("img/Logo.png").await.ok();

    let sfx_jump: Option<Sound> = load_sound("sfx/jump.wav").await.ok();
    let sfx_point: Option<Sound> = load_sound("sfx/point.wav").await.ok();
    let sfx_touch: Option<Sound> = load_sound("sfx/touch.wav").await.ok();
    let sfx_failed: Option<Sound> = load_sound("sfx/failed.wav").await.ok();
    let sfx_destroy: Option<Sound> = load_sound("sfx/destroy.wav").await.ok();

    let ost_main: Option<Sound> = load_sound("music/main_ost.wav").await.ok();
    let ost_game: Option<Sound> = load_sound("music/game_ost.wav").await.ok();

    let credits_content = std::fs::read_to_string("credits.txt").unwrap_or_else(|_| "Created by Stormwindsky".to_string());
    let credits_lines: Vec<String> = credits_content.lines().map(|s| s.to_string()).collect();

    let mut state = GameState::MainMenu;
    let mut current_game_mode = GameMode::Classic;
    let mut show_exit_popup = false;
    let mut credits_scroll_y = 0.0;
    let mut death_timer = 0.0;

    let mut player_y = screen_height() / 2.0;
    let mut velocity = 0.0;
    let gravity = 800.0;
    let jump_strength = -300.0;
    let player_x = 100.0;
    let player_size = 40.0;

    let mut score: u32 = 0;
    let mut high_score: u32 = 0;
    let mut pipes: Vec<Pipe> = Vec::new();
    let mut pipe_timer = 0.0;
    let pipe_spawn_rate = 1.8;
    let pipe_speed = 200.0;
    let pipe_width = 60.0;
    let gap_height = 150.0;

    if let Some(ref music) = ost_main {
        play_sound(music, PlaySoundParams { looped: true, volume: 0.8 });
    }

    loop {
        let delta = get_frame_time();
        let time = get_time();

        clear_background(SKYBLUE);

        match state {
            GameState::MainMenu => {
                let center_x = screen_width() / 2.0;
                let logo_y_offset = (time * 3.0).sin() as f32 * 12.0;
                let logo_y = 50.0 + logo_y_offset;

                if let Some(ref logo) = tex_logo {
                    let logo_w = 320.0;
                    let logo_h = 160.0;
                    draw_texture_ex(
                        logo,
                        center_x - logo_w / 2.0,
                        logo_y,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(logo_w, logo_h)),
                            ..Default::default()
                        },
                    );
                } else {
                    let title = "Znak's Flight";
                    let dim = measure_text(title, font.as_ref(), 50, 1.0);
                    draw_text_ex(
                        title,
                        center_x - dim.width / 2.0,
                        logo_y + 80.0,
                        TextParams { font: font.as_ref(), font_size: 50, color: DARKBLUE, ..Default::default() },
                    );
                }

                let btn_w = 220.0;
                let btn_h = 45.0;
                let start_btn_y = 240.0;

                if !show_exit_popup {
                    if draw_button("Play", center_x - btn_w / 2.0, start_btn_y, btn_w, btn_h, font.as_ref()) {
                        state = GameState::ModeSelect;
                    }

                    if draw_button("Credits", center_x - btn_w / 2.0, start_btn_y + 60.0, btn_w, btn_h, font.as_ref()) {
                        credits_scroll_y = 0.0;
                        state = GameState::Credits;
                    }

                    if draw_button("Leave", center_x - btn_w / 2.0, start_btn_y + 120.0, btn_w, btn_h, font.as_ref()) {
                        show_exit_popup = true;
                    }
                }

                if show_exit_popup {
                    let pop_w = 380.0;
                    let pop_h = 220.0;
                    let pop_x = center_x - pop_w / 2.0;
                    let pop_y = screen_height() / 2.0 - pop_h / 2.0;

                    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.6));
                    draw_rectangle(pop_x, pop_y, pop_w, pop_h, DARKGRAY);
                    draw_rectangle_lines(pop_x, pop_y, pop_w, pop_h, 4.0, WHITE);

                    let q_text = "Are you sure you want to leave?";
                    let dim = measure_text(q_text, font.as_ref(), 20, 1.0);
                    draw_text_ex(
                        q_text,
                        pop_x + (pop_w - dim.width) / 2.0,
                        pop_y + 60.0,
                        TextParams { font: font.as_ref(), font_size: 20, color: WHITE, ..Default::default() },
                    );

                    let p_btn_w = 120.0;
                    let p_btn_h = 40.0;

                    if draw_button("Proceed", pop_x + 40.0, pop_y + 130.0, p_btn_w, p_btn_h, font.as_ref()) {
                        std::process::exit(0);
                    }

                    if draw_button("Cancel", pop_x + pop_w - 40.0 - p_btn_w, pop_y + 130.0, p_btn_w, p_btn_h, font.as_ref()) {
                        show_exit_popup = false;
                    }
                }
            }

            GameState::ModeSelect => {
                let center_x = screen_width() / 2.0;
                let title = "SELECT GAME MODE";
                let dim = measure_text(title, font.as_ref(), 35, 1.0);
                draw_text_ex(
                    title,
                    center_x - dim.width / 2.0,
                    100.0,
                    TextParams { font: font.as_ref(), font_size: 35, color: DARKBLUE, ..Default::default() },
                );

                let btn_w = 260.0;
                let btn_h = 50.0;

                if draw_button("Classic Mode", center_x - btn_w / 2.0, 200.0, btn_w, btn_h, font.as_ref()) {
                    current_game_mode = GameMode::Classic;
                    if let Some(ref music) = ost_main { stop_sound(music); }
                    if let Some(ref music) = ost_game { play_sound(music, PlaySoundParams { looped: true, volume: 0.8 }); }

                    player_y = screen_height() / 2.0;
                    velocity = 0.0;
                    score = 0;
                    pipes.clear();
                    pipe_timer = 0.0;
                    state = GameState::Playing;
                }

                if draw_button("Swaped Rule", center_x - btn_w / 2.0, 270.0, btn_w, btn_h, font.as_ref()) {
                    current_game_mode = GameMode::SwapedRule;
                    if let Some(ref music) = ost_main { stop_sound(music); }
                    if let Some(ref music) = ost_game { play_sound(music, PlaySoundParams { looped: true, volume: 0.8 }); }

                    player_y = screen_height() / 2.0;
                    velocity = 0.0;
                    score = 0;
                    pipes.clear();
                    pipe_timer = 0.0;
                    state = GameState::Playing;
                }

                if draw_button("Back", center_x - btn_w / 2.0, 360.0, btn_w, btn_h, font.as_ref()) {
                    state = GameState::MainMenu;
                }
            }

            GameState::Credits => {
                let center_x = screen_width() / 2.0;
                let title = "CREDITS";
                let dim = measure_text(title, font.as_ref(), 40, 1.0);
                draw_text_ex(
                    title,
                    center_x - dim.width / 2.0,
                    50.0,
                    TextParams { font: font.as_ref(), font_size: 40, color: DARKBLUE, ..Default::default() },
                );

                let mouse_wheel = mouse_wheel().1;
                credits_scroll_y += mouse_wheel * 25.0;
                if is_key_down(KeyCode::Up) { credits_scroll_y += 5.0; }
                if is_key_down(KeyCode::Down) { credits_scroll_y -= 5.0; }
                if credits_scroll_y > 0.0 { credits_scroll_y = 0.0; }

                let start_y = 110.0 + credits_scroll_y;
                let mouse_pos = mouse_position();

                for (i, line) in credits_lines.iter().enumerate() {
                    let line_y = start_y + (i as f32 * 32.0);

                    if line_y > 80.0 && line_y < screen_height() - 80.0 {
                        let is_url = line.starts_with("http://") || line.starts_with("https://");
                        let text_color = if is_url { YELLOW } else { WHITE };

                        let line_dim = measure_text(line, font.as_ref(), 20, 1.0);
                        let line_x = center_x - line_dim.width / 2.0;

                        if is_url {
                            let link_rect = Rect::new(line_x, line_y - 18.0, line_dim.width, 24.0);
                            let hovered = link_rect.contains(vec2(mouse_pos.0, mouse_pos.1));

                            let draw_color = if hovered { GOLD } else { YELLOW };
                            draw_text_ex(
                                line,
                                line_x,
                                line_y,
                                TextParams { font: font.as_ref(), font_size: 20, color: draw_color, ..Default::default() },
                            );
                            draw_line(line_x, line_y + 3.0, line_x + line_dim.width, line_y + 3.0, 2.0, draw_color);

                            if hovered && is_mouse_button_pressed(MouseButton::Left) {
                                let _ = open::that(line);
                            }
                        } else {
                            draw_text_ex(
                                line,
                                line_x,
                                line_y,
                                TextParams { font: font.as_ref(), font_size: 20, color: text_color, ..Default::default() },
                            );
                        }
                    }
                }

                if draw_button("Back", 20.0, screen_height() - 60.0, 120.0, 40.0, font.as_ref()) {
                    state = GameState::MainMenu;
                }
            }

            GameState::Playing => {
                if is_key_pressed(KeyCode::Space) {
                    velocity = jump_strength;
                    if let Some(ref sound) = sfx_jump { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                }

                velocity += gravity * delta;
                player_y += velocity * delta;

                pipe_timer += delta;
                if pipe_timer >= pipe_spawn_rate {
                    pipe_timer = 0.0;
                    let min_y = 100.0;
                    let max_y = screen_height() - 100.0 - gap_height;
                    let gap_y = rand::gen_range(min_y, max_y);

                    pipes.push(Pipe {
                        x: screen_width(),
                        gap_y,
                        gap_height,
                        width: pipe_width,
                        passed: false,
                        top_hit: false,
                        bottom_hit: false,
                        top_collapsing: false,
                        bottom_collapsing: false,
                        top_collapse_y: 0.0,
                        bottom_collapse_y: gap_y + gap_height,
                        top_shake_offset: 0.0,
                        bottom_shake_offset: 0.0,
                    });
                }

                let player_rect = Rect::new(player_x, player_y, player_size, player_size);
                let mut died = false;
                let mut play_failed_sound = false;

                for pipe in pipes.iter_mut() {
                    pipe.x -= pipe_speed * delta;

                    if pipe.top_collapsing {
                        pipe.top_collapse_y += 400.0 * delta;
                        pipe.top_shake_offset = (time * 50.0).sin() as f32 * 6.0;
                    }
                    if pipe.bottom_collapsing {
                        pipe.bottom_collapse_y += 400.0 * delta;
                        pipe.bottom_shake_offset = (time * 50.0).sin() as f32 * 6.0;
                    }

                    let top_pipe_rect = Rect::new(pipe.x, 0.0, pipe.width, pipe.gap_y);
                    let bottom_pipe_rect = Rect::new(pipe.x, pipe.gap_y + pipe.gap_height, pipe.width, screen_height() - (pipe.gap_y + pipe.gap_height));

                    // Only check overlaps if they haven't been hit/collapsed yet in SwapedRule mode
                    let overlaps_top = !pipe.top_hit && player_rect.overlaps(&top_pipe_rect);
                    let overlaps_bottom = !pipe.bottom_hit && player_rect.overlaps(&bottom_pipe_rect);

                    if current_game_mode == GameMode::Classic {
                        if player_rect.overlaps(&top_pipe_rect) || player_rect.overlaps(&bottom_pipe_rect) {
                            died = true;
                            if let Some(ref sound) = sfx_touch { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        }

                        if !pipe.passed && pipe.x + pipe.width < player_x {
                            pipe.passed = true;
                            score += 1;
                            if score > high_score { high_score = score; }
                            if let Some(ref sound) = sfx_point { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        }
                    } else {
                        // Swaped Rule Mode: touching the top OR the bottom castle gives 1 point (max 1 per pipe)
                        let mut already_scored = pipe.top_hit || pipe.bottom_hit;

                        if overlaps_top {
                            pipe.top_hit = true;
                            pipe.top_collapsing = true;
                            if !already_scored {
                                score += 1;
                                if score > high_score { high_score = score; }
                                already_scored = true;
                            }
                            if let Some(ref sound) = sfx_destroy { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        }

                        if overlaps_bottom {
                            pipe.bottom_hit = true;
                            pipe.bottom_collapsing = true;
                            if !already_scored {
                                score += 1;
                                if score > high_score { high_score = score; }
                            }
                            if let Some(ref sound) = sfx_destroy { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                        }

                        // FIX: you only die if you touched NEITHER castle (&& instead of ||)
                        if !pipe.passed && pipe.x + pipe.width < player_x {
                            pipe.passed = true;
                            if !pipe.top_hit && !pipe.bottom_hit {
                                died = true;
                                play_failed_sound = true;
                            }
                        }
                    }
                }

                pipes.retain(|p| p.x + p.width > 0.0 && (p.top_collapse_y < screen_height() || p.bottom_collapse_y < screen_height()));

                if player_y < 0.0 || player_y + player_size > screen_height() {
                    died = true;
                    if current_game_mode == GameMode::SwapedRule {
                        play_failed_sound = true;
                    } else {
                        if let Some(ref sound) = sfx_touch { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                    }
                }

                if died {
                    if play_failed_sound {
                        if let Some(ref sound) = sfx_failed { play_sound(sound, PlaySoundParams { looped: false, volume: 1.0 }); }
                    }
                    if let Some(ref music) = ost_game { stop_sound(music); }

                    state = GameState::Dying;
                    death_timer = 0.0;
                    velocity = -200.0;
                }

                // Render Castles independently
                for pipe in pipes.iter() {
                    let top_draw_x = pipe.x + pipe.top_shake_offset;
                    if let Some(ref tex) = tex_castle_down {
                        draw_texture_ex(tex, top_draw_x, pipe.top_collapse_y, WHITE, DrawTextureParams { dest_size: Some(vec2(pipe.width, pipe.gap_y)), ..Default::default() });
                    } else {
                        draw_rectangle(top_draw_x, pipe.top_collapse_y, pipe.width, pipe.gap_y, GREEN);
                    }

                    let bottom_draw_x = pipe.x + pipe.bottom_shake_offset;
                    let bottom_height = screen_height() - pipe.bottom_collapse_y;
                    if let Some(ref tex) = tex_castle_up {
                        draw_texture_ex(tex, bottom_draw_x, pipe.bottom_collapse_y, WHITE, DrawTextureParams { dest_size: Some(vec2(pipe.width, bottom_height)), ..Default::default() });
                    } else {
                        draw_rectangle(bottom_draw_x, pipe.bottom_collapse_y, pipe.width, bottom_height, GREEN);
                    }
                }

                if let Some(ref tex) = tex_normal {
                    draw_texture_ex(tex, player_x, player_y, WHITE, DrawTextureParams { dest_size: Some(vec2(player_size, player_size)), ..Default::default() });
                } else {
                    draw_rectangle(player_x, player_y, player_size, player_size, YELLOW);
                }

                let score_str = format!("Score: {}", score);
                draw_text_ex(&score_str, 20.0, 40.0, TextParams { font: font.as_ref(), font_size: 30, color: WHITE, ..Default::default() });
            }

            GameState::Dying => {
                death_timer += delta;
                velocity += (gravity * 1.5) * delta;
                player_y += velocity * delta;

                for pipe in pipes.iter() {
                    let top_draw_x = pipe.x + pipe.top_shake_offset;
                    if let Some(ref tex) = tex_castle_down {
                        draw_texture_ex(tex, top_draw_x, pipe.top_collapse_y, WHITE, DrawTextureParams { dest_size: Some(vec2(pipe.width, pipe.gap_y)), ..Default::default() });
                    }
                    let bottom_draw_x = pipe.x + pipe.bottom_shake_offset;
                    let bottom_height = screen_height() - pipe.bottom_collapse_y;
                    if let Some(ref tex) = tex_castle_up {
                        draw_texture_ex(tex, bottom_draw_x, pipe.bottom_collapse_y, WHITE, DrawTextureParams { dest_size: Some(vec2(pipe.width, bottom_height)), ..Default::default() });
                    }
                }

                if let Some(ref tex) = tex_game_over {
                    draw_texture_ex(
                        tex,
                        player_x,
                        player_y,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(vec2(player_size, player_size)),
                            rotation: (death_timer * 4.0).min(1.57),
                            ..Default::default()
                        },
                    );
                } else {
                    draw_rectangle(player_x, player_y, player_size, player_size, RED);
                }

                if death_timer >= 0.8 || player_y > screen_height() {
                    state = GameState::GameOver;
                }
            }

            GameState::GameOver => {
                draw_text_ex("GAME OVER", screen_width() / 2.0 - 110.0, screen_height() / 3.0, TextParams { font: font.as_ref(), font_size: 50, color: RED, ..Default::default() });

                let final_score_str = format!("Score: {}  |  Best: {}", score, high_score);
                draw_text_ex(&final_score_str, screen_width() / 2.0 - 130.0, screen_height() / 2.0, TextParams { font: font.as_ref(), font_size: 30, color: WHITE, ..Default::default() });

                draw_text_ex("Press SPACE to return to Menu", screen_width() / 2.0 - 160.0, screen_height() / 2.0 + 60.0, TextParams { font: font.as_ref(), font_size: 25, color: LIGHTGRAY, ..Default::default() });

                if is_key_pressed(KeyCode::Space) {
                    if let Some(ref music) = ost_main { play_sound(music, PlaySoundParams { looped: true, volume: 0.8 }); }
                    state = GameState::MainMenu;
                }
            }
        }

        next_frame().await
    }
}
