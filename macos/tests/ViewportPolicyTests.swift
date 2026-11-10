import Foundation
import CoreGraphics

// Standalone headless executable. Compile ONLY with NativeViewportPolicy.swift.
@main
@MainActor
struct ViewportPolicyTests {
    typealias Policy = NativeViewportPolicy
    static var checks = 0
    struct Failure: Error, CustomStringConvertible { let description: String }

    static func expect(_ condition: Bool, _ label: String) throws {
        checks += 1
        if !condition { throw Failure(description: label) }
    }

    static func near(_ actual: CGFloat, _ expected: CGFloat, _ label: String,
                     tolerance: CGFloat = 1e-9) throws {
        try expect(actual.isFinite && abs(actual - expected) <= tolerance,
                   "\(label): got \(actual), expected \(expected)")
    }

    static func point(_ actual: CGPoint?, _ x: CGFloat, _ y: CGFloat, _ label: String) throws {
        guard let actual = actual else { throw Failure(description: "\(label): missing point") }
        try near(actual.x, x, label + " x")
        try near(actual.y, y, label + " y")
    }

    static func rect(_ actual: CGRect, _ x: CGFloat, _ y: CGFloat,
                     _ width: CGFloat, _ height: CGFloat, _ label: String) throws {
        try near(actual.minX, x, label + " x")
        try near(actual.minY, y, label + " y")
        try near(actual.width, width, label + " width")
        try near(actual.height, height, label + " height")
    }

    static func rejects(_ error: Policy.GeometryError, rect: CGRect,
                        scale: CGFloat = 1, mode: Policy.Mode = .continuousFit) throws {
        do {
            _ = try Policy(contentRect: rect, backingScale: scale, mode: mode, windowYAxis: .up)
            throw Failure(description: "accepted invalid geometry \(rect), scale \(scale)")
        } catch let actual as Policy.GeometryError {
            try expect(actual == error, "expected \(error), got \(actual)")
        }
    }

    static func fixedReferences() throws {
        // Committed NativeLiveWindow: game frame (0,105,800,600), unflipped NSView.
        for mode in [Policy.Mode.integerFit, .continuousFit] {
            let current = try Policy(contentRect: CGRect(x: 0, y: 105, width: 800, height: 600),
                                     backingScale: 1, mode: mode, windowYAxis: .up)
            try rect(current.viewportInWindow, 0, 105, 800, 600, "current")
            try point(current.originalPoint(atWindowPoint: CGPoint(x: 180, y: 132)),
                      180, 573, "current CAVE point")
            try point(current.originalPoint(atWindowPoint: CGPoint(x: 400, y: 405)),
                      400, 300, "current center")
            try point(current.windowPoint(forOriginalPoint: CGPoint(x: 180, y: 573)),
                      180, 132, "current synthetic event")
        }
        let resized = try Policy(contentRect: CGRect(x: 20, y: 100, width: 1200, height: 900),
                                 backingScale: 1, mode: .continuousFit, windowYAxis: .up)
        try near(resized.scale, 1.5, "resized fit")
        try point(resized.originalPoint(atWindowPoint: CGPoint(x: 290, y: 140.5)),
                  180, 573, "resized CAVE point")
        try point(resized.windowPoint(forOriginalPoint: CGPoint(x: 400, y: 300)),
                  620, 550, "resized center")

        let retina = try Policy(contentRect: CGRect(x: 0, y: 0, width: 1000, height: 700),
                                backingScale: 2, mode: .integerFit, windowYAxis: .up)
        try expect(retina.backingSize == CGSize(width: 2000, height: 1400), "Retina drawable")
        try near(retina.scale, 2, "Retina integer scale counts backing pixels")
        try rect(retina.viewportInBacking, 200, 100, 1600, 1200, "Retina backing")
        try rect(retina.viewportInWindow, 100, 50, 800, 600, "Retina points")
        try point(retina.originalPoint(atWindowPoint: CGPoint(x: 280, y: 77)),
                  180, 573, "Retina CAVE point")
        try point(retina.windowPoint(forOriginalPixel: .init(x: 799, y: 599)),
                  899.5, 50.5, "Retina final pixel center")

        let fit = try Policy(contentRect: CGRect(x: 10, y: 20, width: 1000, height: 700),
                             backingScale: 2, mode: .continuousFit, windowYAxis: .down)
        try near(fit.scale, 7.0 / 3, "Retina continuous backing scale")
        try rect(fit.viewportInWindow, 130.0 / 3, 20, 2800.0 / 3, 700, "Retina fit")
        try point(fit.originalPoint(atWindowPoint: CGPoint(x: 760.0 / 3, y: 410.0 / 3)),
                  180, 100, "Retina continuous reference")

        let small = try Policy(contentRect: CGRect(x: 30, y: 40, width: 400, height: 400),
                               backingScale: 1, mode: .continuousFit, windowYAxis: .down)
        try rect(small.viewportInWindow, 30, 90, 400, 300, "continuous downscale")
        try point(small.originalPoint(atWindowPoint: CGPoint(x: 120, y: 376.5)),
                  180, 573, "downscale reference")
        try rejects(.insufficientSpace, rect: CGRect(x: 0, y: 0, width: 400, height: 400),
                    mode: .integerFit)
        // The same point size DOES fit 1× original pixels on Retina.
        let smallRetina = try Policy(contentRect: CGRect(x: 30, y: 40, width: 400, height: 400),
                                     backingScale: 2, mode: .integerFit, windowYAxis: .down)
        try near(smallRetina.scale, 1, "small Retina integer")
        try rect(smallRetina.viewportInWindow, 30, 90, 400, 300, "small Retina viewport")
    }

    static func boundaries() throws {
        for axis in [Policy.YAxis.up, .down] {
            // Independent fixed frame: left/right bars 200, top/bottom bars 100.
            let p = try Policy(contentRect: CGRect(x: 10, y: 20, width: 1200, height: 800),
                               backingScale: 1, mode: .integerFit, windowYAxis: axis)
            try rect(p.viewportInWindow, 210, 120, 800, 600, "letterbox")
            let top: CGFloat = axis == .up ? 720 : 120
            let bottom: CGFloat = axis == .up ? 120 : 720
            let inward: CGFloat = axis == .up ? -1 : 1
            try point(p.originalPoint(atWindowPoint: CGPoint(x: 210, y: top)), 0, 0, "included top left")
            try expect(p.originalPixel(atWindowPoint: CGPoint(x: 210, y: top)) == .init(x: 0, y: 0), "top left engine pixel")
            try expect(p.originalPoint(atWindowPoint: CGPoint(x: 1010, y: top)) == nil, "excluded right edge")
            try expect(p.originalPoint(atWindowPoint: CGPoint(x: 210, y: bottom)) == nil, "excluded bottom edge")
            for point in [CGPoint(x: 209.99, y: 420), CGPoint(x: 1010.01, y: 420),
                          CGPoint(x: 400, y: top - inward * 0.01),
                          CGPoint(x: 400, y: bottom + inward * 0.01)] {
                try expect(p.originalPoint(atWindowPoint: point) == nil, "bar rejected")
                try expect(p.originalPixel(atWindowPoint: point) == nil, "bar cannot become engine click")
            }
            try expect(p.originalPixel(atWindowPoint: CGPoint(x: 1009.999, y: bottom - inward * 0.001)) == .init(x: 799, y: 599), "last interior pixel")
            try expect(p.originalPixel(atWindowPoint: CGPoint(x: 210.999, y: top + inward * 0.999)) == .init(x: 0, y: 0), "floor after inclusion")

            let odd = try Policy(contentRect: CGRect(x: 10, y: 20, width: 1001, height: 701),
                                 backingScale: 1, mode: .integerFit, windowYAxis: axis)
            try rect(odd.viewportInBacking, 100, 50, 800, 600, "odd backing alignment")
            try rect(odd.viewportInWindow, 110, axis == .up ? 71 : 70, 800, 600, "odd window alignment")
            let fractional = try Policy(contentRect: CGRect(x: 10, y: 20, width: 800.4, height: 600.4),
                                        backingScale: 1, mode: .integerFit, windowYAxis: axis)
            try expect(fractional.backingSize == CGSize(width: 800, height: 600), "fractional drawable rounded down")
            try rect(fractional.viewportInWindow, 10, 20, 800, 600, "fractional remainder excluded")
        }
        let wide = try Policy(contentRect: CGRect(x: 0, y: 0, width: 1200, height: 800),
                              backingScale: 1, mode: .continuousFit, windowYAxis: .down)
        try rect(wide.viewportInWindow, 200.0 / 3, 0, 3200.0 / 3, 800, "fractional bars")
        try point(wide.originalPoint(atWindowPoint: CGPoint(x: 600, y: 400)), 400, 300, "fractional bar center")
        try expect(wide.originalPixel(atWindowPoint: CGPoint(x: 66, y: 400)) == nil, "fractional left bar")
        try expect(wide.originalPixel(atWindowPoint: CGPoint(x: 1134, y: 400)) == nil, "fractional right bar")
    }

    static func invalidInputs() throws {
        for value: CGFloat in [0, -1, .nan, .infinity, -.infinity] {
            try rejects(.invalidContentRect, rect: CGRect(x: 0, y: 0, width: value, height: 600))
            try rejects(.invalidContentRect, rect: CGRect(x: 0, y: 0, width: 800, height: value))
            try rejects(.invalidBackingScale, rect: CGRect(x: 0, y: 0, width: 800, height: 600), scale: value)
        }
        for value: CGFloat in [.nan, .infinity, -.infinity] {
            try rejects(.invalidContentRect, rect: CGRect(x: value, y: 0, width: 800, height: 600))
            try rejects(.invalidContentRect, rect: CGRect(x: 0, y: value, width: 800, height: 600))
        }
        try rejects(.insufficientSpace, rect: CGRect(x: 0, y: 0, width: 0.4, height: 600))
        try rejects(.unrepresentableGeometry, rect: CGRect(x: 0, y: 0, width: 800, height: 600), scale: .greatestFiniteMagnitude)
        try rejects(.insufficientSpace, rect: CGRect(x: 0, y: 0, width: 800, height: 600), scale: .leastNonzeroMagnitude)
        try rejects(.unrepresentableGeometry, rect: CGRect(x: 1e300, y: 0, width: 800, height: 600))

        for axis in [Policy.YAxis.up, .down] {
            let p = try Policy(contentRect: CGRect(x: -20, y: -30, width: 800, height: 600),
                               backingScale: 1, mode: .continuousFit, windowYAxis: axis)
            for value: CGFloat in [.nan, .infinity, -.infinity] {
                for point in [CGPoint(x: value, y: 100), CGPoint(x: 100, y: value)] {
                    try expect(p.originalPoint(atWindowPoint: point) == nil, "invalid window point")
                    try expect(p.originalPixel(atWindowPoint: point) == nil, "invalid engine point")
                    try expect(p.windowPoint(forOriginalPoint: point) == nil, "invalid inverse point")
                }
            }
            for point in [CGPoint(x: -0.001, y: 0), CGPoint(x: 800, y: 0),
                          CGPoint(x: 0, y: -0.001), CGPoint(x: 0, y: 600)] {
                try expect(p.windowPoint(forOriginalPoint: point) == nil, "invalid original extent")
            }
            for pixel in [Policy.Pixel(x: -1, y: 0), .init(x: 800, y: 0),
                          .init(x: 0, y: -1), .init(x: 0, y: 600),
                          .init(x: Int.max, y: Int.min)] {
                try expect(p.windowPoint(forOriginalPixel: pixel) == nil, "invalid pixel inverse")
            }
        }
    }

    static func boundedInverses() throws {
        var layouts = 0
        for size in [CGSize(width: 800, height: 600), CGSize(width: 1200, height: 800),
                     CGSize(width: 900, height: 1200), CGSize(width: 1921, height: 1081)] {
            for backing: CGFloat in [1, 1.5, 2] {
                for mode in [Policy.Mode.integerFit, .continuousFit] {
                    for axis in [Policy.YAxis.up, .down] {
                        let p = try Policy(contentRect: CGRect(origin: CGPoint(x: -37.25, y: 105.5), size: size),
                                           backingScale: backing, mode: mode, windowYAxis: axis)
                        layouts += 1
                        try expect(p.viewportInBacking.minX >= 0 && p.viewportInBacking.minY >= 0 &&
                                   p.viewportInBacking.maxX <= p.backingSize.width + 1e-9 &&
                                   p.viewportInBacking.maxY <= p.backingSize.height + 1e-9, "viewport fits")
                        if mode == .integerFit {
                            try expect(p.scale >= 1 && p.scale == p.scale.rounded(.down), "whole backing multiple")
                            try expect(p.viewportInBacking.minX == p.viewportInBacking.minX.rounded(.down) &&
                                       p.viewportInBacking.minY == p.viewportInBacking.minY.rounded(.down), "aligned backing origin")
                        }
                        for original in [CGPoint(x: 0, y: 0), CGPoint(x: 0.25, y: 0.75),
                                         CGPoint(x: 123.25, y: 456.75), CGPoint(x: 799.5, y: 599.5)] {
                            let window = p.windowPoint(forOriginalPoint: original)
                            guard let window = window else { throw Failure(description: "inverse missing") }
                            try point(p.originalPoint(atWindowPoint: window), original.x, original.y, "bounded continuous inverse")
                        }
                        // Independently sampled window points strictly inside pixel
                        // cells: roundoff on an exact cell line can choose either cell.
                        for u: CGFloat in [0.001, 0.1234, 0.5003, 0.999] {
                            for v: CGFloat in [0.001, 0.3753, 0.999] {
                                let r = p.viewportInWindow
                                let window = CGPoint(x: r.minX + u * r.width,
                                                     y: axis == .up ? r.maxY - v * r.height : r.minY + v * r.height)
                                guard let original = p.originalPoint(atWindowPoint: window),
                                      let pixel = p.originalPixel(atWindowPoint: window) else {
                                    throw Failure(description: "sampled window point missing")
                                }
                                try point(original, u * 800, v * 600, "sampled window reference")
                                try point(p.windowPoint(forOriginalPoint: original), window.x, window.y, "bounded window inverse")
                                try expect(pixel.x == Int((u * 800).rounded(.down)) &&
                                           pixel.y == Int((v * 600).rounded(.down)), "sampled window quantization")
                            }
                        }
                        for x in stride(from: 0, through: 799, by: 37) {
                            for y in stride(from: 0, through: 599, by: 29) {
                                let pixel = Policy.Pixel(x: x, y: y)
                                guard let window = p.windowPoint(forOriginalPixel: pixel),
                                      let original = p.originalPoint(atWindowPoint: window),
                                      let quantized = p.originalPixel(atWindowPoint: window) else {
                                    throw Failure(description: "pixel center missing")
                                }
                                try near(original.x, CGFloat(x) + 0.5, "pixel-center x")
                                try near(original.y, CGFloat(y) + 0.5, "pixel-center y")
                                try expect(quantized == pixel, "pixel-center inverse")
                                try expect(p.clampedOriginalPixel(atWindowPoint: window) == pixel, "held drag inside equals hit test")
                                try expect(original.x - CGFloat(quantized.x) >= 0 && original.x - CGFloat(quantized.x) < 1 &&
                                           original.y - CGFloat(quantized.y) >= 0 && original.y - CGFloat(quantized.y) < 1, "quantization error below one original pixel")
                            }
                        }
                    }
                }
            }
        }
        try expect(layouts == 48, "bounded layout count")
    }

    /// A held drag that leaves the viewport reaches the nearest edge pixel; a
    /// fresh hit test outside still returns nil (bars never become game input).
    static func clampedDrag() throws {
        for axis in [Policy.YAxis.up, .down] {
            for backing: CGFloat in [1, 2] {
                let p = try Policy(contentRect: CGRect(x: 10, y: 105, width: 1200, height: 800),
                                   backingScale: backing, mode: .continuousFit, windowYAxis: axis)
                let r = p.viewportInWindow
                guard let center = p.windowPoint(forOriginalPixel: Policy.Pixel(x: 400, y: 300)) else { throw Failure(description: "center pixel") }
                let top = axis == .up ? r.maxY + 50 : r.minY - 50
                let bottom = axis == .up ? r.minY - 50 : r.maxY + 50
                let cases: [(CGPoint, Int, Int)] = [
                    (CGPoint(x: r.minX - 40, y: top), 0, 0),
                    (CGPoint(x: r.maxX + 40, y: bottom), 799, 599),
                    (CGPoint(x: r.minX - 1e6, y: center.y), 0, 300),
                    (CGPoint(x: center.x, y: bottom), 400, 599),
                    (CGPoint(x: r.maxX, y: center.y), 799, 300)]
                for (window, x, y) in cases {
                    try expect(p.clampedOriginalPixel(atWindowPoint: window) == Policy.Pixel(x: x, y: y), "clamped drag \(x),\(y)")
                    if window.x != r.maxX { try expect(p.originalPixel(atWindowPoint: window) == nil, "outside hit test stays nil") }
                }
                try expect(p.clampedOriginalPixel(atWindowPoint: CGPoint(x: CGFloat.nan, y: 0)) == nil, "non-finite drag point")
                try expect(p.clampedOriginalPixel(atWindowPoint: CGPoint(x: CGFloat.infinity, y: 0)) == nil, "infinite drag point")
            }
        }
    }

    static func main() {
        do {
            try fixedReferences()
            try boundaries()
            try invalidInputs()
            try boundedInverses()
            try clampedDrag()
            print("ViewportPolicyTests: \(checks) checks passed; 48 layouts; continuous error <= 1e-9 original pixels; quantization error < 1 original pixel")
        } catch {
            FileHandle.standardError.write(Data("ViewportPolicyTests FAILED: \(error)\n".utf8))
            exit(1)
        }
    }
}
