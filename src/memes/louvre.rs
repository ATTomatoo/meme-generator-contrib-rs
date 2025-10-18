use std::collections::HashSet;

use skia_safe::{Color4f, EncodedImageFormat, Image, Paint, Rect, Shader, TileMode};
use meme_generator_core::error::Error;
use meme_generator_utils::{
    builder::{InputImage, MemeOptions},
    image::ImageExt,
    tools::{load_image, local_date, new_surface},
};
use crate::register_meme;

#[derive(MemeOptions)]
struct Louvre {
    /// 是否开启降噪（当前实现忽略该选项以保证兼容）
    #[option(long, default = false)]
    denoise: Option<bool>,
}

/// 模拟滤镜蒙版（为保证兼容性，暂不做模糊处理）
fn make_mask(base: &Image, pencil: &Image, _denoise: bool) -> Result<Image, Error> {
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();

    // 直接将 base 与 pencil 叠加（简化实现）
    canvas.draw_image(base, (0, 0), None);
    canvas.draw_image(pencil, (0, 0), None);

    Ok(surface.image_snapshot())
}

/// 创建渐变背景（使用 linear_gradient 的标准 6 参数签名）
fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    // 颜色数组，平均分布（转换为切片以符合 API 要求）
    let colors: [Color4f; 6] = [
        Color4f::new(0.984, 0.729, 0.188, 1.0),
        Color4f::new(0.988, 0.447, 0.207, 1.0),
        Color4f::new(0.988, 0.207, 0.305, 1.0),
        Color4f::new(0.811, 0.211, 0.874, 1.0),
        Color4f::new(0.215, 0.709, 0.850, 1.0),
        Color4f::new(0.243, 0.713, 0.854, 1.0),
    ];

    // linear_gradient 需要切片而不是数组引用
    let shader = Shader::linear_gradient(
        ((0.0f32, 0.0f32), (width as f32, height as f32)),
        &colors[..],          // 转换为切片
        None,                 // positions (None => 平均分布)
        TileMode::Clamp,      // tile mode
        None,                 // flags
        None,                 // local matrix
    );

    let mut paint = Paint::default();
    paint.set_shader(shader);

    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, width as f32, height as f32), &paint);

    Ok(surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, options: Louvre) -> Result<Vec<u8>, Error> {
    // 直接消费 images vec，取第一个 InputImage 并转换为 skia Image
    let first_input = images.into_iter().next().unwrap();
    let base: Image = first_input.to_image()?; // 使用 to_image() 而不是 into_image()

    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载素材 01.png 并缩放到目标尺寸
    let pencil = load_image("louvre/01.png")?.resize_exact((w, h));

    // 渐变背景
    let gradient = make_gradient(w, h)?;

    // 蒙版
    let mask = make_mask(&base, &pencil, options.denoise.unwrap_or(false))?;

    // 合成结果：先画 gradient，再把 mask 作为一层绘制
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    canvas.draw_image(&gradient, (0, 0), None);
    canvas.draw_image(&mask, (0, 0), None);

    let result = surface.image_snapshot();

    // 使用新的 encode_to_data_with_context API 避免弃用警告
    let png_data = result
        .encode_to_data_with_context(None, EncodedImageFormat::PNG, 100)
        .unwrap();

    Ok(png_data.as_bytes().to_vec())
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫"],
    // macro 需要 HashSet<String>：在宏中直接构造一个 HashSet 以保证兼容
    tags = {
        let mut __tags = HashSet::new();
        __tags.insert("艺术".to_string());
        __tags.insert("滤镜".to_string());
        __tags.insert("素描".to_string());
        __tags
    },
    date_created = local_date(2025, 10, 19),
    date_modified = local_date(2025, 10, 19),
);