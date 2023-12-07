use super::{Interrupt, InterruptMask, InterruptSource};

const OAM_SLOTS: usize = 40;
const LCD_X: usize = 160;
const LCD_Y: usize = 144;
const CYCLES_PER_LINE: u64 = 456;
const CYCLES_PER_FRAME: u64 = 70224;
const VRAM_BASE: u16 = 0x8000;
const BG_WIN_ALT_BASE: u16 = 0x8800;

struct OamEntry {
    y: u8,
    x: u8,
    idx: u8,
    flags: u8,
}

impl OamEntry {
    fn below_bg(&self) -> bool {
        self.flags & 0b1000_0000 != 0
    }

    fn flip_y(&self) -> bool {
        self.flags & 0b0100_0000 != 0
    }

    fn flip_x(&self) -> bool {
        self.flags & 0b0010_0000 != 0
    }

    fn obp1(&self) -> bool {
        self.flags & 0b0001_0000 != 0
    }
}

#[derive(Clone, Copy)]
struct Lcdc(u8);

impl Lcdc {
    fn bit(self, n: u8) -> bool {
        self.0 & (1 << n) != 0
    }

    fn lcd_enable(self) -> bool {
        self.bit(7)
    }

    fn window_tile_map_base(self) -> u16 {
        if self.bit(6) {
            0x9c00
        } else {
            0x9800
        }
    }

    fn window_enable(self) -> bool {
        self.bit(5)
    }

    fn bg_win_tile_base(self) -> u16 {
        if self.bit(4) {
            0x8000
        } else {
            0x8800
        }
    }

    fn background_tile_map_base(self) -> u16 {
        if self.bit(3) {
            0x9c00
        } else {
            0x9800
        }
    }

    fn obj_size(self) -> u8 {
        if self.bit(2) {
            16
        } else {
            8
        }
    }

    fn obj_enable(self) -> bool {
        self.bit(1)
    }

    fn bw_win_enable(self) -> bool {
        self.bit(0)
    }
}

#[derive(PartialEq)]
enum Mode {
    HBlank,
    VBlank,
    Drawing,
    OamScan,
}

pub struct Video {
    framebuffer: [u8; LCD_X * LCD_Y],
    enable_cycle: u64,
    render_cycle: u64,
    lcdc: Lcdc,
    scy: u8,
    scx: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    video_ram: [u8; 8192],
    oam: [u8; OAM_SLOTS * 4],
    irq_vblank_pending: bool,
    irq_stat_pending: bool,
    irq_acknowledge_cycle: u64,
}

impl Video {
    pub(crate) fn new() -> Self {
        Self {
            framebuffer: [0u8; LCD_X * LCD_Y],
            render_cycle: 0,
            enable_cycle: 0,
            lcdc: Lcdc(0b1000_0000),
            scy: 0,
            scx: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            video_ram: [0u8; 8192],
            oam: [0u8; OAM_SLOTS * 4],
            irq_vblank_pending: false,
            irq_stat_pending: false,
            irq_acknowledge_cycle: 0,
        }
    }

    pub(crate) fn framebuffer(&mut self, cycle: u64) -> &[u8] {
        self.render_until(cycle);
        &self.framebuffer
    }

    fn cycle_in_frame(&self, cycle: u64) -> u64 {
        cycle.saturating_sub(self.enable_cycle) % CYCLES_PER_FRAME
    }

    fn frame(&self, cycle: u64) -> u64 {
        cycle.saturating_sub(self.enable_cycle) / CYCLES_PER_FRAME
    }

    fn cycle_in_line(&self, cycle: u64) -> u64 {
        self.cycle_in_frame(cycle) % CYCLES_PER_LINE
    }

    fn line(&self, cycle: u64) -> u8 {
        (self.cycle_in_frame(cycle) / CYCLES_PER_LINE) as u8
    }

    fn mode(&self, cycle: u64) -> Mode {
        let cycle_in_line = self.cycle_in_line(cycle);
        let line = self.line(cycle);

        if line >= (LCD_Y as _) {
            Mode::VBlank
        } else if cycle_in_line >= 252 {
            Mode::HBlank
        } else if cycle_in_line >= 80 {
            Mode::Drawing
        } else {
            Mode::OamScan
        }
    }

    fn oam_entry(&self, idx: u8) -> OamEntry {
        let base = (idx as usize) * 4;

        OamEntry {
            y: self.oam[base],
            x: self.oam[base + 1],
            idx: self.oam[base + 2],
            flags: self.oam[base + 3],
        }
    }

    fn video_ram_read(&self, addr: u16) -> u8 {
        self.video_ram[(addr - VRAM_BASE) as usize]
    }

    fn get_bg_win_tile_row(&self, idx: u8, row: u8) -> (u8, u8) {
        let tile_data_base = self.lcdc.bg_win_tile_base();

        let tile_data_addr = match tile_data_base {
            VRAM_BASE => {
                let idx = idx as u16;
                tile_data_base + idx * 16 + (row as u16) * 2
            }
            BG_WIN_ALT_BASE => {
                let idx = (idx as i8) as i16;
                0x9000u16.wrapping_add_signed(idx * 16) + (row as u16) * 2
            }
            _ => panic!(),
        };

        let tile_data_l = self.video_ram_read(tile_data_addr);
        let tile_data_h = self.video_ram_read(tile_data_addr + 1);

        (tile_data_l, tile_data_h)
    }

    fn get_obj_tile_row(&self, idx: u8, row: u8) -> (u8, u8) {
        let tile_data_addr = 0x8000 + (idx as u16) * 16 + (row as u16) * 2;

        let tile_data_l = self.video_ram_read(tile_data_addr);
        let tile_data_h = self.video_ram_read(tile_data_addr + 1);

        (tile_data_l, tile_data_h)
    }

    fn draw_background_line(&mut self) {
        let lcd_y = self.line(self.render_cycle);
        let tile_map_base = self.lcdc.background_tile_map_base();

        let scrolled_y = lcd_y.wrapping_add(self.scy);
        let tile_y = scrolled_y / 8;
        let in_tile_y = scrolled_y % 8;

        for lcd_x in 0u8..160 {
            let scrolled_x = lcd_x.wrapping_add(self.scx);
            let tile_x = scrolled_x / 8;
            let in_tile_x = scrolled_x % 8;

            let tile_map_addr = (tile_y as u16) * 32 + (tile_x as u16);
            let tile_data_idx = self.video_ram_read(tile_map_base + tile_map_addr);

            let (tile_data_l, tile_data_h) = self.get_bg_win_tile_row(tile_data_idx, in_tile_y);

            let bit_l = (tile_data_l << in_tile_x) & 0b1000_0000 != 0;
            let bit_h = (tile_data_h << in_tile_x) & 0b1000_0000 != 0;

            let pal_idx = (bit_h as u8) << 1 | (bit_l as u8);
            let val = (self.bgp >> (pal_idx * 2)) & 0b0000_0011;

            let idx = (lcd_y as usize) * 160 + lcd_x as usize;

            self.framebuffer[idx] = val;
        }
    }

    fn draw_window_line(&mut self) {
        let lcd_y = self.line(self.render_cycle);
        let tile_map_base = self.lcdc.window_tile_map_base();

        let window_y = (lcd_y as i16) - (self.wy as i16);

        if window_y < 0 {
            return;
        }

        let window_y = window_y as u8;
        let tile_y = window_y / 8;
        let in_tile_y = window_y % 8;

        for lcd_x in 0..(LCD_X as u8) {
            let window_x = (lcd_x as i16) - (self.wx as i16) + 7;

            if window_x < 0 {
                continue;
            }

            let window_x = window_x as u8;
            let tile_x = window_x / 8;
            let in_tile_x = window_x % 8;

            let tile_map_addr = (tile_y as u16) * 32 + (tile_x as u16);
            let tile_data_idx = self.video_ram_read(tile_map_base + tile_map_addr);

            let (tile_data_l, tile_data_h) = self.get_bg_win_tile_row(tile_data_idx, in_tile_y);

            let bit_l = (tile_data_l << in_tile_x) & 0b1000_0000 != 0;
            let bit_h = (tile_data_h << in_tile_x) & 0b1000_0000 != 0;

            let pal_idx = (bit_h as u8) << 1 | (bit_l as u8);
            let val = (self.bgp >> (pal_idx * 2)) & 0b0000_0011;

            let idx = (lcd_y as usize) * LCD_X + lcd_x as usize;

            self.framebuffer[idx] = val;
        }
    }

    fn draw_obj_line(&mut self) {
        assert!(self.lcdc.obj_size() == 8);

        let lcd_y = self.line(self.render_cycle);

        for idx in 0..(OAM_SLOTS as _) {
            let obj = self.oam_entry(idx);

            let in_obj_y = (lcd_y as i16) - (obj.y as i16) + 16;

            if !(0..8).contains(&in_obj_y) {
                continue;
            }

            let in_obj_y = match obj.flip_y() {
                true => (7 - in_obj_y) as u8,
                false => in_obj_y as u8,
            };

            let (tile_data_l, tile_data_h) = self.get_obj_tile_row(obj.idx, in_obj_y);

            let pal = match obj.obp1() {
                true => self.obp1,
                false => self.obp0,
            };

            for in_obj_x in 0..8 {
                let lcd_x = in_obj_x + (obj.x as i16) - 8;

                if !(0..(LCD_X as _)).contains(&lcd_x) {
                    continue;
                }

                let in_obj_x = match obj.flip_x() {
                    true => (7 - in_obj_x) as u8,
                    false => in_obj_x as u8,
                };

                let bit_l = (tile_data_l.wrapping_shl(in_obj_x as u32)) & 0b1000_0000 != 0;
                let bit_h = (tile_data_h.wrapping_shl(in_obj_x as u32)) & 0b1000_0000 != 0;

                let pal_idx = (bit_h as u8) << 1 | (bit_l as u8);

                if pal_idx != 0 {
                    let val = (pal >> (pal_idx * 2)) & 0b0000_0011;

                    let idx = (lcd_y as usize) * 160 + lcd_x as usize;

                    // TODO: This background priority implementation is not correct,
                    // as overlapping objects also trigger it, but ... eh.
                    let bg_recessive = self.framebuffer[idx] == self.bgp & 0b0000_0011;

                    if !obj.below_bg() || bg_recessive {
                        self.framebuffer[idx] = val;
                    }
                }
            }
        }
    }

    fn draw_line(&mut self) {
        if self.lcdc.bw_win_enable() {
            self.draw_background_line();

            if self.lcdc.window_enable() {
                self.draw_window_line();
            }
        }

        if self.lcdc.obj_enable() {
            self.draw_obj_line();
        }
    }

    fn render_until(&mut self, cycle: u64) {
        while self.render_cycle.saturating_add(CYCLES_PER_LINE) < cycle {
            assert!(self.cycle_in_line(self.render_cycle) == 0);
            assert!(self.line(self.render_cycle) < (LCD_Y as _));

            self.draw_line();

            if self.line(self.render_cycle) + 1 == (LCD_Y as _) {
                self.render_cycle += 11 * CYCLES_PER_LINE;
            } else {
                self.render_cycle += CYCLES_PER_LINE;
            }
        }
    }

    pub(super) fn read(&self, cycle: u64, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9fff => {
                let offset = (addr as usize) - 0x8000;
                self.video_ram[offset]
            }
            0xfe00..=0xfe9f => {
                let offset = (addr as usize) - 0xfe00;
                self.oam[offset]
            }
            0xff40 => self.lcdc.0,
            0xff41 => {
                let lym = self.lyc == self.line(cycle);
                let mode = self.mode(cycle) as u8;

                (lym as u8) << 2 | mode
            }
            0xff42 => self.scy,
            0xff43 => self.scx,
            0xff44 => self.line(cycle),
            0xff45 => self.lyc,
            0xff46 => {
                panic!("OAM DMA should be handled at the peripherial level");
            }
            0xff47 => self.bgp,
            0xff48 => self.obp0,
            0xff49 => self.obp1,
            0xff4a => self.wy,
            0xff4b => self.wx,
            _ => panic!("Address {addr} is not in video range"),
        }
    }

    pub(super) fn write(&mut self, cycle: u64, addr: u16, val: u8) {
        self.render_until(cycle);

        match addr {
            0x8000..=0x9fff => {
                let offset = (addr as usize) - 0x8000;
                self.video_ram[offset] = val;
            }
            0xfe00..=0xfe9f => {
                let offset = (addr as usize) - 0xfe00;
                self.oam[offset] = val;
            }
            0xff40 => {
                let en_pre = self.lcdc.lcd_enable();
                self.lcdc = Lcdc(val);
                let en_post = self.lcdc.lcd_enable();

                if !en_pre && en_post {
                    self.enable_cycle = cycle;
                    self.render_cycle = cycle;
                }

                if en_pre && !en_post {
                    self.enable_cycle = u64::MAX;
                    self.render_cycle = u64::MAX;
                }
            }
            0xff41 => {
                if val != 0 {
                    // TODO: if implemented also needs changes in read_u8
                    unimplemented!("writing STAT != 0 (e.g. 0x{val:02x})")
                }
            }
            0xff42 => {
                self.scy = val;
            }
            0xff43 => {
                self.scx = val;
            }
            0xff44 => {}
            0xff45 => {
                self.lyc = val;
            }
            0xff46 => {
                panic!("OAM DMA should be handled at the peripherial level");
            }
            0xff47 => {
                self.bgp = val;
            }
            0xff48 => {
                self.obp0 = val;
            }
            0xff49 => {
                self.obp1 = val;
            }
            0xff4a => {
                self.wy = val;
            }
            0xff4b => {
                self.wx = val;
            }
            _ => panic!("Address {addr} is not in video range"),
        }
    }
}

impl InterruptSource for Video {
    fn pending(&self, cycle: u64) -> InterruptMask {
        let mut res = InterruptMask::default();

        if self.irq_vblank_pending || cycle >= self.next_pending(cycle) {
            res.set(Interrupt::VBlank)
        }

        if self.irq_stat_pending {
            res.set(Interrupt::Lcd)
        }

        res
    }

    fn set_pending(&mut self, cycle: u64, mask: InterruptMask) {
        self.irq_vblank_pending = mask.is_set(Interrupt::VBlank);
        self.irq_stat_pending = mask.is_set(Interrupt::Lcd);

        if !self.irq_vblank_pending {
            self.irq_acknowledge_cycle = cycle;
        }
    }

    fn next_pending(&self, cycle: u64) -> u64 {
        if self.enable_cycle == u64::MAX {
            return u64::MAX;
        }

        let in_vblank = self.mode(cycle) == Mode::VBlank;
        let vblank_acknowledged = self.frame(cycle) == self.frame(self.irq_acknowledge_cycle);

        if in_vblank && !vblank_acknowledged {
            return cycle;
        }

        let vblank_in_frame = (LCD_Y as u64) * CYCLES_PER_LINE;
        let vblank_in_next_frame = vblank_in_frame + CYCLES_PER_FRAME;
        let cycles_till_vblank = vblank_in_next_frame - self.cycle_in_frame(cycle);
        let ctv_wrapped = cycles_till_vblank % CYCLES_PER_FRAME;

        cycle + ctv_wrapped
    }
}
