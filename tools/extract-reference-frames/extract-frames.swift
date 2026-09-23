// usage: swift frames.swift <video> <outdir> <fps> [x y w h] [scale]
import AVFoundation
import CoreGraphics
import ImageIO
import UniformTypeIdentifiers

let a = CommandLine.arguments
let url = URL(fileURLWithPath: a[1])
let out = a[2]
let fps = Double(a[3])!
var crop: CGRect? = nil
if a.count >= 8 { crop = CGRect(x: Double(a[4])!, y: Double(a[5])!, width: Double(a[6])!, height: Double(a[7])!) }
let scale = a.count >= 9 ? Double(a[8])! : 1.0

let asset = AVURLAsset(url: url)
let gen = AVAssetImageGenerator(asset: asset)
gen.appliesPreferredTrackTransform = true
gen.requestedTimeToleranceBefore = .zero
gen.requestedTimeToleranceAfter = .zero
let dur = CMTimeGetSeconds(asset.duration)
var t = 0.0, i = 0
while t < dur {
  if var img = try? gen.copyCGImage(at: CMTime(seconds: t, preferredTimescale: 600), actualTime: nil) {
    if let c = crop, let g = img.cropping(to: c) { img = g }
    let w = Int(Double(img.width) * scale), h = Int(Double(img.height) * scale)
    let ctx = CGContext(data: nil, width: w, height: h, bitsPerComponent: 8, bytesPerRow: 0,
                        space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = scale >= 1 ? .none : .high
    ctx.draw(img, in: CGRect(x: 0, y: 0, width: w, height: h))
    let d = CGImageDestinationCreateWithURL(URL(fileURLWithPath: String(format: "%@/f%03d.png", out, i)) as CFURL, UTType.png.identifier as CFString, 1, nil)!
    CGImageDestinationAddImage(d, ctx.makeImage()!, nil)
    CGImageDestinationFinalize(d)
  }
  t += 1.0 / fps
  i += 1
}
print("frames:", i, "dur:", dur)
