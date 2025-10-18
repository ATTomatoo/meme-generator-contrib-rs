use skia_safe::{Image, Paint, Rect, Shader, TileMode};
use meme_generator_core::error::Error;
use meme_generator_utils::{
    builder::{InputImage, MemeOptions},
    image::ImageExt,
    tools::{load_image, local_date, new_surface},
};
use crate::register_meme;

#[derive(MemeOptions)]
struct Louvre {
    /// 是否开启降噪
    #[option(long, default = false)]
    denoise: Option<bool>,
}

/// 模拟滤镜蒙版
fn make_mask(base: &Image, pencil: &Image, denoise: bool) -> Result<Image, Error> {
    // 从 base 取尺寸
    let size = base.dimensions();
    let width = size.width;
    let height = size.height;

    // new_surface 以 (i32, i32) 或 ISize 接受尺寸，视库实现而定 - 这里使用 tuple 保证兼容
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    // 先绘制 base，再绘制 pencil（作为叠加）
    canvas.draw_image(base, (0, 0), None);
    canvas.draw_image(pencil, (0, 0), None);

    if denoise {
        // 如果 ImageExt 提供模糊函数则调用；不同版本名可能不同（blur_image_auto / blur）
        // 保持 ? 让错误上抛
        let blurred = base.blur_image_auto(3.0)?;
        canvas.draw_image(&blurred, (0, 0), None);
    }

    Ok(surface.image_snapshot())
}

/// 创建渐变背景
fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    let positions = [0.0, 0.4, 0.6, 0.7, 0.8, 1.0];
    let colors = [
        skia_safe::Color4f::new(0.984, 0.729, 0.188, 1.0),
        skia_safe::Color4f::new(0.988, 0.447, 0.207, 1.0),
        skia_safe::Color4f::new(0.988, 0.207, 0.305, 1.0),
        skia_safe::Color4f::new(0.811, 0.211, 0.874, 1.0),
        skia_safe::Color4f::new(0.215, 0.709, 0.850, 1.0),
        skia_safe::Color4f::new(0.243, 0.713, 0.854, 1.0),
    ];

    let shader = Shader::linear_gradient(
        (0.0, 0.0),
        (width as f32, height as f32),
        &colors,
        Some(&positions),
        TileMode::Clamp,
        None,
        None,
    );

    let mut paint = Paint::default();
    paint.set_shader(shader);

    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, width as f32, height as f32), &paint);

    Ok(surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, options: Louvre) -> Result<Vec<u8>, Error> {
    // InputImage -> Image
    let base = images[0].as_image()?;
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载素材 01.png（变量名用 pencil）
    let pencil = load_image("louvre/01.png")?.resize_exact((w, h));

    // 渐变背景
    let gradient = make_gradient(w, h)?;

    // 蒙版
    let mask = make_mask(&base, &pencil, options.denoise.unwrap_or(false))?;

    // 合成结果
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    canvas.draw_image(&gradient, (0, 0), None);
    // 这里简单把 mask 当作一层绘制（如果你需要把 mask 当成 alpha mask，需要额外的遮罩/paint 逻辑）
    canvas.draw_image(&mask, (0, 0), None);

    let result = surface.image_snapshot();

    let png_data = result
        .encode_to_data(skia_safe::EncodedImageFormat::PNG)
        .ok_or_else(|| Error::Other("生成 PNG 失败".into()))?;

    Ok(png_data.as_bytes().to_vec())
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫"],
    // tags_from helper: 若你的版本不匹配，可以改成 tags = meme_generator_utils::builder::meme_setters::tags_from(["艺术","滤镜","素描"])
    tags = meme_generator_utils::builder::meme_setters::tags_from(["艺术", "滤镜", "素描"]),
    date_created = local_date(2025, 10, 19),
    date_modified = local_date(2025, 10, 19),
);
