use minifb::*;
use u4::U4;

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
    display: [u8; 2048],
}

fn main() {
    let mut machine = Machine {
        cpu: Cpu {
            bus: Bus::new(),
            stack: Vec::new(),
            pc: 0,
            index: 0,
            delay: 0,
            sound: 0,

            reg: [0; 16],
        },
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
        let instruction = fetch(&mut machine);
        decode(&mut machine, instruction);
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
                machine.display.fill(0);
            } // clear screen
            _ => {}
        },
        U4::Dec01 => {
            machine.cpu.pc = instruction & 0x0FFF;
        }
        U4::Dec06 => {
            machine.cpu.reg[(instruction >> 8 & 0x000F) as usize] = (instruction & 0x00FF) as u8;
        }
        U4::Dec07 => {
            machine.cpu.reg[(instruction >> 8 & 0x000F) as usize] += (instruction & 0x00FF) as u8;
        }
        U4::Dec10 => {
            machine.cpu.index = instruction & 0x0FFF;
        }
        U4::Dec13 => {
            let x = (machine.cpu.reg[(instruction >> 8 & 0x000F) as usize] & 63) as usize;
            let y = (machine.cpu.reg[(instruction >> 4 & 0x000F) as usize] & 31) as usize;
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
        _ => {
            print_instruction(&instruction);
        }
    }
}

pub fn print_instruction(byte: &u16) {
    let mut buffer = [0; 16];

    for i in 0..16 {
        print!("{}", get_bit(byte, i));
    }
    println!();
}

pub fn get_bit(byte: &u16, index: u32) -> u16 {
    return (byte >> index) & 1;
}
