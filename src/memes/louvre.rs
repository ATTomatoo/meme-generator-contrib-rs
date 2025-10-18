use std::collections::HashSet;

use skia_safe::{Color4f, Image, Paint, Rect, Shader, TileMode, BlendMode, Color};
use meme_generator_core::error::Error;
use meme_generator_utils::{
    builder::{InputImage, MemeOptions},
    encoder::make_png_or_gif,
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

/// 简化的素描效果实现
fn make_sketch_effect(base: &Image, pencil: &Image) -> Result<Image, Error> {
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);
    
    // 步骤1: 创建灰度版本
    let mut gray_surface = new_surface((w, h));
    let gray_canvas = gray_surface.canvas();
    
    // 使用luma颜色滤镜创建灰度效果
    if let Some(luma_filter) = skia_safe::ColorFilter::luma() {
        let mut paint = Paint::default();
        paint.set_color_filter(luma_filter);
        gray_canvas.draw_image(base, (0, 0), Some(&paint));
    } else {
        gray_canvas.draw_image(base, (0, 0), None);
    }
    
    let gray_image = gray_surface.image_snapshot();
    
    // 步骤2: 创建高对比度版本（模拟素描线条）
    let mut high_contrast_surface = new_surface((w, h));
    let high_contrast_canvas = high_contrast_surface.canvas();
    
    // 使用高对比度滤镜
    let mut contrast_config = skia_safe::HighContrastConfig::default();
    contrast_config.grayscale = true;
    contrast_config.contrast = 1.5; // 高对比度
    
    if let Some(contrast_filter) = skia_safe::ColorFilter::high_contrast(&contrast_config) {
        let mut paint = Paint::default();
        paint.set_color_filter(contrast_filter);
        high_contrast_canvas.draw_image(&gray_image, (0, 0), Some(&paint));
    } else {
        high_contrast_canvas.draw_image(&gray_image, (0, 0), None);
    }
    
    let high_contrast_image = high_contrast_surface.image_snapshot();
    
    // 步骤3: 反转图像（素描效果）
    let mut invert_surface = new_surface((w, h));
    let invert_canvas = invert_surface.canvas();
    
    // 使用混合模式模拟反转效果
    invert_canvas.clear(Color::WHITE);
    
    let mut invert_paint = Paint::default();
    invert_paint.set_blend_mode(BlendMode::Difference);
    invert_canvas.draw_image(&high_contrast_image, (0, 0), Some(&invert_paint));
    
    let inverted = invert_surface.image_snapshot();
    
    // 步骤4: 与铅笔纹理混合
    let mut final_surface = new_surface((w, h));
    let final_canvas = final_surface.canvas();
    
    // 先绘制素描效果
    final_canvas.draw_image(&inverted, (0, 0), None);
    
    // 使用Multiply混合模式叠加铅笔纹理
    let mut pencil_paint = Paint::default();
    pencil_paint.set_blend_mode(BlendMode::Multiply);
    final_canvas.draw_image(pencil, (0, 0), Some(&pencil_paint));
    
    Ok(final_surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, _options: Louvre) -> Result<Vec<u8>, Error> {
    let frame = load_image("louvre/01.png")?;

    let func = |images: Vec<Image>| {
        let base = &images[0];
        let size = base.dimensions();
        let (w, h) = (size.width, size.height);

        // 调整铅笔纹理大小
        let pencil = frame.resize_exact((w, h));

        // 创建渐变背景
        let gradient = make_gradient(w, h)?;

        // 创建素描效果蒙版
        let sketch_mask = make_sketch_effect(base, &pencil)?;

        // 创建结果表面
        let mut surface = new_surface((w, h));
        let canvas = surface.canvas();

        // 先绘制渐变背景
        canvas.draw_image(&gradient, (0, 0), None);
        
        // 使用素描效果作为蒙版
        let mut mask_paint = Paint::default();
        mask_paint.set_blend_mode(BlendMode::Multiply);
        canvas.draw_image(&sketch_mask, (0, 0), Some(&mask_paint));

        Ok(surface.image_snapshot())
    };

    make_png_or_gif(images, func)
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫"],
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