use {
    crate::{log_to_console, pd_func_caller, pd_func_caller_log, system::System},
    alloc::{format, rc::Rc},
    anyhow::{anyhow, ensure, Error},
    core::{cell::RefCell, ptr, slice},
    crankstart_sys::{
        ctypes::c_int, size_t, LCDBitmapDrawMode, LCDBitmapDrawMode_kDrawModeBlackTransparent,
        LCDBitmapDrawMode_kDrawModeCopy, LCDBitmapDrawMode_kDrawModeFillBlack,
        LCDBitmapDrawMode_kDrawModeFillWhite, LCDBitmapDrawMode_kDrawModeInverted,
        LCDBitmapDrawMode_kDrawModeNXOR, LCDBitmapDrawMode_kDrawModeWhiteTransparent,
        LCDBitmapDrawMode_kDrawModeXOR, LCDBitmapFlip, LCDBitmapFlip_kBitmapFlippedX,
        LCDBitmapFlip_kBitmapFlippedXY, LCDBitmapFlip_kBitmapFlippedY,
        LCDBitmapFlip_kBitmapUnflipped, LCDBitmapTable, LCDSolidColor, LCDSolidColor_kColorBlack,
        LCDSolidColor_kColorClear, LCDSolidColor_kColorWhite, LCDSolidColor_kColorXOR,
        PDStringEncoding_kUTF8Encoding, LCD_ROWS, LCD_ROWSIZE,
    },
    cstr_core::{CStr, CString},
    hashbrown::HashMap,
};

pub use crankstart_sys::{LCDColor, LCDRect, PDRect};

pub fn rect_make(x: f32, y: f32, width: f32, height: f32) -> PDRect {
    PDRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Debug)]
pub struct BitmapData {
    pub width: c_int,
    pub height: c_int,
    pub rowbytes: c_int,
    pub hasmask: bool,
}

#[derive(Debug)]
pub struct BitmapInner {
    pub(crate) raw_bitmap: *mut crankstart_sys::LCDBitmap,
    graphics: Graphics,
}

impl BitmapInner {
    pub fn get_data(&self) -> Result<BitmapData, Error> {
        let mut width = 0;
        let mut height = 0;
        let mut rowbytes = 0;
        let mut hasmask = 0;
        pd_func_caller!(
            (*self.graphics.0).getBitmapData,
            self.raw_bitmap,
            &mut width,
            &mut height,
            &mut rowbytes,
            &mut hasmask,
            ptr::null_mut(),
        )?;
        Ok(BitmapData {
            width,
            height,
            rowbytes,
            hasmask: hasmask != 0,
        })
    }

    pub fn draw(
        &self,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        x: i32,
        y: i32,
        mode: BitmapDrawMode,
        flip: BitmapFlip,
        clip: LCDRect,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.graphics.0).drawBitmap,
            self.raw_bitmap,
            raw_bitmap(target),
            raw_bitmap(stencil),
            x,
            y,
            mode.into(),
            flip.into(),
            clip,
        )?;
        Ok(())
    }

    pub fn duplicate(&self) -> Result<Self, Error> {
        let raw_bitmap = pd_func_caller!((*self.graphics.0).copyBitmap, self.raw_bitmap)?;

        Ok(Self {
            raw_bitmap,
            graphics: self.graphics.clone(),
        })
    }

    pub fn transform(&self, rotation: f32, scale_x: f32, scale_y: f32) -> Result<Self, Error> {
        let raw_bitmap = pd_func_caller!(
            (*self.graphics.0).transformedBitmap,
            self.raw_bitmap,
            rotation,
            scale_x,
            scale_y,
            core::ptr::null_mut(),
        )?;
        Ok(Self {
            raw_bitmap,
            graphics: self.graphics.clone(),
        })
    }
}

impl Drop for BitmapInner {
    fn drop(&mut self) {
        log_to_console!("dropping bitmap {}", self.raw_bitmap as u64);
        pd_func_caller_log!((*self.graphics.0).freeBitmap, self.raw_bitmap);
    }
}

pub type BitmapInnerPtr = Rc<RefCell<BitmapInner>>;

#[derive(Clone, Debug)]
pub struct Bitmap {
    pub(crate) inner: BitmapInnerPtr,
}

impl Bitmap {
    fn new(raw_bitmap: *mut crankstart_sys::LCDBitmap, graphics: &Graphics) -> Self {
        Bitmap {
            inner: Rc::new(RefCell::new(BitmapInner {
                raw_bitmap,
                graphics: graphics.clone(),
            })),
        }
    }

    pub fn get_data(&self) -> Result<BitmapData, Error> {
        self.inner.borrow().get_data()
    }

    pub fn draw(
        &self,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        x: i32,
        y: i32,
        mode: BitmapDrawMode,
        flip: BitmapFlip,
        clip: LCDRect,
    ) -> Result<(), Error> {
        self.inner
            .borrow()
            .draw(target, stencil, x, y, mode, flip, clip)
    }

    pub fn transform(&self, rotation: f32, scale_x: f32, scale_y: f32) -> Result<Bitmap, Error> {
        let inner = self.inner.borrow().transform(rotation, scale_x, scale_y)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(inner)),
        })
    }
}

type OptionalBitmap<'a> = Option<&'a mut Bitmap>;

fn raw_bitmap(bitmap: OptionalBitmap<'_>) -> *mut crankstart_sys::LCDBitmap {
    if let Some(bitmap) = bitmap {
        bitmap.inner.borrow().raw_bitmap
    } else {
        ptr::null_mut() as *mut crankstart_sys::LCDBitmap
    }
}

pub struct Font(*mut crankstart_sys::LCDFont);

impl Font {
    pub fn new(font: *mut crankstart_sys::LCDFont) -> Result<Self, Error> {
        anyhow::ensure!(font != ptr::null_mut(), "Null pointer passed to Font::new");
        Ok(Self(font))
    }
}

pub enum BitmapDrawMode {
    Copy,
    WhiteTransparent,
    BlackTransparent,
    FillWhite,
    FillBlack,
    XOR,
    NXOR,
    Inverted,
}

impl From<BitmapDrawMode> for LCDBitmapDrawMode {
    fn from(draw_mode: BitmapDrawMode) -> Self {
        let lcd_draw_mode = match draw_mode {
            BitmapDrawMode::Copy => LCDBitmapDrawMode_kDrawModeCopy,
            BitmapDrawMode::WhiteTransparent => LCDBitmapDrawMode_kDrawModeWhiteTransparent,
            BitmapDrawMode::BlackTransparent => LCDBitmapDrawMode_kDrawModeBlackTransparent,
            BitmapDrawMode::FillWhite => LCDBitmapDrawMode_kDrawModeFillWhite,
            BitmapDrawMode::FillBlack => LCDBitmapDrawMode_kDrawModeFillBlack,
            BitmapDrawMode::XOR => LCDBitmapDrawMode_kDrawModeXOR,
            BitmapDrawMode::NXOR => LCDBitmapDrawMode_kDrawModeNXOR,
            BitmapDrawMode::Inverted => LCDBitmapDrawMode_kDrawModeInverted,
        };
        lcd_draw_mode
    }
}

#[derive(Debug)]
pub enum BitmapFlip {
    Unflipped,
    FlippedX,
    FlippedY,
    FlippedXY,
}

impl From<BitmapFlip> for LCDBitmapFlip {
    fn from(flip: BitmapFlip) -> Self {
        let lcd_flip = match flip {
            BitmapFlip::Unflipped => LCDBitmapFlip_kBitmapUnflipped,
            BitmapFlip::FlippedX => LCDBitmapFlip_kBitmapFlippedX,
            BitmapFlip::FlippedY => LCDBitmapFlip_kBitmapFlippedY,
            BitmapFlip::FlippedXY => LCDBitmapFlip_kBitmapFlippedXY,
        };
        lcd_flip
    }
}

pub enum SolidColor {
    Black,
    White,
    Clear,
    XOR,
}

impl From<SolidColor> for LCDSolidColor {
    fn from(color: SolidColor) -> Self {
        let solid_color: LCDSolidColor = match color {
            SolidColor::Black => LCDSolidColor_kColorBlack,
            SolidColor::White => LCDSolidColor_kColorWhite,
            SolidColor::Clear => LCDSolidColor_kColorClear,
            SolidColor::XOR => LCDSolidColor_kColorXOR,
        };
        solid_color
    }
}

#[derive(Debug)]
struct BitmapTableInner {
    raw_bitmap_table: *mut LCDBitmapTable,
    bitmaps: HashMap<usize, Bitmap>,
    graphics: Graphics,
}

impl BitmapTableInner {
    fn get_bitmap(&mut self, index: usize) -> Result<Bitmap, Error> {
        if let Some(bitmap) = self.bitmaps.get(&index) {
            Ok(bitmap.clone())
        } else {
            let raw_bitmap = pd_func_caller!(
                (*self.graphics.0).getTableBitmap,
                self.raw_bitmap_table,
                index as c_int
            )?;
            ensure!(
                raw_bitmap != ptr::null_mut(),
                "Failed to load bitmap {} from table {:?}",
                index,
                self.raw_bitmap_table
            );
            let bitmap = Bitmap::new(raw_bitmap, &self.graphics);
            self.bitmaps.insert(index, bitmap.clone());
            Ok(bitmap)
        }
    }
}

impl Drop for BitmapTableInner {
    fn drop(&mut self) {
        log_to_console!("dropping bitmap table {}", self.raw_bitmap_table as u64);
        pd_func_caller_log!((*self.graphics.0).freeBitmapTable, self.raw_bitmap_table);
    }
}

type BitmapTableInnerPtr = Rc<RefCell<BitmapTableInner>>;

#[derive(Clone, Debug)]
pub struct BitmapTable {
    inner: BitmapTableInnerPtr,
}

impl BitmapTable {
    pub fn get_bitmap(&self, index: usize) -> Result<Bitmap, Error> {
        self.inner.borrow_mut().get_bitmap(index)
    }
}

#[derive(Clone, Debug)]
pub struct Graphics(*mut crankstart_sys::playdate_graphics);

impl Graphics {
    pub fn new(graphics: *mut crankstart_sys::playdate_graphics) -> Self {
        Self(graphics)
    }

    pub fn get_frame(&self) -> Result<&'static mut [u8], Error> {
        let ptr = pd_func_caller!((*self.0).getFrame)?;
        anyhow::ensure!(
            ptr != ptr::null_mut(),
            "Null pointer returned from getFrame"
        );
        let frame = unsafe { slice::from_raw_parts_mut(ptr, (LCD_ROWSIZE * LCD_ROWS) as usize) };
        Ok(frame)
    }

    pub fn get_display_frame(&self) -> Result<&'static mut [u8], Error> {
        let ptr = pd_func_caller!((*self.0).getDisplayFrame)?;
        anyhow::ensure!(
            ptr != ptr::null_mut(),
            "Null pointer returned from getDisplayFrame"
        );
        let frame = unsafe { slice::from_raw_parts_mut(ptr, (LCD_ROWSIZE * LCD_ROWS) as usize) };
        Ok(frame)
    }

    pub fn mark_updated_rows(&self, x: i32, y: i32) -> Result<(), Error> {
        pd_func_caller!((*self.0).markUpdatedRows, x, y)
    }

    pub fn new_bitmap(&self, width: i32, height: i32, bg_color: SolidColor) -> Result<Bitmap, Error> {
        let raw_bitmap = pd_func_caller!((*self.0).newBitmap, width, height, bg_color as usize)?;
        Ok(Bitmap {
            inner: Rc::new(RefCell::new(BitmapInner {
                raw_bitmap,
                graphics: self.clone(),
            })),
        })
    }

    pub fn load_bitmap(&self, path: &str) -> Result<Bitmap, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut out_err: *const crankstart_sys::ctypes::c_char = ptr::null_mut();
        let raw_bitmap = pd_func_caller!((*self.0).loadBitmap, c_path.as_ptr(), &mut out_err)?;
        if raw_bitmap == ptr::null_mut() {
            if out_err != ptr::null_mut() {
                let err_msg = unsafe { CStr::from_ptr(out_err).to_string_lossy().into_owned() };
                Err(anyhow!(err_msg))
            } else {
                Err(anyhow!(
                    "load_bitmap failed without providing an error message"
                ))
            }
        } else {
            Ok(Bitmap {
                inner: Rc::new(RefCell::new(BitmapInner {
                    raw_bitmap,
                    graphics: self.clone(),
                })),
            })
        }
    }

    pub fn load_bitmap_table(&self, path: &str) -> Result<BitmapTable, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut out_err: *const crankstart_sys::ctypes::c_char = ptr::null_mut();
        let raw_bitmap_table =
            pd_func_caller!((*self.0).loadBitmapTable, c_path.as_ptr(), &mut out_err)?;
        if raw_bitmap_table == ptr::null_mut() {
            if out_err != ptr::null_mut() {
                let err_msg = unsafe { CStr::from_ptr(out_err).to_string_lossy().into_owned() };
                Err(anyhow!(err_msg))
            } else {
                Err(anyhow!(
                    "load_bitmap_table failed without providing an error message"
                ))
            }
        } else {
            Ok(BitmapTable {
                inner: Rc::new(RefCell::new(BitmapTableInner {
                    raw_bitmap_table,
                    bitmaps: HashMap::new(),
                    graphics: self.clone(),
                })),
            })
        }
    }

    pub fn clear(&self, color: SolidColor) -> Result<(), Error> {
        let color: LCDSolidColor = color.into();
        pd_func_caller!((*self.0).clear, color as usize)
    }

    pub fn fill_triangle(
        &self,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        color: SolidColor,
        clip: LCDRect,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).fillTriangle,
            raw_bitmap(target),
            raw_bitmap(stencil),
            x1,
            y1,
            x2,
            y2,
            x3,
            y3,
            color as usize,
            clip
        )
    }

    pub fn fill_rect(
        &self,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: SolidColor,
        clip: LCDRect,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).fillRect,
            raw_bitmap(target),
            raw_bitmap(stencil),
            x,
            y,
            width,
            height,
            color as usize,
            clip
        )
    }

    pub fn load_font(&self, path: &str) -> Result<Font, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let font = pd_func_caller!((*self.0).loadFont, c_path.as_ptr(), ptr::null_mut())?;
        Font::new(font)
    }

    pub fn draw_text(
        &self,
        font: &Font,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        text: &str,
        x: i32,
        y: i32,
        mode: BitmapDrawMode,
        tracking: i32,
        clip: LCDRect,
    ) -> Result<i32, Error> {
        let c_text = CString::new(text).map_err(Error::msg)?;
        pd_func_caller!(
            (*self.0).drawText,
            font.0,
            raw_bitmap(target),
            raw_bitmap(stencil),
            c_text.as_ptr() as *const core::ffi::c_void,
            text.len() as size_t,
            PDStringEncoding_kUTF8Encoding,
            x,
            y,
            mode.into(),
            tracking,
            clip,
        )
    }

    pub fn get_text_width(&self, font: &Font, text: &str, tracking: i32) -> Result<i32, Error> {
        let c_text = CString::new(text).map_err(Error::msg)?;
        pd_func_caller!(
            (*self.0).getTextWidth,
            font.0,
            c_text.as_ptr() as *const core::ffi::c_void,
            text.len() as size_t,
            PDStringEncoding_kUTF8Encoding,
            tracking,
        )
    }
}
