use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;
use std::{thread, time};

use pixels::{
    // Error, // removed Error because it was unused and caused warning
    Pixels, 
    SurfaceTexture,
    wgpu::Color,
};

use std::time::Duration;
use rodio::{OutputStream, Source, Sink};

mod oscillator;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Cell {
    pub alive: bool,
    pub row: usize,
    pub col: usize
} // actually this class is not necessary?

#[derive(Debug, Default)]
pub struct GameBoard {
    height: usize,
    width: usize, 
    board: Vec<Vec<bool>>
    // board[i][j] - i is the row number, j is the column number
    // do we need to make board Vec<Vec<Cell>>?
}

const WIN_WIDTH: f64 = 900.0;
const WIN_HEIGHT: f64 = 600.0;
const PIX_WIDTH: u32 = 150;
const PIX_HEIGHT: u32 = 100;
const VOLUME_RANGE: f32 = 100.0;
const FREQUENCY_RANGE: f32 = 15000.0;
const NOTE_NUM: usize = 12;
// the game board is divided into 12 sections in a row

// Do we want to wrap the return values with Enums?
impl GameBoard {

    pub fn new(height: usize, width: usize) -> GameBoard {
        GameBoard {
            height,
            width,
            board: vec![vec![false; width]; height]
        }
    }

    pub fn get_board(&self) -> &Vec<Vec<bool>> {
        &self.board
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    // update the game board with the data in a new game board, 
    // not taking the new game board's ownership
    // need to work on error handling: try avoid using panic!()
    pub fn update(&mut self, board_from: &GameBoard) {
        if board_from.height != self.height || board_from.width != self.width {
            panic!("two boards have different sizes!");
        }
        for row in 0..board_from.height {
            for col in 0..board_from.width {
                self.board[row][col] = board_from.board[row][col];
            }
        }
    }

    pub fn clear(&mut self) {
        for row in 0..self.height {
            for col in 0..self.width {
                self.board[row][col] = false;
            }
        }
    }

    // a print function for the game board
    pub fn print(&self) {
        println!("---Current Game Board---");
        println!("height: {:?}      width: {:?}", self.height, self.width);
        println!("------------------------");
        for row in 0..self.height {
            let mut row_str: String = String::from(" ");
            for col in 0..self.width {
                if self.board[row][col] {
                    row_str += "1 ";
                } else {
                    row_str += "0 ";
                }
            }
            println!("{:?}", row_str);
        }
    }

    // sets the board element at (row, col) to the alive parameter (true/false)
    pub fn set_cell(&mut self, alive: bool, row: usize, col: usize) -> bool {
        if row >= self.height || col >= self.width {
            panic!("invalid board position!");
        }
        self.board[row][col] = alive;

        alive
    }

    pub fn get_sound_data(&self) -> (f32, usize) {
        // let mut sound_data: f32 = 0;
        let SCALE_HERTZ: Vec<f32> = vec![261.63, 277.18, 293.66, 311.13, 329.63, 349.23, 
            369.99, 392.00, 415.30, 440.00, 466.16, 493.88];
        let section_width: usize = PIX_WIDTH as usize / NOTE_NUM;
        let pixel_per_section: usize = section_width * PIX_HEIGHT as usize;
        let mut max_alive_count: usize = 0;
        let mut max_section_num: usize = 0;

        for sec in 0..NOTE_NUM { 
            let mut total_alive: usize = 0;
            for col in sec * section_width..(sec + 1) * section_width {
                for row in 0..self.height {
                    if self.board[row][col] {
                        total_alive += 1;
                    }
                }
            }
            if max_alive_count < total_alive {
                max_alive_count = total_alive;
                max_section_num = sec;
            }
        }

        return (SCALE_HERTZ[max_section_num], max_section_num);
    }

    // evolves the elements of the board
    pub fn evolve(&mut self) {
        // copy of self's board to evolve
        let mut future = GameBoard::new(self.height,self.width);
        future.update(self);
        // iterate through future copy (starts out same as self's)
        for i in 0..future.height {
            for j in 0..future.width {
                if self.board[i][j] == true { // if cell at location is live
                    future.evolve_live_cell(self, i, j);
                } else { // else if dead
                    future.evolve_dead_cell(self, i, j);
                }
            }
        }
    
        // copy board back into self
        self.update(&future);
    }

    // counts the number of neighbors around board element at (row,col) that are alive
    fn count_live_neighbors(&mut self, row: usize, col: usize) -> usize {
        // live nieghbor count
        let mut n: usize = 0;
    
        // negatives are not allowed in ranges in rust :/
        // so i have to shift things and make i32 copies of the usizes
        let mut i: i32 = -1;
        let mut j: i32 = -1;

        let r: i32 = row.try_into().unwrap();
        let c: i32 = col.try_into().unwrap();
        let height: i32 = self.height.try_into().unwrap();
        let width: i32 = self.width.try_into().unwrap();

        // iterate through neighbors
        while i < 2 {
            while j < 2 {
                if i == 0 && j == 0 { // if at reference element
                    // do nothing (quick but lazy sorry)
                } else {
                    // if location is in board
                    if r + i >= 0 && r + i < height && 
                    c + j >= 0 && c + j < width {
                        // if live
                        // can only index with usize so must typecast
                        if self.board[(r + i) as usize][(c + j) as usize] == true {
                            n += 1; // increment live count
                        }
                    }
                }

                j += 1;
            }

            i += 1;
            j = -1;
        }
        
    
        // return live count
        n
    }

    fn evolve_live_cell(&mut self, board: &mut GameBoard, row: usize, col: usize) {
        let n: usize = board.count_live_neighbors(row, col);
        if n < 2 || n > 3 { // if less than 2 or greater than 3 live neighbors
            self.set_cell(false, row, col); // set to dead
        }
    }

    fn evolve_dead_cell(&mut self, board: &mut GameBoard, row: usize, col: usize) {
        let n: usize = board.count_live_neighbors(row, col);
        if n == 3 { // if exactly 3 live neighbors
            self.set_cell(true, row, col); // set to live
        }
    }

    fn draw(&self, frame: &mut [u8]) {
        debug_assert_eq!(frame.len(), 4 * self.width * self.height);
        let mut count: usize = 0;
        for cell in frame.chunks_exact_mut(4) {
            let col: usize = count % (PIX_WIDTH as usize);
            let row: usize = count / (PIX_WIDTH as usize);

            let color = if self.board[row][col] {
                // light blue if alive
                [0, 0xff, 0xff, 0xff]
            } else {
                
                // black if not alive
                [0, 0, 0, 0xff]
            };

            cell.copy_from_slice(&color);
            count += 1;
        }
    }

    fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) {
        // probably should do sutherland-hodgeman if this were more serious.
        // instead just clamp the start pos, and draw until moving towards the
        // end pos takes us out of bounds.
        let x0 = x0.max(0).min(self.width as isize);
        let y0 = y0.max(0).min(self.height as isize);
        for (x, y) in line_drawing::Bresenham::new((x0, y0), (x1, y1)) {
            if let Some(i) = self.grid_idx(x, y) {
                self.set_cell(alive, y as usize, x as usize);
            } else {
                break;
            }
        }
    }

    fn grid_idx<I: std::convert::TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        if let (Ok(x), Ok(y)) = (x.try_into(), y.try_into()) {
            if x < self.width && y < self.height {
                Some(x + y * self.width)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn main() -> Result<(), pixels::Error> {

    println!("Starting the game...");
    
    /*

    test codes
    
    let mut b_test = GameBoard::new(10,20);
    b_test.print(); // display empty board
    b_test.set_cell(true, 5, 5);
    b_test.set_cell(true, 6, 5);
    b_test.set_cell(true, 6, 6);
    b_test.set_cell(true, 7, 7);
    b_test.set_cell(true, 8, 8);
    b_test.print(); // display starting board
    // let it evolve a bit
    b_test.evolve(); // 1
    b_test.print();
    
    b_test.evolve(); // 2
    b_test.print();
    
    b_test.evolve(); // 3
    b_test.print(); // (same as 2 because it is a stagnant square pattern)

    */

    let event_loop = EventLoop::new();
    let mut win_input = WinitInputHelper::new();
    let window = WindowBuilder::new()
                .with_title("KAL Seagull")
                .with_inner_size(LogicalSize::new(WIN_WIDTH, WIN_HEIGHT))
                .with_min_inner_size(LogicalSize::new(WIN_WIDTH, WIN_HEIGHT))
                .build(&event_loop).unwrap();
    
    // display GameBoard with pixels            
    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(PIX_WIDTH, PIX_HEIGHT, surface_texture)?
    };

    // before displaying the color of the window should be white 
    // although it's never white because the program displays the gameboard instantly
    pixels.set_clear_color(Color::WHITE);

    let mut game_board = GameBoard::new(PIX_HEIGHT as usize, PIX_WIDTH as usize);

    let mut paused = true;
    let mut draw_state: Option<bool> = None;

    let NOTE_NAMES: Vec<String> = vec!["C".to_string(), "C#".to_string(), "D".to_string(), 
        "D#".to_string(), "E".to_string(), "E#".to_string(), 
        "F".to_string(), "G".to_string(), "G#".to_string(), 
        "A".to_string(), "A#".to_string(), "B".to_string()];

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::RedrawRequested(_) => {
                game_board.draw(pixels.get_frame());
                let r = pixels.render();
                match r {
                    Ok(_) => (),
                    Err(error) => panic!("Problem rendering: {:?}", error),
                };
            },
            _ => (),
        }

        // if keys are pressed
        if win_input.update(&event) {
            // Close the window
            if win_input.key_pressed(VirtualKeyCode::Escape) || win_input.quit() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            if win_input.key_pressed(VirtualKeyCode::Space) {
                // use the space key to change the state of paused or not paused
                paused = !paused;
            }
            if paused && win_input.key_pressed(VirtualKeyCode::E) {
                game_board.evolve();
                game_board.draw(pixels.get_frame());
                let r = pixels.render();
                match r {
                    Ok(_) => (),
                    Err(error) => panic!("Problem rendering: {:?}", error),
                };
            }
            if win_input.key_pressed(VirtualKeyCode::C) {
                game_board.clear();
                game_board.draw(pixels.get_frame());
                let r = pixels.render();
                match r {
                    Ok(_) => (),
                    Err(error) => panic!("Problem rendering: {:?}", error),
                };
            }

            /* 
                playing sound
                the game board is divided into 12 sections (accords to 12 pitches in an octave)
                count the number of alive cells in each section
                play the pitch corresponding to the section that has the most alive cells
                if there are two sections that have the same amount of alive cells, 
                the leftmost section would be favored.
            */

            if win_input.key_pressed(VirtualKeyCode::P) {
                let (freq, note_index) = game_board.get_sound_data();
                println!("Note played: {:?}", NOTE_NAMES[note_index]);
                println!("Frequency: {:?}", freq);
                let mut sine_oscillator = oscillator::WavetableOscillator::init_sine_oscillator();
                sine_oscillator.set_frequency(freq);
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                let _result = stream_handle.play_raw(sine_oscillator.convert_samples());
                std::thread::sleep(Duration::from_secs(1));
            }

            // Altered from pixels example 
            // win_input was input, game_board was life
            // Handle mouse. This is a bit involved since support some simple
            // line drawing (mostly because it makes nice looking patterns).
            let (mouse_cell, mouse_prev_cell) = win_input
                .mouse()
                .map(|(mx, my)| {
                    let (dx, dy) = win_input.mouse_diff();
                    let prev_x = mx - dx;
                    let prev_y = my - dy;

                    let (mx_i, my_i) = pixels
                        .window_pos_to_pixel((mx, my))
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    let (px_i, py_i) = pixels
                        .window_pos_to_pixel((prev_x, prev_y))
                        .unwrap_or_else(|pos| pixels.clamp_pixel_pos(pos));

                    (
                        (mx_i as isize, my_i as isize),
                        (px_i as isize, py_i as isize),
                    )
                })
                .unwrap_or_default();

            if win_input.mouse_pressed(0) {
                // debug!("Mouse click at {:?}", mouse_cell);
                // set cell to alive
                let y_pos: usize = mouse_cell.1.try_into().unwrap();
                let x_pos: usize = mouse_cell.0.try_into().unwrap();
                let state: bool = game_board.board[y_pos][x_pos];
                draw_state = Some(game_board.set_cell(!state, y_pos, x_pos));
                // game_board.evolve();
                
            } else if let Some(draw_alive) = draw_state {
                let release = win_input.mouse_released(0);
                let held = win_input.mouse_held(0);
                // debug!("Draw at {:?} => {:?}", mouse_prev_cell, mouse_cell);
                // debug!("Mouse held {:?}, release {:?}", held, release);
                // If they either released (finishing the drawing) or are still
                // in the middle of drawing, keep going.
                if release || held {
                    // debug!("Draw line of {:?}", draw_alive);
                    game_board.set_line(
                        mouse_prev_cell.0.try_into().unwrap(),
                        mouse_prev_cell.1.try_into().unwrap(),
                        mouse_cell.0.try_into().unwrap(),
                        mouse_cell.1.try_into().unwrap(),
                        draw_alive,
                    );
                }
                // If they let go or are otherwise not clicking anymore, stop drawing.
                if release || !held {
                    // debug!("Draw end");
                    game_board.draw(pixels.get_frame());
                    draw_state = None;
                }

                
            }
            game_board.draw(pixels.get_frame());
            let r = pixels.render();
            match r {
                Ok(_) => (),
                Err(error) => panic!("Problem rendering: {:?}", error),
            };

            if !paused {
                game_board.evolve();
                game_board.draw(pixels.get_frame());
                let r = pixels.render();
                match r {
                    Ok(_) => (),
                    Err(error) => panic!("Problem rendering: {:?}", error),
                };
            }

            window.request_redraw();
        }
        
    });
}
