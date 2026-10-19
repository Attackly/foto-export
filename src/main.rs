use std::{fs, path::Path};

use clap::Parser;
use image::{GenericImageView, imageops::{FilterType, overlay}};
use scan_dir::ScanDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input dir
    #[arg(short, long)]
    input: String,

    /// Output dir
    #[arg(short, long)]
    output: String,

    #[arg(short, long)]
    logo: String,

    #[arg(short, long)]
    format: String,

    #[arg(short, long, default_value_t = 80.0)]
    quality: f32,

    #[arg(short, long, default_value_t = false)]
    destructive: bool,

    #[arg(short, long, default_value_t = false)]
    nologo: bool,

    #[arg(long, default_value_t = false)]
    noresize: bool,
}

fn main() {
    let args = Args::parse();

    
    println!("Input: {}", args.input);
    println!("Output: {}", args.output);
    println!("Logo: {}", args.logo);
    println!("Format: {}", args.format);
    println!("Quality: {}", args.quality);


    let files: Vec<_> = ScanDir::files().read(".", |iter| {
        iter.filter(|&(_, ref name)| name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png"))
            .map(|(entry, _)| entry.path())
            .collect()
    }).unwrap();

    for file in files {

        let mut img = image::open(file.clone()).unwrap();


        img = if (img.width() > 1280 || img.height() > 720) && !args.noresize {
            img.resize(1280, 720, FilterType::Lanczos3)
        }
        else {
            img // If Image is too small or should not be resized, keep it as is
        };

        let watermark_width = (img.width() * 45 / 100).max(1);


        let watermark = load_svg_watermark(
            Path::new(&args.logo),
            watermark_width,    
        ).unwrap();

        let x = img.width().saturating_sub(watermark.width())  / 2;
        let y = img.height().saturating_sub(watermark.height()) / 2;

        overlay(&mut img, &watermark, i64::from(x), i64::from(y));

        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();

        // 3. Create a lossy WebP encoder
        let encoder = webp::Encoder::from_rgba(rgba.as_raw(), width, height);

        // 4. Encode; 80 is a sensible starting quality
        let webp_data = encoder.encode(args.quality);

        let mut output = file.clone();
            output.set_file_name(format!(
            "{}-web.webp",
            file.file_stem().unwrap().to_string_lossy()
        ));

        if output.exists() && !args.destructive {
            println!("File {} already exists, skipping", output.display());
            continue;
        }
    
        // 5. Save Image as Webp
        fs::write(output, &*webp_data).unwrap();
    }
}


use image::{Rgba, RgbaImage};
use resvg::{tiny_skia, usvg};

fn load_svg_watermark(
    path: &Path,
    target_width: u32,
) -> Result<RgbaImage, Box<dyn std::error::Error>> {
    let svg_data = std::fs::read(path)?;

    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options)?;

    let width = target_width.max(1);
    let scale = width as f32 / tree.size().width();
    let height = (tree.size().height() * scale).ceil().max(1.0) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or("Could not allocate watermark image")?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // tiny-skia uses premultiplied alpha; image expects straight alpha.
    let mut watermark = RgbaImage::new(width, height);

    for (destination, source) in
        watermark.pixels_mut().zip(pixmap.pixels())
    {
        let color = source.demultiply();

        *destination = Rgba([
            255,
            255,
            255,
            (f32::from(color.alpha()) * 0.50) as u8,
        ]);
    }

    Ok(watermark)
}