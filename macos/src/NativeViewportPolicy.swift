import Foundation
import CoreGraphics

/// Pure geometry for the immutable 800×600 surface; no window or engine state.
struct NativeViewportPolicy {
    enum Mode {
        /// Largest whole backing-pixel multiple. Throws if even 1× cannot fit.
        case integerFit
        /// Largest aspect-preserving scale, including fractional downscaling.
        case continuousFit
    }

    /// Direction of increasing Y in the caller's window (or converted view) space.
    /// Original surface coordinates always start at the top left, with Y down.
    enum YAxis { case up, down }

    enum GeometryError: Error, Equatable {
        case invalidContentRect
        case invalidBackingScale
        case insufficientSpace
        case unrepresentableGeometry
    }

    struct Pixel: Equatable { let x: Int; let y: Int }

    static let originalSize = CGSize(width: 800, height: 600)
    let contentRect: CGRect
    let backingScale: CGFloat
    let mode: Mode
    let windowYAxis: YAxis
    /// Whole drawable pixels, rounded down from content size × backing scale.
    let backingSize: CGSize
    /// Backing pixels per original pixel (integer only in integerFit).
    let scale: CGFloat
    /// Relative to content's top left, Y down, in backing pixels (Metal viewport).
    let viewportInBacking: CGRect
    /// Same viewport in the caller's point space and Y convention.
    let viewportInWindow: CGRect

    /// contentRect excludes title bars, controls and other reserved host space.
    /// Its origin and input points must be in the same caller-selected space.
    init(contentRect: CGRect, backingScale: CGFloat, mode: Mode,
         windowYAxis: YAxis) throws {
        guard contentRect.origin.x.isFinite, contentRect.origin.y.isFinite,
              contentRect.size.width.isFinite, contentRect.size.height.isFinite,
              contentRect.size.width > 0, contentRect.size.height > 0,
              contentRect.maxX.isFinite, contentRect.maxY.isFinite else {
            throw GeometryError.invalidContentRect
        }
        guard backingScale.isFinite, backingScale > 0 else {
            throw GeometryError.invalidBackingScale
        }
        let width = (contentRect.width * backingScale).rounded(.down)
        let height = (contentRect.height * backingScale).rounded(.down)
        guard width.isFinite, height.isFinite else {
            throw GeometryError.unrepresentableGeometry
        }
        guard width >= 1, height >= 1 else { throw GeometryError.insufficientSpace }
        let fit = min(width / 800, height / 600)
        let scale = mode == .integerFit ? fit.rounded(.down) : fit
        guard scale > 0 else { throw GeometryError.insufficientSpace }
        let size = CGSize(width: 800 * scale, height: 600 * scale)
        var x = (width - size.width) / 2
        var y = (height - size.height) / 2
        if mode == .integerFit {
            // Odd leftover pixels go to the right/bottom; every texel stays aligned.
            x = x.rounded(.down)
            y = y.rounded(.down)
        }
        let viewport = CGRect(origin: CGPoint(x: x, y: y), size: size)
        let windowY = windowYAxis == .down ? y : height - viewport.maxY
        let windowViewport = CGRect(x: contentRect.origin.x + x / backingScale,
                                    y: contentRect.origin.y + windowY / backingScale,
                                    width: size.width / backingScale,
                                    height: size.height / backingScale)
        guard scale.isFinite, (scale / backingScale).isFinite,
              scale / backingScale > 0,
              windowViewport.minX.isFinite, windowViewport.minY.isFinite,
              windowViewport.maxX.isFinite, windowViewport.maxY.isFinite,
              windowViewport.maxX > windowViewport.minX,
              windowViewport.maxY > windowViewport.minY else {
            throw GeometryError.unrepresentableGeometry
        }
        self.contentRect = contentRect
        self.backingScale = backingScale
        self.mode = mode
        self.windowYAxis = windowYAxis
        self.backingSize = CGSize(width: width, height: height)
        self.scale = scale
        self.viewportInBacking = viewport
        self.viewportInWindow = windowViewport
    }

    /// Continuous original coordinates, or nil for bars, invalid points or edges
    /// outside [0,800) × [0,600). Top/left included; bottom/right excluded,
    /// regardless of the caller's Y direction. Never clamps a bar into the game.
    func originalPoint(atWindowPoint point: CGPoint) -> CGPoint? {
        guard point.x.isFinite, point.y.isFinite else { return nil }
        let rect = viewportInWindow
        guard point.x >= rect.minX, point.x < rect.maxX else { return nil }
        let y: CGFloat
        switch windowYAxis {
        case .up:
            guard point.y > rect.minY, point.y <= rect.maxY else { return nil }
            y = rect.maxY - point.y
        case .down:
            guard point.y >= rect.minY, point.y < rect.maxY else { return nil }
            y = point.y - rect.minY
        }
        let point = CGPoint(x: (point.x - rect.minX) / (scale / backingScale),
                            y: y / (scale / backingScale))
        return Self.isOriginalPoint(point) ? point : nil
    }

    /// Engine-safe integer coordinates. Quantization happens only after hit testing.
    func originalPixel(atWindowPoint point: CGPoint) -> Pixel? {
        guard let original = originalPoint(atWindowPoint: point) else { return nil }
        return Pixel(x: Int(original.x.rounded(.down)), y: Int(original.y.rounded(.down)))
    }

    /// Nearest surface pixel for a gesture that started inside the game (a held
    /// drag leaving the viewport). The original fullscreen pointer could not leave
    /// 800×600, so the edge pixel stands in for any point beyond it.
    func clampedOriginalPixel(atWindowPoint point: CGPoint) -> Pixel? {
        guard point.x.isFinite, point.y.isFinite else { return nil }
        let rect = viewportInWindow, ratio = scale / backingScale
        let offsetY = windowYAxis == .up ? rect.maxY - point.y : point.y - rect.minY
        let x = ((point.x - rect.minX) / ratio).rounded(.down)
        let y = (offsetY / ratio).rounded(.down)
        guard x.isFinite, y.isFinite else { return nil }
        return Pixel(x: Int(min(799, max(0, x))), y: Int(min(599, max(0, y))))
    }

    /// Inverse of originalPoint, in the same point space supplied at construction.
    /// Input denotes continuous pixel-edge coordinates, not pixel centers.
    func windowPoint(forOriginalPoint point: CGPoint) -> CGPoint? {
        guard Self.isOriginalPoint(point) else { return nil }
        let rect = viewportInWindow
        let x = rect.minX + point.x * (scale / backingScale)
        let offsetY = point.y * (scale / backingScale)
        return CGPoint(x: x, y: windowYAxis == .up ? rect.maxY - offsetY : rect.minY + offsetY)
    }

    /// Pixel-center inverse for synthetic events/cursor placement (avoids edges).
    func windowPoint(forOriginalPixel pixel: Pixel) -> CGPoint? {
        guard (0..<800).contains(pixel.x), (0..<600).contains(pixel.y) else { return nil }
        return windowPoint(forOriginalPoint: CGPoint(x: CGFloat(pixel.x) + 0.5,
                                                     y: CGFloat(pixel.y) + 0.5))
    }

    private static func isOriginalPoint(_ point: CGPoint) -> Bool {
        point.x.isFinite && point.y.isFinite &&
            point.x >= 0 && point.x < 800 && point.y >= 0 && point.y < 600
    }
}
