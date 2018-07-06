// Copyright 2018 Google LLC, licensed under http://www.apache.org/licenses/LICENSE-2.0

use colors::{ColorScheme, Colors};
use ezgui::GfxCtx;
use ezgui::canvas::Canvas;
use ezgui::input::UserInput;
use ezgui::menu;
use graphics;
use piston::input::{Key, MouseCursorEvent};
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;

// TODO assumes minimum screen size
const WIDTH: u32 = 255;
const HEIGHT: u32 = 255;
const TILE_DIMS: u32 = 2;

// TODO parts of this should be in ezgui
pub enum ColorPicker {
    Inactive,
    Choosing(menu::Menu),
    // Remember the original color, in case we revert
    PickingColor(Colors, graphics::types::Color),
}

impl ColorPicker {
    pub fn new() -> ColorPicker {
        ColorPicker::Inactive
    }

    pub fn handle_event(
        &mut self,
        input: &mut UserInput,
        canvas: &Canvas,
        cs: &mut ColorScheme,
    ) -> bool {
        match self {
            ColorPicker::Inactive => {
                if input.unimportant_key_pressed(Key::D8, "Press 8 to configure colors") {
                    *self = ColorPicker::Choosing(menu::Menu::new(
                        Colors::iter().map(|c| c.to_string()).collect(),
                    ));
                    return true;
                }
                false
            }
            ColorPicker::Choosing(ref mut menu) => {
                // TODO arrow keys scroll canvas too
                match menu.event(input.use_event_directly()) {
                    menu::Result::Canceled => {
                        *self = ColorPicker::Inactive;
                    }
                    menu::Result::StillActive => {}
                    menu::Result::Done(choice) => {
                        let c = Colors::from_str(&choice).unwrap();
                        *self = ColorPicker::PickingColor(c, cs.get(c));
                    }
                };
                true
            }
            ColorPicker::PickingColor(c, orig_color) => {
                if input.key_pressed(
                    Key::Escape,
                    &format!(
                        "Press escape to stop configuring color for {:?} and revert",
                        c
                    ),
                ) {
                    cs.set(*c, *orig_color);
                    *self = ColorPicker::Inactive;
                    return true;
                }

                if input.key_pressed(
                    Key::Return,
                    &format!("Press enter to finalize new color for {:?}", c),
                ) {
                    println!("Setting color for {:?}", c);
                    *self = ColorPicker::Inactive;
                    return true;
                }

                if let Some(pos) = input.use_event_directly().mouse_cursor_args() {
                    // TODO argh too much casting
                    let (start_x, start_y) = get_screen_offset(canvas);
                    let x = (pos[0] - (start_x as f64)) / (TILE_DIMS as f64) / 255.0;
                    let y = (pos[1] - (start_y as f64)) / (TILE_DIMS as f64) / 255.0;
                    if x >= 0.0 && x <= 1.0 && y >= 0.0 && y <= 1.0 {
                        cs.set(*c, get_color(x as f32, y as f32));
                    }
                }

                true
            }
        }
    }

    pub fn draw(&self, canvas: &Canvas, g: &mut GfxCtx) {
        match self {
            ColorPicker::Inactive => {}
            ColorPicker::Choosing(menu) => {
                // TODO sloppy to use a mouse tooltip. ideally should be easy to figure out how
                // many lines to display and center it.
                // TODO would be nice to display the text in the current color
                canvas.draw_mouse_tooltip(g, &menu.lines_to_display());
            }
            ColorPicker::PickingColor(_, _) => {
                let (start_x, start_y) = get_screen_offset(canvas);

                for x in 0..WIDTH {
                    for y in 0..HEIGHT {
                        let color = get_color((x as f32) / 255.0, (y as f32) / 255.0);
                        g.draw_rectangle(
                            color,
                            [
                                canvas.screen_to_map_x((x * TILE_DIMS + start_x) as f64),
                                canvas.screen_to_map_y((y * TILE_DIMS + start_y) as f64),
                                TILE_DIMS as f64,
                                TILE_DIMS as f64,
                            ],
                        );
                    }
                }
            }
        }
    }
}

fn get_screen_offset(canvas: &Canvas) -> (u32, u32) {
    let total_width = TILE_DIMS * WIDTH;
    let total_height = TILE_DIMS * HEIGHT;
    let start_x = (canvas.window_size.width - total_width) / 2;
    let start_y = (canvas.window_size.height - total_height) / 2;
    (start_x, start_y)
}

fn get_color(x: f32, y: f32) -> graphics::types::Color {
    assert!(x >= 0.0 && x <= 1.0);
    assert!(y >= 0.0 && y <= 1.0);
    [x, y, (x + y) / 2.0, 1.0]
}
