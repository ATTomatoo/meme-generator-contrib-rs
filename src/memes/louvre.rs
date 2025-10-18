use skia_safe::{Image, ColorType, AlphaType};
use meme_generator_core::error::Error;
use meme_generator_utils::{
    builder::{InputImage, MemeOptions},
    encoder::{make_gif_or_combined_gif, FrameAlign, GifInfo},
    image::ImageExt,
    tools::{load_image, local_date, new_surface},
};
use crate::register_meme;

#[derive(MemeOptions)]
struct Louvre {
    // 是否开启降噪
    #[option(long, default = false)]
    denoise: Option<bool>,
}

fn make_mask(base: &Image, 01: &Image, denoise: bool) -> Result<Image, Error> {
    // 模拟 Python 里的 mask 算法
    let (w, h) = base.dimensions();
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();

    // 模拟图像融合（使用 multiply blend）
    canvas.draw_image(base, (0, 0), None);
    canvas.draw_image(01, (0, 0), None);

    if denoise {
        // 模拟简单的模糊去噪效果
        let blurred = base.blur_image(3.0)?;
        canvas.draw_image(&blurred, (0, 0), None);
    }

    Ok(surface.image_snapshot())
}

fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    // 创建渐变背景
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    let colors = [
        (0.0, (0.984, 0.729, 0.188)),
        (0.4, (0.988, 0.447, 0.207)),
        (0.6, (0.988, 0.207, 0.305)),
        (0.7, (0.811, 0.211, 0.874)),
        (0.8, (0.215, 0.709, 0.850)),
        (1.0, (0.243, 0.713, 0.854)),
    ];

    let shader = skia_safe::shader::LinearGradient::new(
        (0.0, 0.0),
        (width as f32, height as f32),
        colors
            .iter()
            .map(|(_, (r, g, b))| skia_safe::Color4f::new(*r, *g, *b, 1.0))
            .collect::<Vec<_>>()
            .as_slice(),
        colors.iter().map(|(p, _)| *p).collect::<Vec<_>>().as_slice(),
        skia_safe::tile_mode::TileMode::Clamp,
        None,
        None,
    )
    .ok_or_else(|| Error::msg("无法创建渐变"))?;

    let paint = skia_safe::Paint::default().with_shader(shader);
    canvas.draw_rect((0.0, 0.0, width as f32, height as f32), &paint);

    Ok(surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, options: Louvre) -> Result<Vec<u8>, Error> {
    let base = images[0].to_image()?;
    let (w, h) = base.dimensions();

    // 加载素材 01.png
    let 01 = load_image("louvre/01.png")?.resize_exact((w, h));

    // 渐变背景
    let gradient = make_gradient(w, h)?;

    // 蒙版
    let mask = make_mask(&base, &01, options.denoise.unwrap_or(false))?;

    // 生成合成图
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();

    canvas.draw_image(&gradient, (0, 0), None);
    canvas.draw_image_with_mask(&base, &mask, (0, 0));

    let result = surface.image_snapshot();

    // 输出为 PNG
    let png_data = result.encode_to_data_with_format(skia_safe::EncodedImageFormat::PNG)
        .ok_or_else(|| Error::msg("生成 PNG 失败"))?;

    Ok(png_data.as_bytes().to_vec())
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫"],
    tags = &["艺术", "滤镜", "素描"],
    date_created = local_date(2025, 10, 19),
    date_modified = local_date(2025, 10, 19),
);
