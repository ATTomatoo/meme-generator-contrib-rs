use std::collections::HashSet;

use skia_safe::{Color4f, EncodedImageFormat, Image, Paint, Rect, Shader, TileMode, BlendMode};
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

/// 简化版的蒙版创建，使用混合模式模拟素描效果
fn make_mask(base: &Image, pencil: &Image) -> Result<Image, Error> {
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 创建灰度版本的基础图像
    let mut gray_surface = new_surface((w, h));
    let gray_canvas = gray_surface.canvas();
    
    // 使用颜色矩阵将图像转换为灰度
    let gray_matrix = [
        0.299, 0.587, 0.114, 0.0, 0.0,
        0.299, 0.587, 0.114, 0.0, 0.0,
        0.299, 0.587, 0.114, 0.0, 0.0,
        0.0,   0.0,   0.0,   1.0, 0.0,
    ];
    
    let mut gray_paint = Paint::default();
    if let Some(color_filter) = skia_safe::ColorFilter::matrix(&gray_matrix) {
        gray_paint.set_color_filter(color_filter);
    }
    
    gray_canvas.draw_image(base, (0, 0), Some(&gray_paint));
    let gray_base = gray_surface.image_snapshot();

    // 反转图像（素描效果）
    let mut invert_surface = new_surface((w, h));
    let invert_canvas = invert_surface.canvas();
    
    let invert_matrix = [
        -1.0, 0.0, 0.0, 0.0, 1.0,
        0.0, -1.0, 0.0, 0.0, 1.0,
        0.0, 0.0, -1.0, 0.0, 1.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    
    let mut invert_paint = Paint::default();
    if let Some(invert_filter) = skia_safe::ColorFilter::matrix(&invert_matrix) {
        invert_paint.set_color_filter(invert_filter);
    }
    
    invert_canvas.draw_image(&gray_base, (0, 0), Some(&invert_paint));
    let inverted = invert_surface.image_snapshot();

    // 将铅笔纹理与反转图像混合
    let mut final_surface = new_surface((w, h));
    let final_canvas = final_surface.canvas();
    
    // 先绘制反转的基础图像
    final_canvas.draw_image(&inverted, (0, 0), None);
    
    // 使用Multiply混合模式叠加铅笔纹理
    let mut pencil_paint = Paint::default();
    pencil_paint.set_blend_mode(BlendMode::Multiply);
    final_canvas.draw_image(pencil, (0, 0), Some(&pencil_paint));

    Ok(final_surface.image_snapshot())
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

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, options: Louvre) -> Result<Vec<u8>, Error> {
    // 使用正确的方法转换InputImage
    let base = images.into_iter().next().unwrap().into_image()?;
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载铅笔纹理（已改名为01.png）
    let pencil = load_image("louvre/01.png")?.resize_exact((w, h));

    // 创建渐变背景
    let gradient = make_gradient(w, h)?;

    // 创建蒙版，解包denoise选项
    let denoise = options.denoise.unwrap_or(false);
    let mask = make_mask(&base, &pencil)?;

    // 最终合成：渐变背景 + 蒙版
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    
    // 先绘制渐变背景
    canvas.draw_image(&gradient, (0, 0), None);
    
    // 使用Multiply混合模式叠加蒙版
    let mut mask_paint = Paint::default();
    mask_paint.set_blend_mode(BlendMode::Multiply);
    canvas.draw_image(&mask, (0, 0), Some(&mask_paint));

    let result = surface.image_snapshot();

    // 编码为PNG，使用正确的错误处理
    let png_data = result
        .encode(None, EncodedImageFormat::PNG, 100)
        .ok_or_else(|| Error::msg("Failed to encode image"))?;

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