use libdmg::{Button, Cartridge, Dmg};

mod ui;

use ui::Key;

const BUTTON_MAP: [(Key, Button); 8] = [
    (Key::A, Button::A),
    (Key::S, Button::B),
    (Key::Q, Button::Select),
    (Key::W, Button::Start),
    (Key::Right, Button::Right),
    (Key::Left, Button::Left),
    (Key::Up, Button::Up),
    (Key::Down, Button::Down),
];

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let mut window = ui::Ui::new()?;

    let mut dmg = {
        let rom = std::fs::read("pr.gb")?;
        let bootrom = std::fs::read("boot.gb")?;
        let sram = std::fs::read("pr.sav").ok();

        let cartridge = Cartridge::new(rom, sram);

        Dmg::new(bootrom, cartridge)
    };

    loop {
        let buttons = window.buttons(&BUTTON_MAP);

        let frame = dmg.run_frame(&buttons);

        if !window.update(frame.as_ref())? {
            break;
        }
    }

    Ok(())
}
