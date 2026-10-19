use minifb::*;
use rodio::{DeviceSinkBuilder, Player, source::SquareWave};
use u4::U4;

const SHIFT_ORIGINAL: bool = false;
const JUMP_ORIGINAL: bool = false;
const STORE_LOAD_ORIGINAL: bool = false;

static FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

const KEYMAP: [Key; 16] = [
    Key::X,
    Key::Key1,
    Key::Key2,
    Key::Key3,
    Key::Q,
    Key::W,
    Key::E,
    Key::A,
    Key::S,
    Key::D,
    Key::Z,
    Key::C,
    Key::Key4,
    Key::R,
    Key::F,
    Key::V,
];

static ROM: &[u8] = include_bytes!("../IBM.rom");

pub struct Bus {
    ram: Vec<u8>,
}

impl Bus {
    pub fn new() -> Self {
        Self { ram: vec![0; 4096] }
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
    }
}

pub struct Cpu {
    bus: Bus,
    stack: Vec<u16>,
    pc: u16,
    index: u16,
    delay: u8,
    sound: u8,

    reg: [u8; 16],
}

pub struct Machine {
    cpu: Cpu,
    keys: [u8; 16],
    display: [u8; 2048],
}

fn main() {
    let handle = DeviceSinkBuilder::open_default_sink().expect("open default audio");
    let player = Player::connect_new(handle.mixer());
    player.append(SquareWave::new(440.0));
    player.set_volume(0.2);
    player.pause();

    let mut machine = Machine {
        cpu: Cpu {
            bus: Bus::new(),
            stack: Vec::new(),
            pc: 0x200,
            index: 0,
            delay: 0,
            sound: 0,

            reg: [0; 16],
        },
        keys: [0; 16],
        display: [0; 2048],
    };

    for i in 0..FONT.len() {
        machine.cpu.bus.ram[0x050 + i] = FONT[i];
    }
    for i in 0..ROM.len() {
        machine.cpu.bus.ram[0x200 + i] = ROM[i];
    }

    let mut window = match Window::new("Test", 640, 400, WindowOptions::default()) {
        Ok(win) => win,
        Err(err) => {
            println!("Unable to create window {}", err);
            return;
        }
    };

    window.set_target_fps(60);
    window.set_title("Chip 8 Emulator");

    while window.is_open() {
        machine.cpu.delay -= if machine.cpu.delay > 0 { 1 } else { 0 };
        machine.cpu.sound -= if machine.cpu.sound > 0 {
            player.play();
            1
        } else {
            player.pause();
            0
        };

        for i in 0..16 {
            machine.keys[i] = window.is_key_down(KEYMAP[i]) as u8;
        }

        for _ in 0..12 {
            let instruction = fetch(&mut machine);
            decode(&mut machine, instruction);
        }

        let buffer: Vec<u32> = machine
            .display
            .iter()
            .map(|&p| if p == 1 { 0xFFFFFF } else { 0x000000 })
            .collect();

        window.update_with_buffer(&buffer, 64, 32).unwrap();
    }
}

fn fetch(machine: &mut Machine) -> u16 {
    let r = u16::from_be_bytes(
        machine.cpu.bus.ram[machine.cpu.pc as usize..machine.cpu.pc as usize + 2]
            .try_into()
            .unwrap(),
    );

    machine.cpu.pc += 2;

    return r;
}

fn decode(machine: &mut Machine, instruction: u16) {
    let first4 = U4::new((instruction >> 12) as u8).unwrap();

    match first4 {
        U4::Dec00 => match instruction {
            0x00e0 => {
                // clear screen
                machine.display.fill(0);
            }
            0x00ee => {
                // return pc from stack
                machine.cpu.pc = machine.cpu.stack.pop().unwrap();
            }
            _ => {
                println!("{:04X}", instruction);
            }
        },
        U4::Dec01 => {
            machine.cpu.pc = instruction & 0x0FFF; // jump
        }
        U4::Dec02 => {
            machine.cpu.stack.push(machine.cpu.pc);
            machine.cpu.pc = instruction & 0x0FFF; // jump and return
        }
        U4::Dec03 => {
            if get_registervx(machine, &instruction) == instruction as u8 {
                // skip if equal 3xnn
                machine.cpu.pc += 2;
            }
        }
        U4::Dec04 => {
            if get_registervx(machine, &instruction) != instruction as u8 {
                // skip if not equal 4xnn
                machine.cpu.pc += 2;
            }
        }
        U4::Dec05 => {
            if get_registervx(machine, &instruction) == get_registervy(machine, &instruction) {
                // skip if equal
                machine.cpu.pc += 2;
            }
        }
        U4::Dec06 => {
            set_registervx(machine, &instruction, (instruction & 0x00FF) as u8); // set register vx
        }
        U4::Dec07 => {
            let val =
                get_registervx(machine, &instruction).wrapping_add(instruction as u8 & 0x00FF); // add value to register
            set_registervx(machine, &instruction, val); // set value
        }
        U4::Dec08 => {
            let last = instruction as u8 & 0x0F;
            binary_operations(machine, &instruction, last);
        }
        U4::Dec09 => {
            if get_registervx(machine, &instruction) != get_registervy(machine, &instruction) {
                // skip if not equal
                machine.cpu.pc += 2;
            }
        }
        U4::Dec10 => {
            machine.cpu.index = instruction & 0x0FFF; // set index register
        }
        U4::Dec11 => {
            if JUMP_ORIGINAL {
                machine.cpu.pc = (instruction & 0x0FFF) + machine.cpu.reg[0] as u16;
            } else {
                machine.cpu.pc =
                    (instruction & 0x0FFF) + get_registervx(machine, &instruction) as u16;
            }
            machine.cpu.pc = machine.cpu.pc & 0x0fff;
        }
        U4::Dec12 => {
            set_registervx(
                machine,
                &instruction,
                rand::random::<u8>() & instruction as u8,
            );
        }
        U4::Dec13 => {
            // draw
            let x = (get_registervx(machine, &instruction) & 63) as usize;
            let y = (get_registervy(machine, &instruction) & 31) as usize;
            machine.cpu.reg[15] = 0;

            let n = instruction & 0x000F;
            for j in 0..n {
                let nth: u8 = machine.cpu.bus.read(machine.cpu.index + j);
                let row = y + j as usize;

                for i in (0..8).rev() {
                    if (1 << i) & nth != 0 {
                        let col = x + (7 - i);
                        if col >= 64 || row >= 32 {
                            break;
                        }
                        if machine.display[col + row * 64] == 1 {
                            machine.cpu.reg[15] = 1
                        }
                        machine.display[col + row * 64] ^= 1;
                    }
                }
            }
        }
        U4::Dec14 => {
            let vx = get_registervx(machine, &instruction);
            let lower = instruction as u8;
            match lower {
                0x9e => {
                    if machine.keys[(vx & 0xf) as usize] == 1 {
                        machine.cpu.pc += 2;
                    }
                }
                0xa1 => {
                    if machine.keys[(vx & 0xf) as usize] != 1 {
                        machine.cpu.pc += 2;
                    }
                }
                _ => {}
            }
        }
        U4::Dec15 => {
            let lower = instruction as u8;
            match lower {
                0x07 => {
                    set_registervx(machine, &instruction, machine.cpu.delay);
                }
                0x15 => {
                    machine.cpu.delay = get_registervx(machine, &instruction);
                }
                0x18 => {
                    machine.cpu.sound = get_registervx(machine, &instruction);
                }
                0x1e => {
                    machine.cpu.index += get_registervx(machine, &instruction) as u16;
                    if machine.cpu.index > 0x0FFF {
                        machine.cpu.reg[15] = 1;
                    }

                    machine.cpu.index = machine.cpu.index & 0x0fff;
                }
                0x0a => {
                    let mut pressed = false;
                    for i in 0..machine.keys.len() as u8 {
                        if machine.keys[i as usize] == 1 {
                            set_registervx(machine, &instruction, i);
                            pressed = true;
                        }
                    }
                    if !pressed {
                        machine.cpu.pc -= 2;
                    }
                }
                0x29 => {
                    machine.cpu.index =
                        0x50 + (get_registervx(machine, &instruction) & 0xf) as u16 * 5;
                }
                0x33 => {
                    let mut vx = get_registervx(machine, &instruction);
                    machine.cpu.bus.write(machine.cpu.index + 2, vx % 10);
                    vx /= 10;
                    machine.cpu.bus.write(machine.cpu.index + 1, vx % 10);
                    vx /= 10;
                    machine.cpu.bus.write(machine.cpu.index + 0, vx % 10);
                }
                0x55 => {
                    let vx = instruction >> 8 & 0x000F;
                    for i in 0..=vx {
                        machine
                            .cpu
                            .bus
                            .write(machine.cpu.index + i as u16, machine.cpu.reg[i as usize]);
                    }
                    if STORE_LOAD_ORIGINAL {
                        machine.cpu.index += vx as u16 + 1;
                    }
                }
                0x65 => {
                    let vx = instruction >> 8 & 0x000F;
                    for i in 0..=vx {
                        machine.cpu.reg[i as usize] =
                            machine.cpu.bus.read(machine.cpu.index + i as u16);
                    }
                    if STORE_LOAD_ORIGINAL {
                        machine.cpu.index += vx as u16 + 1;
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn get_registervx(machine: &mut Machine, instruction: &u16) -> u8 {
    machine.cpu.reg[(instruction >> 8 & 0x000F) as usize]
}

pub fn set_registervx(machine: &mut Machine, instruction: &u16, val: u8) {
    machine.cpu.reg[(instruction >> 8 & 0x000F) as usize] = val;
}

pub fn get_registervy(machine: &mut Machine, instruction: &u16) -> u8 {
    machine.cpu.reg[(instruction >> 4 & 0x000F) as usize]
}

pub fn set_registervy(machine: &mut Machine, instruction: &u16, val: u8) {
    machine.cpu.reg[(instruction >> 4 & 0x000F) as usize] = val;
}

pub fn binary_operations(machine: &mut Machine, instruction: &u16, last: u8) {
    let mut vx = get_registervx(machine, instruction);
    let vy = get_registervy(machine, instruction);
    match last {
        0 => {
            set_registervx(machine, instruction, vx);
        }

        1 => {
            set_registervx(machine, instruction, vx | vy);
        }
        2 => {
            set_registervx(machine, instruction, vx & vy);
        }
        3 => {
            set_registervx(machine, instruction, vx ^ vy);
        }
        4 => {
            let (val, over) = vx.overflowing_add(vy);
            set_registervx(machine, instruction, val);
            machine.cpu.reg[15] = over as u8;
        }
        5 => {
            let (val, over) = vx.overflowing_sub(vy);
            set_registervx(machine, instruction, val);
            machine.cpu.reg[15] = !over as u8;
        }
        6 => {
            if SHIFT_ORIGINAL {
                vx = get_registervy(machine, instruction);
            }
            set_registervx(machine, instruction, vx >> 1);
            machine.cpu.reg[15] = vx & 1;
        }
        7 => {
            let (val, over) = vy.overflowing_sub(vx);
            set_registervx(machine, instruction, val);
            machine.cpu.reg[15] = !over as u8;
        }
        14 => {
            if SHIFT_ORIGINAL {
                vx = get_registervy(machine, instruction);
            }
            set_registervx(machine, instruction, vx << 1);
            machine.cpu.reg[15] = (vx >> 7) & 1;
        }
        _ => {}
    }
}

pub fn print_instruction(byte: &u16) {
    for i in 0..16 {
        print!("{}", get_bit(byte, i));
    }
    println!();
}

pub fn get_bit(byte: &u16, index: u32) -> u16 {
    return (byte >> index) & 1;
}
