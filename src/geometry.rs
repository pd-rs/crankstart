pub struct ScreenSpace;
pub type ScreenCoord = i32;
pub type ScreenPoint = euclid::Point2D<ScreenCoord, ScreenSpace>;
pub type ScreenVector = euclid::Vector2D<ScreenCoord, ScreenSpace>;
pub type ScreenRect = euclid::Rect<ScreenCoord, ScreenSpace>;
pub type ScreenSize = euclid::Size2D<ScreenCoord, ScreenSpace>;

pub struct GrSpace;
pub type GrCoord = f32;
pub type GrPoint = euclid::Point2D<GrCoord, GrSpace>;
pub type GrVector = euclid::Vector2D<GrCoord, GrSpace>;
pub type GrRect = euclid::Rect<GrCoord, GrSpace>;
pub type GrSize = euclid::Size2D<GrCoord, GrSpace>;
