
pub mod chip8 {
    struct Colour;
    impl Colour {
        pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
    }

    use crate::display::display::{Display, Pixel};
    extern crate piston_window;
    use piston_window::*;
    extern crate opengl_graphics;
    use opengl_graphics::{GlGraphics, OpenGL};
    use std::collections::HashMap;
    use std::fs;

    const MEMORY_OFFSET: u16 = 512;
    const SPRITE_START: usize = 0;
    const PROGRAM_START: usize = 512;
    const INSTRUCTIONS_PER_SECOND: u16 = 700;
    const MEMORY_SIZE: usize = 4096;

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
    
    pub struct Chip8 {
        memory: [u8; 4096],
        display: Display,
        window: PistonWindow,
        gl: GlGraphics,
        events: Events,
        keys_map: HashMap<Key, bool>,

        general_registers: [u8; 16],
        program_counter: u16,
        memory_register: u16,
        address_register: u16,

        stack_pointer: i8,
        stack: [u16; 16],
        sound_timer: u32,
        delay_timer: u32,
    }
    
    impl Chip8 {
        pub fn new() -> Chip8 {
            // Graphics
            let opengl = OpenGL::V3_2;
            let window: PistonWindow = WindowSettings::new("Chip8", [1300, 660])
                .graphics_api(opengl)
                .exit_on_esc(true)
                .build()
                .unwrap();
    
            let gl = GlGraphics::new(opengl);
            let mut event_settings = EventSettings::new();
            event_settings.max_fps(1);
            event_settings.set_ups(50);
            let events = Events::new(event_settings);

            let mut keysMap: HashMap<Key, bool> = HashMap::new();
            
            let mut memory_prepared = [0; 4096];

            // Load sprites to memory (in the interpreter part)
            for i in 0..SPRITES.len() {
                memory_prepared[i+SPRITE_START] = SPRITES[i];
            }

            Chip8 {
                display: Display::new(),
                window,
                gl,
                events,
                keys_map: keysMap,
                memory: memory_prepared,

                memory_register: 0,
                general_registers: [0; 16],
                program_counter: PROGRAM_START as u16,
                address_register: 0,

                sound_timer: 0,
                stack: [0; 16],    
                delay_timer: 0,
                stack_pointer: -1,
            }
        }
    
        // TODO: Display screen array in window
        fn render_screen(&mut self, &args: &RenderArgs) {
            self.gl.draw(args.viewport(), |c, gl| {
                // Wipe screen
                clear([0.0; 4], gl);
                
                // for each row
                for y_offset in 0..32 {
                    for x_offset in 0..64 {
                        let pixel = self.display.get_pixel(x_offset, y_offset);
    
                        // Position pixel
                        let c = c.trans((x_offset) as f64 * 20.0, y_offset as f64 * 20.0);
                        let rect = math::margin_rectangle([20.0; 4], 1.0);

                        // Default to black
                        let mut colour = Colour::BLACK;

                        if pixel == Pixel::White {
                            colour = Colour::WHITE;
                            rectangle(colour, rect, c.transform, gl);
                        } else if SHOW_GRID {
                            rectangle(colour, rect, c.transform, gl);
                            let border_tickness = 0.5;
                            Rectangle::new_border(Colour::WHITE, border_tickness).draw(rect, &c.draw_state, c.transform, gl);
                        }
                    }
                }
            });
        }

        // TODO: handle one cpu cycle
        fn update(&mut self, &args: &UpdateArgs) {
            // Get instruction PC points to. They are split in two bytes
            let instruction : (u8, u8) = ((self.memory[self.program_counter as usize]), self.memory[(self.program_counter+1) as usize]);

            // increment to get next instruction next cycle
            self.program_counter += 2;

            // Catch PC overflow
            if self.program_counter >= MEMORY_SIZE as u16 {
                self.program_counter = 0;
            }

            let operand = (&instruction.0 & 0xF0) >> 4;
            match operand {
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
                            // println!("Skip instruction if key not pressed with value of register {:X}", x);
                            let keyIn = self.general_registers[x as usize];
                            // convert to key enum
                            if !self.isKeyPressed(keyToEnum(keyIn)) {
                                self.skipNext()
                            }
                            
                        }
                        0x9E => {
                            // println!("Skip instruction if key pressed with value of register {:X}", x);
                            let keyIn = self.general_registers[x as usize];
                            // convert to key enum
                            if self.isKeyPressed(keyToEnum(keyIn)) {
                                self.skipNext()
                            }
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
                            println!("Copy value of Delay Timer to Reg {:X}", x);
                        }
                        0x0A => {
                            println!("Wait for key press and store in Reg {:X}", x);
                            let mut has_key_pressed = false;
                            let mut key_pressed: Key = Key::Z;
                            for (key, value) in &self.keys_map {
                                if *value == true {
                                    has_key_pressed = true;
                                    key_pressed = *key;
                                }
                            }
                            if has_key_pressed {
                                self.general_registers[x as usize] = keyToHex(key_pressed);
                            } else {
                                self.program_counter -= 2;
                            }
                        }
                        0x15 => {
                            println!("Set Delay timer to value of Reg {:X}", x)
                        }
                        0x18 => {
                            println!("Set sound timer to value of Reg {:X}", x)
                        }
                        0x1E => {
                            // println!("Set I to I + Reg {:X}", x)
                            self.memory_register = self.memory_register + self.general_registers[x as usize] as u16;
                        }
                        0x29 => {
                            // Is it value in Reg X or value X?? 
                            println!("Getting sprite {:?}", x);
                            // println!("Set I to location of Sprite for digit in Reg {:X}", x);
                            self.memory_register = (SPRITE_START + ( 5 * (x-1)) as usize) as u16;
                        }
                        0x33 => {
                            // The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I, the tens digit at location I+1, and the ones digit at location I+2.
                            println!("Store BCD representation of Reg {:X} at I, I+1, I+2", x)
                            
                        }
                        0x55 => {
                            println!("Store registers 0 through Reg {:X} in memory starting at location I. ", x);
                            for i in 0..x+1 {
                                let reg_value = self.general_registers[i as usize];
                                self.memory[self.memory_register as usize] = reg_value;
                            }
                        }
                        0x65 => {
                            println!("Load registers 0 through Reg {:X} from memory starting at location I. ", x);
                            for i in 0..x+1 {
                                let memory_value = self.memory[self.memory_register as usize];
                                self.general_registers[i as usize] = memory_value;
                            }
                        }
                        _ => {
                            handle_invalid_instruction(&instruction);
                        }
                    }
                }
                _ => println!("Unknown instruction: {:X}{:X}", instruction.0, instruction.1),
            }
        }

        pub fn load_from_file(&mut self, file_location: &str) {
            let program_bytes = fs::read(file_location)
            .expect("Couldn't read file");

            for i in 0..program_bytes.len() {
                self.memory[PROGRAM_START + i] = program_bytes[i];
            }
        }
    
        pub fn run(&mut self) {
            while let Some(e) = self.events.next(&mut self.window) {

                // Key press handling
                if let Some(Button::Keyboard(key)) = e.press_args() {
    
                    self.keys_map.insert(key, true);
    
                    println!("Pressed keyboard key '{:?}'", key);
                };
                if let Some(button) = e.release_args() {
                    match button {
                        Button::Keyboard(key) => {
                            self.keys_map.remove(&key);
                            println!("Released keyboard key '{:?}'", key)
                        },
                        Button::Mouse(button) => println!("Released mouse button '{:?}'", button),
                        Button::Controller(button) => println!("Released controller button '{:?}'", button),
                        Button::Hat(hat) => println!("Released controller hat `{:?}`", hat),
                    }
                };
    
                // Execute a cycle on each update
                if let Some(args) = e.render_args() {
                    self.render_screen(&args);
                }
                
                // Only render to the screen when wanted
                if let Some(args) = e.update_args() {
                    self.update(&args);
                }
            }
        }
    }
}

