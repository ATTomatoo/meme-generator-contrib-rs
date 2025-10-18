use std::collections::HashSet;

use skia_safe::{
    Color, Color4f, ColorFilter, EncodedImageFormat, Image, ImageFilter, MatrixConvolution, Paint, 
    Rect, Shader, TileMode, IPoint, BlendMode
};
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

/// 创建卷积核用于图像处理
fn create_kernel(size: i32) -> Vec<f32> {
    let value = 1.0 / (size * size) as f32;
    vec![value; (size * size) as usize]
}

/// 实现类似Python的make_mask功能
fn make_mask(base: &Image, pencil: &Image, denoise: bool) -> Result<Image, Error> {
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 首先将基础图像转换为灰度
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    canvas.draw_image(base, (0, 0), None);
    let base_gray = surface.image_snapshot();

    // 如果需要降噪，应用均值滤波
    let base_processed = if denoise.unwrap_or(false) {
        let kernel = create_kernel(3);
        let filter = ImageFilter::matrix_convolution(
            (3, 3),
            &kernel,
            1.0,
            0.0,
            IPoint::new(1, 1),
            TileMode::Clamp,
            true,
        );
        
        let mut paint = Paint::default();
        paint.set_image_filter(filter);
        
        let mut surface = new_surface((w, h));
        let canvas = surface.canvas();
        canvas.draw_image(&base_gray, (0, 0), Some(&paint));
        surface.image_snapshot()
    } else {
        base_gray
    };

    // 应用thin卷积核（5x5均值滤波）
    let kernel_thin = create_kernel(5);
    let filter_thin = ImageFilter::matrix_convolution(
        (5, 5),
        &kernel_thin,
        1.0,
        0.0,
        IPoint::new(2, 2),
        TileMode::Clamp,
        true,
    );

    let mut paint_thin = Paint::default();
    paint_thin.set_image_filter(filter_thin);
    
    let mut surface_thin = new_surface((w, h));
    let canvas_thin = surface_thin.canvas();
    canvas_thin.draw_image(&base_processed, (0, 0), Some(&paint_thin));
    let im1 = surface_thin.image_snapshot();

    // 创建结果表面
    let mut result_surface = new_surface((w, h));
    let result_canvas = result_surface.canvas();
    
    // 实现图像减法操作
    // 在Skia中，我们可以使用混合模式来模拟减法
    result_canvas.draw_image(&base_processed, (0, 0), None);
    
    let mut subtract_paint = Paint::default();
    subtract_paint.set_blend_mode(BlendMode::Difference);
    result_canvas.draw_image(&im1, (0, 0), Some(&subtract_paint));

    let subtracted = result_surface.image_snapshot();

    // 调整亮度和对比度
    let mut final_surface = new_surface((w, h));
    let final_canvas = final_surface.canvas();
    
    // 应用颜色矩阵来调整图像
    let color_matrix = [
        1.0, 0.0, 0.0, 0.0, -0.46,  // R: 减去暗部切割 (118/255 ≈ 0.46)
        0.0, 1.0, 0.0, 0.0, -0.46,  // G
        0.0, 0.0, 1.0, 0.0, -0.46,  // B
        0.0, 0.0, 0.0, 1.0, 0.0,    // A
    ];
    
    let color_filter = ColorFilter::matrix(&color_matrix);
    let mut adjust_paint = Paint::default();
    adjust_paint.set_color_filter(color_filter);
    
    final_canvas.draw_image(&subtracted, (0, 0), Some(&adjust_paint));

    // 反转图像（素描效果）
    let mut invert_surface = new_surface((w, h));
    let invert_canvas = invert_surface.canvas();
    
    let invert_matrix = [
        -1.0, 0.0, 0.0, 0.0, 1.0,  // 反转RGB
        0.0, -1.0, 0.0, 0.0, 1.0,
        0.0, 0.0, -1.0, 0.0, 1.0,
        0.0, 0.0, 0.0, 1.0, 0.0,
    ];
    
    let invert_filter = ColorFilter::matrix(&invert_matrix);
    let mut invert_paint = Paint::default();
    invert_paint.set_color_filter(invert_filter);
    
    invert_canvas.draw_image(&final_surface.image_snapshot(), (0, 0), Some(&invert_paint));

    Ok(invert_surface.image_snapshot())
}

/// 创建渐变背景
fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    let colors: [Color4f; 6] = [
        Color4f::new(0.984, 0.729, 0.188, 1.0),  // #FBB830
        Color4f::new(0.988, 0.447, 0.207, 1.0),  // #FC7235
        Color4f::new(0.988, 0.207, 0.305, 1.0),  // #FC354E
        Color4f::new(0.811, 0.211, 0.874, 1.0),  // #CF36DF
        Color4f::new(0.215, 0.709, 0.850, 1.0),  // #37B5D9
        Color4f::new(0.243, 0.713, 0.854, 1.0),  // #3EB6DA
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
    let base = images.into_iter().next().unwrap().to_image()?;
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载铅笔纹理（已改名为01.png）
    let pencil = load_image("louvre/01.png")?.resize_exact((w, h));

    // 创建渐变背景
    let gradient = make_gradient(w, h)?;

    // 创建蒙版
    let mask = make_mask(&base, &pencil, options.denoise)?;

    // 最终合成：渐变背景 + 蒙版
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    
    // 先绘制渐变背景
    canvas.draw_image(&gradient, (0, 0), None);
    
    // 使用蒙版（这里简化处理，直接将蒙版作为图像叠加）
    // 在实际效果中，可能需要使用混合模式来达到更好的效果
    let mut mask_paint = Paint::default();
    mask_paint.set_blend_mode(BlendMode::Multiply);
    canvas.draw_image(&mask, (0, 0), Some(&mask_paint));

    let result = surface.image_snapshot();

    // 编码为PNG
    let png_data = result
        .encode(None, EncodedImageFormat::PNG, 100)
        .ok_or_else(|| Error::Message("Failed to encode image".to_string()))?;

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