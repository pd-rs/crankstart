#![no_std]

extern crate alloc;

use alloc::vec;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use hashbrown::HashMap;

use {
    alloc::boxed::Box,
    anyhow::Error,
    crankstart::{
        crankstart_game,
        Game,
        geometry::ScreenPoint,
        graphics::{Graphics, LCDColor, LCDSolidColor},
        log_to_console,
        Playdate, system::{MenuItem, System},
    },
    euclid::point2,
};
use crankstart::system::MenuItemKind;

struct State {
    _menu_items: Rc<RefCell<HashMap<&'static str, MenuItem>>>,
    text_location: ScreenPoint,
}

impl State {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        crankstart::display::Display::get().set_refresh_rate(20.0)?;
        let menu_items = Rc::new(RefCell::new(HashMap::new()));
        let system = System::get();
        let normal_item = {
            system.add_menu_item(
                "Select Me",
                Box::new(|| {
                    log_to_console!("Normal option picked");
                }
                ),
            )?
        };
        let checkmark_item = {
            let ref_menu_items = menu_items.clone();
            system.add_checkmark_menu_item(
                "Toggle Me",
                false,
                Box::new(move || {
                    let value_of_item = {
                        let menu_items = ref_menu_items.borrow();
                        let this_menu_item = menu_items.get("checkmark").unwrap();
                        System::get().get_menu_item_value(this_menu_item).unwrap() != 0
                    };
                    log_to_console!("Checked option picked: Value is now: {}", value_of_item);
                }),
            )?
        };
        let options_item = {
            let ref_menu_items = menu_items.clone();
            let options: Vec<String> = vec!["Small".into(), "Medium".into(), "Large".into()];
            system.add_options_menu_item(
                "Size",
                options,
                Box::new(move || {
                    let value_of_item = {
                        let menu_items = ref_menu_items.borrow();
                        let this_menu_item = menu_items.get("options").unwrap();
                        let idx = System::get().get_menu_item_value(this_menu_item).unwrap();
                        match &this_menu_item.kind {
                            MenuItemKind::Options(opts) => {
                                opts.get(idx ).map(|s| s.clone())
                            }
                            _ => None
                        }
                    };
                    log_to_console!("Checked option picked: Value is now {:?}", value_of_item);
                }),
            )?
        };
        {
            let mut menu_items = menu_items.borrow_mut();
            menu_items.insert("normal", normal_item);
            menu_items.insert("checkmark", checkmark_item);
            menu_items.insert("options", options_item);
        }
        Ok(Box::new(Self {
            _menu_items: menu_items,
            text_location: point2(100, 100),
        }))
    }
}

impl Game for State {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let graphics = Graphics::get();
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;
        graphics.draw_text("Menu Items", self.text_location).unwrap();

        System::get().draw_fps(0, 0)?;

        Ok(())
    }
}


crankstart_game!(State);
