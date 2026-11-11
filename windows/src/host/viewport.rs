//! Pure geometry for the immutable 800x600 surface, ported from
//! `NativeViewportPolicy.swift`: no window or engine state.

pub const ORIGINAL_WIDTH: f64 = 800.0;
pub const ORIGINAL_HEIGHT: f64 = 600.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Largest whole backing-pixel multiple. Fails if even 1x cannot fit.
    IntegerFit,
    /// Largest aspect-preserving scale, including fractional downscaling.
    ContinuousFit,
}

/// Direction of increasing Y in the caller's window space. Original surface
/// coordinates always start at the top left, with Y down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YAxis {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    InvalidContentRect,
    InvalidBackingScale,
    InsufficientSpace,
    UnrepresentableGeometry,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn max_x(self) -> f64 {
        self.x + self.width
    }

    pub fn max_y(self) -> f64 {
        self.y + self.height
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pixel {
    pub x: i32,
    pub y: i32,
}

impl Pixel {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ViewportPolicy {
    pub content_rect: Rect,
    pub backing_scale: f64,
    pub mode: Mode,
    pub window_y_axis: YAxis,
    /// Whole drawable pixels, rounded down from content size x backing scale.
    pub backing_size: (f64, f64),
    /// Backing pixels per original pixel (integer only in integer fit).
    pub scale: f64,
    /// Relative to content's top left, Y down, in backing pixels.
    pub viewport_in_backing: Rect,
    /// Same viewport in the caller's point space and Y convention.
    pub viewport_in_window: Rect,
}

impl ViewportPolicy {
    /// `content_rect` excludes title bars, controls and other reserved host
    /// space. Its origin and input points use the same caller-selected space.
    pub fn new(
        content_rect: Rect,
        backing_scale: f64,
        mode: Mode,
        window_y_axis: YAxis,
    ) -> Result<Self, GeometryError> {
        let r = content_rect;
        if !r.x.is_finite()
            || !r.y.is_finite()
            || !r.width.is_finite()
            || !r.height.is_finite()
            || r.width <= 0.0
            || r.height <= 0.0
            || !r.max_x().is_finite()
            || !r.max_y().is_finite()
        {
            return Err(GeometryError::InvalidContentRect);
        }
        if !backing_scale.is_finite() || backing_scale <= 0.0 {
            return Err(GeometryError::InvalidBackingScale);
        }
        let width = (r.width * backing_scale).floor();
        let height = (r.height * backing_scale).floor();
        if !width.is_finite() || !height.is_finite() {
            return Err(GeometryError::UnrepresentableGeometry);
        }
        if width < 1.0 || height < 1.0 {
            return Err(GeometryError::InsufficientSpace);
        }
        let fit = (width / ORIGINAL_WIDTH).min(height / ORIGINAL_HEIGHT);
        let scale = if mode == Mode::IntegerFit {
            fit.floor()
        } else {
            fit
        };
        if !(scale > 0.0) {
            return Err(GeometryError::InsufficientSpace);
        }
        let size_width = ORIGINAL_WIDTH * scale;
        let size_height = ORIGINAL_HEIGHT * scale;
        let mut x = (width - size_width) / 2.0;
        let mut y = (height - size_height) / 2.0;
        if mode == Mode::IntegerFit {
            // Odd leftover pixels go to the right/bottom; every texel stays aligned.
            x = x.floor();
            y = y.floor();
        }
        let viewport = Rect::new(x, y, size_width, size_height);
        let window_y = if window_y_axis == YAxis::Down {
            y
        } else {
            height - viewport.max_y()
        };
        let window_viewport = Rect::new(
            r.x + x / backing_scale,
            r.y + window_y / backing_scale,
            size_width / backing_scale,
            size_height / backing_scale,
        );
        if !scale.is_finite()
            || !(scale / backing_scale).is_finite()
            || scale / backing_scale <= 0.0
            || !window_viewport.x.is_finite()
            || !window_viewport.y.is_finite()
            || !window_viewport.max_x().is_finite()
            || !window_viewport.max_y().is_finite()
            || window_viewport.max_x() <= window_viewport.x
            || window_viewport.max_y() <= window_viewport.y
        {
            return Err(GeometryError::UnrepresentableGeometry);
        }
        Ok(Self {
            content_rect: r,
            backing_scale,
            mode,
            window_y_axis,
            backing_size: (width, height),
            scale,
            viewport_in_backing: viewport,
            viewport_in_window: window_viewport,
        })
    }

    fn ratio(&self) -> f64 {
        self.scale / self.backing_scale
    }

    /// Continuous original coordinates, or `None` for bars, invalid points or
    /// edges outside [0,800) x [0,600). Top/left included; bottom/right
    /// excluded, regardless of the caller's Y direction. Never clamps a bar
    /// into the game.
    pub fn original_point(&self, point: Point) -> Option<Point> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let rect = self.viewport_in_window;
        if !(point.x >= rect.x && point.x < rect.max_x()) {
            return None;
        }
        let y = match self.window_y_axis {
            YAxis::Up => {
                if !(point.y > rect.y && point.y <= rect.max_y()) {
                    return None;
                }
                rect.max_y() - point.y
            }
            YAxis::Down => {
                if !(point.y >= rect.y && point.y < rect.max_y()) {
                    return None;
                }
                point.y - rect.y
            }
        };
        let ratio = self.ratio();
        let original = Point::new((point.x - rect.x) / ratio, y / ratio);
        if is_original_point(original) {
            Some(original)
        } else {
            None
        }
    }

    /// Engine-safe integer coordinates. Quantization happens only after hit testing.
    pub fn original_pixel(&self, point: Point) -> Option<Pixel> {
        let original = self.original_point(point)?;
        Some(Pixel::new(
            original.x.floor() as i32,
            original.y.floor() as i32,
        ))
    }

    /// Nearest surface pixel for a gesture that started inside the game (a held
    /// drag leaving the viewport). The original fullscreen pointer could not
    /// leave 800x600, so the edge pixel stands in for any point beyond it.
    pub fn clamped_original_pixel(&self, point: Point) -> Option<Pixel> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let rect = self.viewport_in_window;
        let ratio = self.ratio();
        let offset_y = if self.window_y_axis == YAxis::Up {
            rect.max_y() - point.y
        } else {
            point.y - rect.y
        };
        let x = ((point.x - rect.x) / ratio).floor();
        let y = (offset_y / ratio).floor();
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        Some(Pixel::new(
            x.clamp(0.0, 799.0) as i32,
            y.clamp(0.0, 599.0) as i32,
        ))
    }

    /// Inverse of `original_point`, in the same point space supplied at
    /// construction. Input denotes continuous pixel-edge coordinates, not
    /// pixel centers.
    pub fn window_point(&self, original: Point) -> Option<Point> {
        if !is_original_point(original) {
            return None;
        }
        let rect = self.viewport_in_window;
        let ratio = self.ratio();
        let x = rect.x + original.x * ratio;
        let offset_y = original.y * ratio;
        let y = if self.window_y_axis == YAxis::Up {
            rect.max_y() - offset_y
        } else {
            rect.y + offset_y
        };
        Some(Point::new(x, y))
    }

    /// Pixel-center inverse for synthetic events and cursor placement.
    pub fn window_point_for_pixel(&self, pixel: Pixel) -> Option<Point> {
        if !(0..800).contains(&pixel.x) || !(0..600).contains(&pixel.y) {
            return None;
        }
        self.window_point(Point::new(pixel.x as f64 + 0.5, pixel.y as f64 + 0.5))
    }
}

fn is_original_point(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x >= 0.0
        && point.x < ORIGINAL_WIDTH
        && point.y >= 0.0
        && point.y < ORIGINAL_HEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_frame_reference() {
        let policy = ViewportPolicy::new(
            Rect::new(0.0, 105.0, 800.0, 600.0),
            1.0,
            Mode::IntegerFit,
            YAxis::Up,
        )
        .unwrap();
        assert_eq!(policy.viewport_in_window, Rect::new(0.0, 105.0, 800.0, 600.0));
        assert_eq!(
            policy.original_point(Point::new(180.0, 132.0)),
            Some(Point::new(180.0, 573.0))
        );
        assert_eq!(
            policy.window_point(Point::new(180.0, 573.0)),
            Some(Point::new(180.0, 132.0))
        );
    }

    #[test]
    fn bars_reject_input_but_clamp_held_drag() {
        let policy = ViewportPolicy::new(
            Rect::new(10.0, 20.0, 1200.0, 800.0),
            1.0,
            Mode::IntegerFit,
            YAxis::Up,
        )
        .unwrap();
        assert_eq!(policy.viewport_in_window, Rect::new(210.0, 120.0, 800.0, 600.0));
        assert_eq!(policy.original_pixel(Point::new(210.0, 720.0)), Some(Pixel::new(0, 0)));
        assert_eq!(policy.original_point(Point::new(1010.0, 720.0)), None);
        assert_eq!(
            policy.clamped_original_pixel(Point::new(0.0, 900.0)),
            Some(Pixel::new(0, 599))
        );
    }

    #[test]
    fn invalid_geometry_is_rejected() {
        for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                ViewportPolicy::new(
                    Rect::new(0.0, 0.0, value, 600.0),
                    1.0,
                    Mode::ContinuousFit,
                    YAxis::Down
                )
                .unwrap_err(),
                GeometryError::InvalidContentRect
            );
            assert!(ViewportPolicy::new(
                Rect::new(0.0, 0.0, 800.0, 600.0),
                value,
                Mode::ContinuousFit,
                YAxis::Down
            )
            .is_err());
        }
    }
}
