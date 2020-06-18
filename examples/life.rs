#![no_std]

extern crate alloc;

use {
    alloc::boxed::Box,
    anyhow::Error,
    crankstart::{crankstart_game, graphics::Graphics, system::System, Game, Playdate},
    crankstart_sys::{PDButtons_kButtonA, LCD_COLUMNS, LCD_ROWS, LCD_ROWSIZE},
    randomize::PCG32,
};

const LIMIT: usize = (LCD_COLUMNS - 1) as usize;

fn ison(row: &'static [u8], x: usize) -> bool {
    (row[x / 8] & (0x80 >> (x % 8))) == 0
}

fn val(row: &'static [u8], x: usize) -> u8 {
    1 - ((row[x / 8] >> (7 - (x % 8))) & 1)
}

fn rowsum(row: &'static [u8], x: usize) -> u8 {
    if x == 0 {
        val(row, LIMIT) + val(row, x) + val(row, x + 1)
    } else if x < LIMIT {
        return val(row, x - 1) + val(row, x) + val(row, x + 1);
    } else {
        val(row, x - 1) + val(row, x) + val(row, 0)
    }
}

fn middlerowsum(row: &'static [u8], x: usize) -> u8 {
    if x == 0 {
        val(row, LIMIT) + val(row, x + 1)
    } else if x < LIMIT {
        val(row, x - 1) + val(row, x + 1)
    } else {
        val(row, x - 1) + val(row, 0)
    }
}

fn do_row<'a>(
    lastrow: &'static [u8],
    row: &'static [u8],
    nextrow: &'static [u8],
    outrow: &'a mut [u8],
) {
    let mut b = 0;
    let mut bitpos = 0x80;

    for x in 0..(LCD_COLUMNS as usize) {
        // If total is 3 cell is alive
        // If total is 4, no change
        // Else, cell is dead

        let sum = rowsum(lastrow, x) + middlerowsum(row, x) + rowsum(nextrow, x);

        if sum == 3 || (ison(row, x) && sum == 2) {
            b |= bitpos;
        }

        bitpos >>= 1;

        if bitpos == 0 {
            outrow[x / 8] = !b;
            b = 0;
            bitpos = 0x80;
        }
    }
}

fn randomize(graphics: &Graphics, rng: &mut PCG32) -> Result<(), Error> {
    let frame = graphics.get_display_frame()?;
    let start = 0;
    for element in &mut frame[start..] {
        *element = rng.next_u32() as u8;
    }
    Ok(())
}

struct Life {
    graphics: Graphics,
    system: System,
    rng: PCG32,
    started: bool,
}

const LAST_ROW_INDEX: usize = ((LCD_ROWS - 1) * LCD_ROWSIZE) as usize;
const LAST_ROW_LIMIT: usize = LAST_ROW_INDEX + LCD_ROWSIZE as usize;

impl Life {
    pub fn new(playdate: &Playdate) -> Result<Box<Self>, Error> {
        let rng0 = PCG32::seed(1, 1);
        Ok(Box::new(Self {
            graphics: playdate.graphics(),
            system: playdate.system(),
            rng: rng0,
            started: false,
        }))
    }
}

impl Game for Life {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        if !self.started {
            randomize(&self.graphics, &mut self.rng)?;
            self.started = true;
        }

        let (_, pushed, _) = self.system.get_button_state()?;

        if (pushed & PDButtons_kButtonA) != 0 {
            randomize(&self.graphics, &mut self.rng)?;
        }

        let frame = self.graphics.get_frame()?;

        let last_frame = self.graphics.get_display_frame()?;
        let mut last_row = &last_frame[LAST_ROW_INDEX..LAST_ROW_LIMIT];
        let mut row = &last_frame[0..LCD_ROWSIZE as usize];
        let mut next_row = &last_frame[LCD_ROWSIZE as usize..(LCD_ROWSIZE * 2) as usize];
        for y in 0..LCD_ROWS {
            let index = (y * LCD_ROWSIZE) as usize;
            let limit = index + LCD_ROWSIZE as usize;
            do_row(last_row, row, next_row, &mut frame[index..limit]);

            last_row = row;
            row = next_row;
            let next_row_index = (y + 2) % LCD_ROWS;

            let index = (next_row_index * LCD_ROWSIZE) as usize;
            let limit = index + LCD_ROWSIZE as usize;
            next_row = &last_frame[index..limit];
        }

        self.graphics.mark_updated_rows(-1, -1)?;

        Ok(())
    }
}

crankstart_game!(Life);
