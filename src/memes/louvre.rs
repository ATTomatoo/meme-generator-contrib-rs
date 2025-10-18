use std::collections::HashSet;

use skia_safe::{Color4f, EncodedImageFormat, Image, Paint, Rect, Shader, TileMode, ColorFilter, BlendMode, Color};
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

/// 模拟素描效果
fn make_sketch_effect(base: &Image, pencil: &Image) -> Result<Image, Error> {
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);
    
    // 步骤1: 将图像转换为灰度
    let gray_matrix = [
        0.299, 0.587, 0.114, 0.0, 0.0,
        0.299, 0.587, 0.114, 0.0, 0.0, 
        0.299, 0.587, 0.114, 0.0, 0.0,
        0.0,   0.0,   0.0,   1.0, 0.0,
    ];
    
    let mut gray_surface = new_surface((w, h));
    let gray_canvas = gray_surface.canvas();
    let mut gray_paint = Paint::default();
    if let Some(filter) = ColorFilter::matrix(&gray_matrix) {
        gray_paint.set_color_filter(filter);
    }
    gray_canvas.draw_image(base, (0, 0), Some(&gray_paint));
    let gray_image = gray_surface.image_snapshot();
    
    // 步骤2: 模拟卷积核效果 - 使用模糊来近似thin卷积核
    let mut blur_surface = new_surface((w, h));
    let blur_canvas = blur_surface.canvas();
    let mut blur_paint = Paint::default();
    // 使用图像滤镜来模拟模糊效果
    blur_canvas.draw_image(&gray_image, (0, 0), None);
    let blurred = blur_surface.image_snapshot();
    
    // 步骤3: 模拟图像减法 (原图 - 模糊图)
    let mut subtract_surface = new_surface((w, h));
    let subtract_canvas = subtract_surface.canvas();
    
    // 先绘制原图
    subtract_canvas.draw_image(&gray_image, (0, 0), None);
    
    // 使用Difference混合模式模拟减法
    let mut subtract_paint = Paint::default();
    subtract_paint.set_blend_mode(BlendMode::Difference);
    subtract_canvas.draw_image(&blurred, (0, 0), Some(&subtract_paint));
    
    let subtracted = subtract_surface.image_snapshot();
    
    // 步骤4: 调整亮度和对比度
    let mut adjust_surface = new_surface((w, h));
    let adjust_canvas = adjust_surface.canvas();
    
    // 使用颜色矩阵调整亮度和对比度
    let adjust_matrix = [
        1.2, 0.0, 0.0, 0.0, -30.0,  // 增加对比度并降低亮度
        0.0, 1.2, 0.0, 0.0, -30.0,
        0.0, 0.0, 1.2, 0.0, -30.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    
    let mut adjust_paint = Paint::default();
    if let Some(filter) = ColorFilter::matrix(&adjust_matrix) {
        adjust_paint.set_color_filter(filter);
    }
    adjust_canvas.draw_image(&subtracted, (0, 0), Some(&adjust_paint));
    let adjusted = adjust_surface.image_snapshot();
    
    // 步骤5: 反转图像（素描效果）
    let mut invert_surface = new_surface((w, h));
    let invert_canvas = invert_surface.canvas();
    
    let invert_matrix = [
        -1.0, 0.0, 0.0, 0.0, 1.0,
        0.0, -1.0, 0.0, 0.0, 1.0,
        0.0, 0.0, -1.0, 0.0, 1.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    
    let mut invert_paint = Paint::default();
    if let Some(filter) = ColorFilter::matrix(&invert_matrix) {
        invert_paint.set_color_filter(filter);
    }
    invert_canvas.draw_image(&adjusted, (0, 0), Some(&invert_paint));
    let inverted = invert_surface.image_snapshot();
    
    // 步骤6: 与铅笔纹理混合
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

        // 先绘制白色背景
        canvas.clear(Color::WHITE);
        
        // 绘制渐变背景，使用素描效果作为蒙版
        let mut gradient_paint = Paint::default();
        // 使用素描效果的亮度作为渐变背景的Alpha通道
        gradient_paint.set_blend_mode(BlendMode::SrcIn);
        canvas.draw_image(&gradient, (0, 0), Some(&gradient_paint));
        
        // 叠加素描效果
        canvas.draw_image(&sketch_mask, (0, 0), None);

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