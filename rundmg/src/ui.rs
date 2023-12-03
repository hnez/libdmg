use std::time::Duration;

pub(super) use minifb::Key;
use minifb::{Scale, ScaleMode, Window, WindowOptions};

const PALLET: [u32; 4] = [0x00_9c_ba_a2, 0x00_60_75_65, 0x00_3d_4a_40, 0x00_13_17_14];

const RES_X: usize = 160;
const RES_Y: usize = 144;

pub struct Ui {
    buffer: Box<[u32]>,
    window: Window,
}

impl Ui {
    pub fn new() -> minifb::Result<Self> {
        let buffer = vec![0; RES_X * RES_Y].into_boxed_slice();

        let options = WindowOptions {
            resize: true,
            scale: Scale::FitScreen,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        };

        let mut window = Window::new("pokemu - ESC to exit", RES_X, RES_Y, options)?;

        window.limit_update_rate(Some(Duration::from_micros(16600)));

        Ok(Self { buffer, window })
    }

    pub fn update(&mut self, screen: &[u8]) -> minifb::Result<bool> {
        self.buffer
            .iter_mut()
            .zip(screen.iter())
            .for_each(|(dst, src)| *dst = PALLET[*src as usize]);

        self.window.update_with_buffer(&self.buffer, RES_X, RES_Y)?;

        Ok(self.window.is_open() && !self.window.is_key_down(Key::Escape))
    }

    pub fn buttons<B: Copy>(&mut self, map: &[(Key, B)]) -> Vec<B> {
        map.iter()
            .filter_map(|(k, b)| self.window.is_key_down(*k).then_some(*b))
            .collect()
    }
}
