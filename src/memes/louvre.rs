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
    /// 是否开启降噪
    #[option(long, default = false)]
    denoise: Option<bool>,
}

/// 创建渐变背景
fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    let colors: [Color4f; 6] = [
        Color4f::new(0.984, 0.729, 0.188, 1.0),
        Color4f::new(0.988, 0.447, 0.207, 1.0),
        Color4f::new(0.988, 0.207, 0.305, 1.0),
        Color4f::new(0.811, 0.211, 0.874, 1.0),
        Color4f::new(0.215, 0.709, 0.850, 1.0),
        Color4f::new(0.243, 0.713, 0.854, 1.0),
    ];

    let shader = Shader::linear_gradient(
        ((0.0, 0.0), (width as f32, height as f32)),
        &colors[..],
        None,
        TileMode::Clamp,
        None,
        None,
    );

    let mut paint = Paint::default();
    paint.set_shader(shader);

    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, width as f32, height as f32), &paint);

    Ok(surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, _options: Louvre) -> Result<Vec<u8>, Error> {
    // 使用和参考代码相同的方式处理输入图片
    let base = images.into_iter().next().unwrap().to_image()?;
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载卢浮宫滤镜层（已改名为01.png）
    let filter = load_image("louvre/01.png")?.resize_exact((w, h));

    // 创建渐变背景
    let gradient = make_gradient(w, h)?;

    // 创建结果表面
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();

    // 绘制渐变背景
    canvas.draw_image(&gradient, (0, 0), None);
    
    // 绘制原始图像
    canvas.draw_image(&base, (0, 0), None);
    
    // 叠加滤镜效果
    canvas.draw_image(&filter, (0, 0), None);

    let result = surface.image_snapshot();

    // 编码为PNG - 使用和参考代码相同的错误处理方式
    let png_data = result
        .encode(None, EncodedImageFormat::PNG, 100)
        .ok_or_else(|| Error::from("Failed to encode image"))?;

    Ok(png_data.as_bytes().to_vec())
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫", "艺术", "滤镜"],
    tags = {
        let mut tags = HashSet::new();
        tags.insert("艺术".to_string());
        tags.insert("滤镜".to_string());
        tags.insert("素描".to_string());
        tags
    },
    date_created = local_date(2025, 10, 19),
    date_modified = local_date(2025, 10, 19),
);