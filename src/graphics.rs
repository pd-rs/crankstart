use {
    crate::{
        geometry::{ScreenPoint, ScreenRect, ScreenSize, ScreenVector},
        log_to_console, pd_func_caller, pd_func_caller_log,
        system::System,
    },
    alloc::{format, rc::Rc},
    anyhow::{anyhow, ensure, Error},
    core::{cell::RefCell, ops::RangeInclusive, ptr, slice},
    crankstart_sys::{ctypes::c_int, LCDBitmapTable, LCDPattern},
    cstr_core::{CStr, CString},
    euclid::default::{Point2D, Vector2D},
    hashbrown::HashMap,
};

pub use crankstart_sys::{
    LCDBitmapDrawMode, LCDBitmapFlip, LCDLineCapStyle, LCDRect, LCDSolidColor, PDRect,
    PDStringEncoding, LCD_COLUMNS, LCD_ROWS, LCD_ROWSIZE,
};

pub fn rect_make(x: f32, y: f32, width: f32, height: f32) -> PDRect {
    PDRect {
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, Debug)]
pub enum LCDColor {
    Solid(LCDSolidColor),
    Pattern(LCDPattern),
}

impl From<LCDColor> for usize {
    fn from(color: LCDColor) -> Self {
        match color {
            LCDColor::Solid(solid_color) => solid_color as usize,
            LCDColor::Pattern(pattern) => {
                let pattern_ptr = &pattern as *const u8;
                pattern_ptr as usize
            }
        }
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
}

impl BitmapInner {
    pub fn get_data(&self) -> Result<BitmapData, Error> {
        let mut width = 0;
        let mut height = 0;
        let mut rowbytes = 0;
        let mut mask_ptr = ptr::null_mut();
        pd_func_caller!(
            (*Graphics::get_ptr()).getBitmapData,
            self.raw_bitmap,
            &mut width,
            &mut height,
            &mut rowbytes,
            &mut mask_ptr,
            ptr::null_mut(),
        )?;
        Ok(BitmapData {
            width,
            height,
            rowbytes,
            hasmask: mask_ptr != ptr::null_mut(),
        })
    }

    pub fn draw(&self, location: ScreenPoint, flip: LCDBitmapFlip) -> Result<(), Error> {
        pd_func_caller!(
            (*Graphics::get_ptr()).drawBitmap,
            self.raw_bitmap,
            location.x,
            location.y,
            flip.into(),
        )?;
        Ok(())
    }

    pub fn draw_scaled(&self, location: ScreenPoint, scale: Vector2D<f32>) -> Result<(), Error> {
        pd_func_caller!(
            (*Graphics::get_ptr()).drawScaledBitmap,
            self.raw_bitmap,
            location.x,
            location.y,
            scale.x,
            scale.y,
        )
    }

    pub fn draw_rotated(
        &self,
        location: ScreenPoint,
        degrees: f32,
        center: Vector2D<f32>,
        scale: Vector2D<f32>,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*Graphics::get_ptr()).drawRotatedBitmap,
            self.raw_bitmap,
            location.x,
            location.y,
            degrees,
            center.x,
            center.y,
            scale.x,
            scale.y,
        )
    }

    pub fn rotated(&self, degrees: f32, scale: Vector2D<f32>) -> Result<Self, Error> {
        let raw_bitmap = pd_func_caller!(
            (*Graphics::get_ptr()).rotatedBitmap,
            self.raw_bitmap,
            degrees,
            scale.x,
            scale.y,
            // No documentation on this anywhere, but null works in testing.
            ptr::null_mut(), // allocedSize
        )?;
        Ok(Self { raw_bitmap })
    }

    pub fn tile(
        &self,
        location: ScreenPoint,
        size: ScreenSize,
        flip: LCDBitmapFlip,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*Graphics::get_ptr()).tileBitmap,
            self.raw_bitmap,
            location.x,
            location.y,
            size.width,
            size.height,
            flip.into(),
        )?;
        Ok(())
    }

    pub fn clear(&self, color: LCDColor) -> Result<(), Error> {
        pd_func_caller!(
            (*Graphics::get_ptr()).clearBitmap,
            self.raw_bitmap,
            color.into()
        )
    }

    pub fn duplicate(&self) -> Result<Self, Error> {
        let raw_bitmap = pd_func_caller!((*Graphics::get_ptr()).copyBitmap, self.raw_bitmap)?;

        Ok(Self { raw_bitmap })
    }

    pub fn transform(&self, rotation: f32, scale: Vector2D<f32>) -> Result<Self, Error> {
        // let raw_bitmap = pd_func_caller!(
        //     (*Graphics::get_ptr()).transformedBitmap,
        //     self.raw_bitmap,
        //     rotation,
        //     scale.x,
        //     scale.y,
        //     core::ptr::null_mut(),
        // )?;
        // Ok(Self { raw_bitmap })
        todo!();
    }

    pub fn into_color(&self, bitmap: Bitmap, top_left: Point2D<i32>) -> Result<LCDColor, Error> {
        let mut pattern = LCDPattern::default();
        let pattern_ptr = pattern.as_mut_ptr();
        let mut pattern_val = pattern_ptr as usize;
        let graphics = Graphics::get();
        pd_func_caller!(
            (*graphics.0).setColorToPattern,
            &mut pattern_val,
            self.raw_bitmap,
            top_left.x,
            top_left.y
        )?;
        Ok(LCDColor::Pattern(pattern))
    }

    pub fn load(&self, path: &str) -> Result<(), Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut out_err: *const crankstart_sys::ctypes::c_char = ptr::null_mut();
        let graphics = Graphics::get();
        pd_func_caller!(
            (*graphics.0).loadIntoBitmap,
            c_path.as_ptr(),
            self.raw_bitmap,
            &mut out_err
        )?;
        if out_err != ptr::null_mut() {
            let err_msg = unsafe { CStr::from_ptr(out_err).to_string_lossy().into_owned() };
            Err(anyhow!(err_msg))
        } else {
            Ok(())
        }
    }

    pub fn check_mask_collision(
        &self,
        my_location: ScreenPoint,
        my_flip: LCDBitmapFlip,
        other: Bitmap,
        other_location: ScreenPoint,
        other_flip: LCDBitmapFlip,
        rect: ScreenRect,
    ) -> Result<bool, Error> {
        let graphics = Graphics::get();
        let other_raw = other.inner.borrow().raw_bitmap;
        let lcd_rect: LCDRect = rect.to_untyped().into();
        let pixels_covered = pd_func_caller!(
            (*graphics.0).checkMaskCollision,
            self.raw_bitmap,
            my_location.x,
            my_location.y,
            my_flip,
            other_raw,
            other_location.x,
            other_location.y,
            other_flip,
            lcd_rect,
        )?;
        Ok(pixels_covered != 0)
    }
}

impl Drop for BitmapInner {
    fn drop(&mut self) {
        pd_func_caller_log!((*Graphics::get_ptr()).freeBitmap, self.raw_bitmap);
    }
}

pub type BitmapInnerPtr = Rc<RefCell<BitmapInner>>;

#[derive(Clone, Debug)]
pub struct Bitmap {
    pub(crate) inner: BitmapInnerPtr,
}

impl Bitmap {
    fn new(raw_bitmap: *mut crankstart_sys::LCDBitmap) -> Self {
        Bitmap {
            inner: Rc::new(RefCell::new(BitmapInner { raw_bitmap })),
        }
    }

    pub fn get_data(&self) -> Result<BitmapData, Error> {
        self.inner.borrow().get_data()
    }

    pub fn draw(&self, location: ScreenPoint, flip: LCDBitmapFlip) -> Result<(), Error> {
        self.inner.borrow().draw(location, flip)
    }

    pub fn draw_scaled(&self, location: ScreenPoint, scale: Vector2D<f32>) -> Result<(), Error> {
        self.inner.borrow().draw_scaled(location, scale)
    }

    /// Draw the `Bitmap` to the given `location`, rotated `degrees` about the `center` point,
    /// scaled up or down in size by `scale`.  `center` is given by two numbers between 0.0 and
    /// 1.0, where (0, 0) is the top left and (0.5, 0.5) is the center point.
    pub fn draw_rotated(
        &self,
        location: ScreenPoint,
        degrees: f32,
        center: Vector2D<f32>,
        scale: Vector2D<f32>,
    ) -> Result<(), Error> {
        self.inner
            .borrow()
            .draw_rotated(location, degrees, center, scale)
    }

    /// Return a copy of self, rotated by `degrees` and scaled up or down in size by `scale`.
    pub fn rotated(&self, degrees: f32, scale: Vector2D<f32>) -> Result<Bitmap, Error> {
        let raw_bitmap = self.inner.borrow().rotated(degrees, scale)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(raw_bitmap)),
        })
    }

    pub fn tile(
        &self,
        location: ScreenPoint,
        size: ScreenSize,
        flip: LCDBitmapFlip,
    ) -> Result<(), Error> {
        self.inner.borrow().tile(location, size, flip)
    }

    pub fn clear(&self, color: LCDColor) -> Result<(), Error> {
        self.inner.borrow().clear(color)
    }

    pub fn transform(&self, rotation: f32, scale: Vector2D<f32>) -> Result<Bitmap, Error> {
        let inner = self.inner.borrow().transform(rotation, scale)?;
        Ok(Self {
            inner: Rc::new(RefCell::new(inner)),
        })
    }

    pub fn into_color(&self, bitmap: Bitmap, top_left: Point2D<i32>) -> Result<LCDColor, Error> {
        self.inner.borrow().into_color(bitmap, top_left)
    }

    pub fn load(&self, path: &str) -> Result<(), Error> {
        self.inner.borrow().load(path)
    }

    pub fn check_mask_collision(
        &self,
        my_location: ScreenPoint,
        my_flip: LCDBitmapFlip,
        other: Bitmap,
        other_location: ScreenPoint,
        other_flip: LCDBitmapFlip,
        rect: ScreenRect,
    ) -> Result<bool, Error> {
        self.inner.borrow().check_mask_collision(
            my_location,
            my_flip,
            other,
            other_location,
            other_flip,
            rect,
        )
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

impl Drop for Font {
    fn drop(&mut self) {
        log_to_console!("Leaking a font");
    }
}

#[derive(Debug)]
struct BitmapTableInner {
    raw_bitmap_table: *mut LCDBitmapTable,
    bitmaps: HashMap<usize, Bitmap>,
}

impl BitmapTableInner {
    fn get_bitmap(&mut self, index: usize) -> Result<Bitmap, Error> {
        if let Some(bitmap) = self.bitmaps.get(&index) {
            Ok(bitmap.clone())
        } else {
            let raw_bitmap = pd_func_caller!(
                (*Graphics::get_ptr()).getTableBitmap,
                self.raw_bitmap_table,
                index as c_int
            )?;
            ensure!(
                raw_bitmap != ptr::null_mut(),
                "Failed to load bitmap {} from table {:?}",
                index,
                self.raw_bitmap_table
            );
            let bitmap = Bitmap::new(raw_bitmap);
            self.bitmaps.insert(index, bitmap.clone());
            Ok(bitmap)
        }
    }

    fn load(&mut self, path: &str) -> Result<(), Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let mut out_err: *const crankstart_sys::ctypes::c_char = ptr::null_mut();
        let graphics = Graphics::get();
        pd_func_caller!(
            (*graphics.0).loadIntoBitmapTable,
            c_path.as_ptr(),
            self.raw_bitmap_table,
            &mut out_err
        )?;
        if out_err != ptr::null_mut() {
            let err_msg = unsafe { CStr::from_ptr(out_err).to_string_lossy().into_owned() };
            Err(anyhow!(err_msg))
        } else {
            Ok(())
        }
    }
}

impl Drop for BitmapTableInner {
    fn drop(&mut self) {
        pd_func_caller_log!(
            (*Graphics::get_ptr()).freeBitmapTable,
            self.raw_bitmap_table
        );
    }
}

type BitmapTableInnerPtr = Rc<RefCell<BitmapTableInner>>;

#[derive(Clone, Debug)]
pub struct BitmapTable {
    inner: BitmapTableInnerPtr,
}

impl BitmapTable {
    pub fn new(raw_bitmap_table: *mut LCDBitmapTable) -> Self {
        Self {
            inner: Rc::new(RefCell::new(BitmapTableInner {
                raw_bitmap_table,
                bitmaps: HashMap::new(),
            })),
        }
    }

    pub fn load(&self, path: &str) -> Result<(), Error> {
        self.inner.borrow_mut().load(path)
    }

    pub fn get_bitmap(&self, index: usize) -> Result<Bitmap, Error> {
        self.inner.borrow_mut().get_bitmap(index)
    }
}

static mut GRAPHICS: Graphics = Graphics(ptr::null_mut());

#[derive(Clone, Debug)]
pub struct Graphics(*const crankstart_sys::playdate_graphics);

impl Graphics {
    pub(crate) fn new(graphics: *const crankstart_sys::playdate_graphics) {
        unsafe {
            GRAPHICS = Self(graphics);
        }
    }

    pub fn get() -> Self {
        unsafe { GRAPHICS.clone() }
    }

    pub fn get_ptr() -> *const crankstart_sys::playdate_graphics {
        Self::get().0
    }

    /// Allows drawing directly into an image rather than the framebuffer, for example for
    /// drawing text into a sprite's image.
    pub fn with_context<F, T>(&self, bitmap: &mut Bitmap, f: F) -> Result<T, Error>
    where
        F: FnOnce() -> Result<T, Error>,
    {
        // Any calls in this context are directly modifying the bitmap, so borrow mutably
        // for safety.
        self.push_context(bitmap.inner.borrow_mut().raw_bitmap)?;
        let res = f();
        self.pop_context()?;
        res
    }

    /// Internal function; use `with_context`.
    fn push_context(&self, raw_bitmap: *mut crankstart_sys::LCDBitmap) -> Result<(), Error> {
        pd_func_caller!((*self.0).pushContext, raw_bitmap)
    }

    /// Internal function; use `with_context`.
    fn pop_context(&self) -> Result<(), Error> {
        pd_func_caller!((*self.0).popContext)
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

    pub fn get_debug_bitmap(&self) -> Result<Bitmap, Error> {
        let raw_bitmap = pd_func_caller!((*self.0).getDebugBitmap)?;
        anyhow::ensure!(
            raw_bitmap != ptr::null_mut(),
            "Null pointer returned from getDebugImage"
        );
        Ok(Bitmap::new(raw_bitmap))
    }

    pub fn get_framebuffer_bitmap(&self) -> Result<Bitmap, Error> {
        let raw_bitmap = pd_func_caller!((*self.0).copyFrameBufferBitmap)?;
        anyhow::ensure!(
            raw_bitmap != ptr::null_mut(),
            "Null pointer returned from getFrameBufferBitmap"
        );
        Ok(Bitmap::new(raw_bitmap))
    }

    pub fn set_background_color(&self, color: LCDSolidColor) -> Result<(), Error> {
        pd_func_caller!((*self.0).setBackgroundColor, color.into())
    }

    pub fn set_draw_mode(&self, mode: LCDBitmapDrawMode) -> Result<(), Error> {
        pd_func_caller!((*self.0).setDrawMode, mode)
    }

    pub fn mark_updated_rows(&self, range: RangeInclusive<i32>) -> Result<(), Error> {
        let (start, end) = range.into_inner();
        pd_func_caller!((*self.0).markUpdatedRows, start, end)
    }

    pub fn display(&self) -> Result<(), Error> {
        pd_func_caller!((*self.0).display)
    }

    pub fn set_draw_offset(&self, offset: ScreenVector) -> Result<(), Error> {
        pd_func_caller!((*self.0).setDrawOffset, offset.x, offset.y)
    }

    pub fn new_bitmap(&self, size: ScreenSize, bg_color: LCDColor) -> Result<Bitmap, Error> {
        let raw_bitmap = pd_func_caller!(
            (*self.0).newBitmap,
            size.width,
            size.height,
            bg_color.into()
        )?;
        anyhow::ensure!(
            raw_bitmap != ptr::null_mut(),
            "Null pointer returned from new_bitmap"
        );
        Ok(Bitmap::new(raw_bitmap))
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
            Ok(Bitmap::new(raw_bitmap))
        }
    }

    pub fn new_bitmap_table(&self, count: usize, size: ScreenSize) -> Result<BitmapTable, Error> {
        let raw_bitmap_table = pd_func_caller!(
            (*self.0).newBitmapTable,
            count as i32,
            size.width,
            size.height
        )?;

        Ok(BitmapTable::new(raw_bitmap_table))
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
            Ok(BitmapTable::new(raw_bitmap_table))
        }
    }

    pub fn clear(&self, color: LCDColor) -> Result<(), Error> {
        pd_func_caller!((*self.0).clear, color.into())
    }

    pub fn draw_line(
        &self,
        p1: ScreenPoint,
        p2: ScreenPoint,
        width: i32,
        color: LCDColor,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).drawLine,
            p1.x,
            p1.y,
            p2.x,
            p2.y,
            width,
            color.into(),
        )
    }

    pub fn fill_triangle(
        &self,
        p1: ScreenPoint,
        p2: ScreenPoint,
        p3: ScreenPoint,
        color: LCDColor,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).fillTriangle,
            p1.x,
            p1.y,
            p2.x,
            p2.y,
            p3.x,
            p3.y,
            color.into(),
        )
    }

    pub fn draw_rect(&self, rect: ScreenRect, color: LCDColor) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).drawRect,
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
            color.into(),
        )
    }

    pub fn fill_rect(&self, rect: ScreenRect, color: LCDColor) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).fillRect,
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
            color.into(),
        )
    }

    pub fn draw_ellipse(
        &self,
        center: ScreenPoint,
        size: ScreenSize,
        line_width: i32,
        start_angle: f32,
        end_angle: f32,
        color: LCDColor,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).drawEllipse,
            center.x,
            center.y,
            size.width,
            size.height,
            line_width,
            start_angle,
            end_angle,
            color.into(),
        )
    }

    pub fn fill_ellipse(
        &self,
        target: OptionalBitmap,
        stencil: OptionalBitmap,
        center: ScreenPoint,
        size: ScreenSize,
        line_width: i32,
        start_angle: f32,
        end_angle: f32,
        color: LCDColor,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.0).fillEllipse,
            center.x,
            center.y,
            size.width,
            size.height,
            start_angle,
            end_angle,
            color.into(),
        )
    }

    pub fn load_font(&self, path: &str) -> Result<Font, Error> {
        let c_path = CString::new(path).map_err(Error::msg)?;
        let font = pd_func_caller!((*self.0).loadFont, c_path.as_ptr(), ptr::null_mut())?;
        Font::new(font)
    }

    pub fn set_font(&self, font: &Font) -> Result<(), Error> {
        pd_func_caller_log!((*self.0).setFont, font.0);
        Ok(())
    }

    pub fn draw_text(&self, text: &str, position: ScreenPoint) -> Result<i32, Error> {
        let c_text = CString::new(text).map_err(Error::msg)?;
        pd_func_caller!(
            (*self.0).drawText,
            c_text.as_ptr() as *const core::ffi::c_void,
            text.len() as usize,
            PDStringEncoding::kUTF8Encoding,
            position.x,
            position.y,
        )
    }

    pub fn get_text_width(&self, font: &Font, text: &str, tracking: i32) -> Result<i32, Error> {
        let c_text = CString::new(text).map_err(Error::msg)?;
        pd_func_caller!(
            (*self.0).getTextWidth,
            font.0,
            c_text.as_ptr() as *const core::ffi::c_void,
            text.len() as usize,
            PDStringEncoding::kUTF8Encoding,
            tracking,
        )
    }

    pub fn get_system_text_width(&self, text: &str, tracking: i32) -> Result<i32, Error> {
        let c_text = CString::new(text).map_err(Error::msg)?;
        pd_func_caller!(
            (*self.0).getTextWidth,
            ptr::null_mut(),
            c_text.as_ptr() as *const core::ffi::c_void,
            text.len(),
            PDStringEncoding::kUTF8Encoding,
            tracking,
        )
    }
}
