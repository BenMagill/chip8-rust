use std::env;
use std::fs;
extern crate hex;
extern crate piston_window;
use piston_window::*;
extern crate opengl_graphics;
use opengl_graphics::{GlGraphics, OpenGL};
#[macro_use]
extern crate twelve_bit;
use twelve_bit::u12::*;
extern crate rand;
use rand::Rng;
const MEMORY_OFFSET: u16 = 512;
const SPRITE_START: usize = 0;
const PROGRAM_START: usize = 512;
const INSTRUCTIONS_PER_SECOND: u16 = 700;
const MEMORY_SIZE: usize = 4096;
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

const SCREEN_X: u8 = 64;
const SCREEN_Y: usize = 32;

const SHOW_GRID: bool = true;

const SPRITES: [u8; 80] = [
    // 0
    0xF0, 0x90, 0x90, 0x90, 0xF0,
    // 1
    0x20, 0x60, 0x20, 0x20, 0x70,
    // 2
    0xF0, 0x10, 0xF0, 0x80, 0xF0,
    // 3
    0xF0, 0x10, 0xF0, 0x10, 0xF0,
    // 4
    0x90, 0x90, 0xF0, 0x10, 0x10,
    // 5
    0xF0, 0x80, 0xF0, 0x10, 0xF0,
    // 6
    0xF0, 0x80, 0xF0, 0x90, 0xF0,
    // 7
    0xF0, 0x10, 0x20, 0x40, 0x40,
    // 8
    0xF0, 0x90, 0xF0, 0x90, 0xF0,
    // 9
    0xF0, 0x90, 0xF0, 0x10, 0xF0,
    // A
    0xF0, 0x90, 0xF0, 0x90, 0x90,
    // B
    0xE0, 0x90, 0xE0, 0x90, 0xE0,
    // C
    0xF0, 0x80, 0x80, 0x80, 0xF0,
    // D
    0xE0, 0x90, 0x90, 0x90, 0xE0,
    // E
    0xF0, 0x80, 0xF0, 0x80, 0xF0,
    // F
    0xF0, 0x80, 0xF0, 0x80, 0x80,
];

/**
 * Memory
 *  8 byte per location
 *  must start memory at 512 bytes as this would contain the interpreter
 *  
 * Registers
 *  16 8 bit registers
 *  V0 to VF
 *  VF used as a flag for some instructions
 *  address register: 
 *      12 bits wide
 *  PC (starts at 64 (40 hex) )
 * 
 * Stack
 *  stores return addresses for subroutines
 * 
 * Timers
 *  count down at 60 times per second 
 *  Delay:
 *      used for timing events in video games
 *  Sound
 *      when nonzero a beep is made
 * 
 * Input
 *  input hex characters for input
 *  maybe remap to different keybaord characters
 * 
 * Graphics
 *  monochrome 64 x 32
 *  drawn with sprites (8 x 1 to 15)
 *  sprite pixels XORd wit corresponding screen pixels
 *  carry flag (VF) set to 1 if any screen pixels flipped from set to unset when sprite drawn otherwise 0
 *  STORING
 *      store all of pixels as binary for each row 
 *  WRITING
 *      XOR the data at location I With data starting at a position
 *      set VF to 1 if any pixels unset
 * 
 * Opcode understanding
 *  NNN = address location
 *  N or NN = value
 *  X or Y 
 *  I (MAR) 16 bit
 */

//  TODO: need to store hex character bytes in the memory from 0x000 to 0x1FF
// TODO: reference suggests memory should be 8 bit not u16 as I have done. Determine what is better?
        // 8 bit would be better for storing font sprites and other things to maybe do this
        // plus memory locations will be broke if not done so 

fn handle_invalid_instruction(&instruction: &(u8, u8)) {
    println!("Invalid instruction {:X},{:X}", instruction.0, instruction.1);
}

fn extract_address(&instruction: &(u8, u8)) -> u16 {
    return ((instruction.0 & 0x0F) as u16) << 8 | instruction.1 as u16;
}

// Returns {0: x, 1: kk}
fn xkk(&instruction: &(u8, u8)) -> (u8, u8) {
    return ((instruction.0 & 0x0F), instruction.1)
}

// Returns {0: x, 1: y, 2: _}
fn xy_(&instruction: &(u8, u8)) -> (u8, u8, u8) {
    return (instruction.0 & 0x0F, (instruction.1 & 0xF0) >> 4 , instruction.1 & 0x0F)
}

fn xor(base: u64, add: u64) -> (u64, bool) {
    let xored = base ^ add; 
    let ored = base | add;
    return (xored, xored != ored);
}

struct Chip8 {
    // to change
    memory: [u8; MEMORY_SIZE],
    general_registers: [u8; 16],
    address_register: U12,
    // I register
    memory_register: u16,
    program_counter: u16,
    stack_pointer: i8,
    sound_timer: u32,
    delay_timer: u32,
    stack: [u16; 16],
    display: [u64; SCREEN_Y],
    window: PistonWindow,
    gl: GlGraphics,
    events: Events,

}

impl Chip8 {
    fn new() -> Chip8 {
        let mut memory_prepared = [0; 4096];

        // Add sprites to memory (interpreter part)
        for i in 0..SPRITES.len() {
            memory_prepared[i+SPRITE_START] = SPRITES[i];
        }

        // Graphics stuff
        let opengl = OpenGL::V3_2;
        let window: PistonWindow = WindowSettings::new("shapes", [1300, 660])
            .exit_on_esc(true)
            .graphics_api(opengl)
            .build()
            .unwrap();

        let gl = GlGraphics::new(opengl);
        let mut event_settings = EventSettings::new();
        event_settings.max_fps(1);
        event_settings.set_ups(700);
        let events = Events::new(event_settings);

        Chip8 {
            memory: memory_prepared,
            general_registers: [0; 16],
            address_register: u12![0],
            program_counter: PROGRAM_START as u16,
            memory_register: 0,
            sound_timer: 0,
            delay_timer: 0,
            stack_pointer: -1,
            stack: [0; 16],
            display: [0b0; SCREEN_Y],
            window,
            gl,
            events,
        }
    }

    // Loads raw data from a file into the memory
    fn load_from_file(&mut self, path: &str ) {
        let program_bytes = fs::read(path)
            .expect("Couldn't read file");

        for i in 0..program_bytes.len() {
            self.memory[PROGRAM_START + i] = program_bytes[i];
        }
    }

    fn run(&mut self) {
        // Graphics loop
        while let Some(e) = self.events.next(&mut self.window) {
            // Execute a cycle on each update
            if let Some(args) = e.render_args() {
                self.update_display(&args);
            }
            
            // Only render to the screen when wanted
            if let Some(args) = e.update_args() {
                self.execute_cycle(&args);
            }
        }
    }

    // Handle the next instruction
    fn execute_cycle(&mut self, _args: &UpdateArgs) {
        // Get instruction PC points to. They are split in two bytes
        let instruction : (u8, u8) = ((self.memory[self.program_counter as usize]), self.memory[(self.program_counter+1) as usize]);

        // increment to get next instruction next cycle
        self.program_counter += 2;

        // Hack for now, should exit instead
        // Many programs have a loop when finished anyway or will exit
        if self.program_counter >= MEMORY_SIZE as u16 {
            self.program_counter = 0;
        }

        match &instruction.0 >> 4 {
            0x0 => {
                match &instruction.1 {
                    0xE0 => {
                        // println!("Clear display");
                        for i in 0..SCREEN_Y {
                            self.display[i] = 0;
                        }
                    }
                    0xEE => {
                        // println!("Return from subroutine");
                        self.program_counter = self.stack[self.stack_pointer as usize];
                        self.stack_pointer = self.stack_pointer - 1;
                    }
                    _ => {
                        handle_invalid_instruction(&instruction);
                    }
                }
            }
            0x1 => {
                let data = extract_address(&instruction);
                self.program_counter = data;
                // println!("JUMP TO {:X}", data)
            }
            0x2 => {
                let data = extract_address(&instruction);
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.program_counter;
                self.program_counter = data;
                // println!("CALLING SUBROUTING AT {:X}", data)
            }
            0x3 => {
                let (x, k) = xkk(&instruction);
                if self.general_registers[x as usize] == k {
                    self.program_counter += 2;
                }
                // println!("SKIP IF Register {:X} == {:X}", x, k);
            }
            0x4 => {
                let (x, k) = xkk(&instruction);
                if self.general_registers[x as usize] != k {
                    self.program_counter += 2;
                }
                // println!("SKIP IF Register {:X} != {:X}", x, k);
            }
            0x5 => {
                let (x, y, _) = xy_(&instruction);
                if self.general_registers[x as usize] == self.general_registers[y as usize] {
                    self.program_counter += 2;
                }
                // println!("SKIP IF Register {:X} == Register {:X}", x, y);
            }
            0x6 => {
                let (x, k) = xkk(&instruction);
                // println!("SET Register {:X} to {:X}", x, k);
                self.general_registers[x as usize] = k;
            }
            0x7 => {
                let (x, k) = xkk(&instruction);
                // println!("SET Register {} to Register {} ({}) + {}",x, x, self.general_registers[x as usize], k);
                self.general_registers[x as usize] = self.general_registers[x as usize].saturating_add(k);
            }
            0x8 => {
                let (x, y, op) = xy_(&instruction);
                match op {
                    0x0 => {
                        // println!("Copy value in Register {:X} to Register {:X}", x, y);
                        self.general_registers[x as usize] = self.general_registers[y as usize];
                    }
                    0x1 => {
                        // println!("Bitwise OR on Registers {:X} and {:X} and store in {:X}", x, y, x);
                        self.general_registers[x as usize] = self.general_registers[y as usize] | self.general_registers[x as usize];
                    }
                    0x2 => {
                        // println!("Bitwise AND on Registers {:X} and {:X} and store in {:X}", x, y, x);
                        self.general_registers[x as usize] = self.general_registers[y as usize] & self.general_registers[x as usize];
                    }
                    0x3 => {
                        // println!("Bitwise XOR on Registers {:X} and {:X} and store in {:X}", x, y, x);
                        self.general_registers[x as usize] = self.general_registers[y as usize] ^ self.general_registers[x as usize];
                    }
                    0x4 => {
                        // IF value overflows then Register F is set to 1, else 0
                        let reg1 = self.general_registers[x as usize];
                        let reg2 = self.general_registers[y as usize];
                        if (reg1 as u16 + reg2 as u16) > 255 {
                            self.general_registers[0xF] = 1;
                        } else {
                            self.general_registers[0xF] = 0;
                        }

                        self.general_registers[x as usize] = reg1.saturating_add(reg2);
                        // println!("Add values of Registers {:X} and {:X} and store in {:X}", x, y, x);
                    }
                    0x5 => {
                        // If Reg X > Reg Y set Reg F to 1 else 0
                        let reg1 = self.general_registers[x as usize];
                        let reg2 = self.general_registers[y as usize];
                        if reg1 > reg2 {
                            self.general_registers[0xF] = 1;
                        } else {
                            self.general_registers[0xF] = 0;
                        }

                        self.general_registers[x as usize] = reg1.saturating_sub(reg2);
                        // println!("Subtract the value of Register {:X} from {:X} and store in {:X}", y, x, x);
                    }
                    0x6 => {
                        // If least significant bit of Reg X is 1 set Reg F to 1, else 0 
                        // println!("Divide Register {:X} by 2", x);
                        let regx = self.general_registers[x as usize];
                        self.general_registers[0xF] = regx & 1;
                        self.general_registers[x as usize] = regx / 2;
                    }
                    0x7 => {
                        let reg1 = self.general_registers[x as usize];
                        let reg2 = self.general_registers[y as usize];
                        if reg1 > reg2 {
                            self.general_registers[0xF] = 0;
                        } else {
                            self.general_registers[0xF] = 1;
                        }
                        
                        self.general_registers[x as usize] = reg2.saturating_sub(reg1);
                        
                        // If Reg Y > Reg X set Reg F to 1 else 0
                        // println!("Subtract the value of Register {:X} from {:X} and store in {:X}", x, y, x);
                    }
                    0xE => {
                        // If most significant bit of Reg X is 1 set Reg F to 1, else 0 
                        let reg1 = self.general_registers[x as usize];
                        // let reg2 = self.general_registers[y as usize];
                        self.general_registers[0xF] = (reg1 & 0b10000000) >> 7;
                        self.general_registers[x as usize] = reg1 << 1;
                        // println!("Multiply register {:X} by 2", x)
                    }
                    _ => {
                        handle_invalid_instruction(&instruction)
                    }
                }                
            }
            0x9 => {
                let (x, y, _) = xy_(&instruction);
                if self.general_registers[x as usize] != self.general_registers[y as usize] {
                    self.program_counter += 2;
                }
                // println!("Skip next instruction if Reg {:X} != Reg {:X}", x, y);
            }
            0xA => {
                let address = extract_address(&instruction);
                // println!("Set Reg I to {:X}", address);
                self.memory_register = address;
            }
            0xB => {
                let address = extract_address(&instruction);
                self.program_counter = address + self.general_registers[0] as u16
                // println!("Jump to location {:X} + Reg 0", address);
            }
            0xC => {
                let (x, k) = xkk(&instruction);
                let random_byte: u8 = rand::thread_rng().gen::<u8>();
                self.general_registers[x as usize] = random_byte & k;
                // println!("Set Reg {:X} to random byte AND {:b}", x, k);
            }
            0xD => {
                // TODO this will be refactored and cleaned
                // CURRENTLY CUTS OFF DATA
                // Set VF = 1 if a pixel erased else 0
                // Data XORed over screen data
                // Wraps around of coordinates outside of screen 
                let (x, y, n) = xy_(&instruction);
                // println!("Draw sprite of size {:X} stored in Reg I at coords Reg {:X}, Reg {:X}", n, x, y);
                let x_pos = self.general_registers[x as usize];
                let y_pos = self.general_registers[y as usize];
                println!("Draw sprite of size {:X} stored in Reg I at coords  {},  {}", n, self.general_registers[x as usize], self.general_registers[y as usize]);
                /*
                get n rows from memory starting at I position
                draw these over current screen from position (Reg x), (Reg y) XOR
                if part out side of screen wrap round
                */
                for i in 0..n {
                    let sprite_byte = self.memory[(self.memory_register as usize) + (i as usize)];
                    println!("sprite row {:#b}", sprite_byte);
                    // TODO should wrap around if larger then SCREEN_Y
                    let y_offset = y_pos + i;
                    let current_row_data = self.display[y_offset as usize];
                    
                    // determine what bytes will wrap round 
                    println!("{} away from end", SCREEN_X-1 - x_pos);
                    let to_end = SCREEN_X-1 - x_pos;
                    if to_end < 8 {
                        // println!("needs wrap of {:X} bits while {:X} not", 8 - to_end,  );
                        let nowrap = (sprite_byte as u64) >> (8 - to_end) ;
                        // let wrap = (current_row_data << to_end) >> to_end;
                        let wrap = (sprite_byte as u64) << (SCREEN_X-1 - to_end);

                        println!("{:#b}", nowrap);
                        println!("data: {:#b} size: {:X}", wrap, 8- to_end);

                        // xor from end for no wrap and start for wrap
                        let (temp_result, new_hidden) = xor(current_row_data, nowrap);
                        let (result, new_hidden2) = xor(temp_result, wrap);
                        if new_hidden | new_hidden2 {
                            self.general_registers[0xF] = 1;
                        } else {
                            self.general_registers[0xF] = 0;
                        }
                        self.display[y_offset as usize] = result;
                    } else {
                        println!("{:#b} previous row", current_row_data);
                        let positioned_byte = (sprite_byte as u64 )<< (to_end - 8);
                        let (result, has_hidden) = xor(current_row_data, positioned_byte);
                        println!("{:#b} result of XOR", result);
                        self.general_registers[0xF] = has_hidden as u8;
                        self.display[y_offset as usize] = result;
                    }
                }

            }
            0xE => {
                let (x, k) = xkk(&instruction);
                match k {
                    0xA1 => {
                        // Is it value in Reg X or value X?? 
                        println!("Skip instruction if key not pressed with value of register {:X}", x);
                    }
                    0x9E => {
                        // Is it value in Reg X or value X?? 
                        println!("Skip instruction if key pressed with value of register {:X}", x);
                    }
                    _ => {
                        handle_invalid_instruction(&instruction);
                    }
                }
            }
            0xF => {
                let (x, k) = xkk(&instruction);
                match k {
                    0x07 => {
                        println!("Copy value of Delay Timer to Reg {:X}", x)
                    }
                    0x0A => {
                        println!("Wait for key press and store in Reg {:X}", x)
                    }
                    0x15 => {
                        println!("Set Delay timer to value of Reg {:X}", x)
                    }
                    0x18 => {
                        println!("Set sound timer to value of Reg {:X}", x)
                    }
                    0x1E => {
                        println!("Set I to I + Reg {:X}", x)
                    }
                    0x29 => {
                        // Is it value in Reg X or value X?? 
                        // println!("Set I to location of Sprite for digit in Reg {:X}", x);
                        self.memory_register = (SPRITE_START + ( 5 * (x-1)) as usize) as u16;
                    }
                    0x33 => {
                        // The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                        println!("Store BCD representation of Reg {:X} at I, I+1, I+2", x)
                    }
                    0x55 => {
                        println!("Store registers 0 through Reg {:X} in memory starting at location I. ", x)
                    }
                    0x65 => {
                        println!("Load registers 0 through Reg {:X} from memory starting at location I. ", x)
                    }
                    _ => {
                        handle_invalid_instruction(&instruction);
                    }
                }
            }
            _ => {
                handle_invalid_instruction(&instruction)
            }
        }
    }

    // This is called when the screen needs updating
    fn update_display(&mut self, args: &RenderArgs) {
        self.gl.draw(args.viewport(), |c, gl| {
            clear([0.0; 4], gl);
            
            for y_offset in 0..SCREEN_Y {
                let row = self.display[y_offset];
                // println!("{:#b}", row);
                for x_offset in 0..SCREEN_X {
                    let offset = ((SCREEN_X-1)-x_offset) as u32;

                    let mut converted = row & 2_u64.pow(offset);
                    if converted != 0 {
                        converted = converted >> offset;
                    }

                    let c = c.trans((x_offset) as f64 * 20.0, y_offset as f64 * 20.0);
                    let white = [1.0, 1.0, 1.0, 1.0];
                    let black = [0.0, 0.0, 0.0, 1.0];
                    let rect = math::margin_rectangle([20.0; 4], 1.0);
                    let mut colour = black;
                    if converted == 0b1 {
                        colour = white;
                        rectangle(colour, rect, c.transform, gl);
                    } else if SHOW_GRID {
                        rectangle(colour, rect, c.transform, gl);
                        let border_tickness = 0.5;
                        Rectangle::new_border(white, border_tickness).draw(rect, &c.draw_state, c.transform, gl);
                    }
                }
            }
        });
    }
}


fn main() {

    let mut prog = Chip8::new();
    let args: Vec<String> = env::args().collect();
    let program_location = &args[1];
    prog.load_from_file(program_location);
    prog.run();

}