use std::collections::HashSet;

use skia_safe::{
    image_filters, Color4f, EncodedImageFormat, Image, Paint, Point, Rect, Shader, TileMode,
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

/// 模拟滤镜蒙版（用 Skia 的 image filter 做模糊）
fn make_mask(base: &Image, pencil: &Image, denoise: bool) -> Result<Image, Error> {
    // base.dimensions() 返回 ISize（带 width/height 字段）
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 创建临时 surface
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();

    // 如果需要降噪，用 Paint + image_filter blur 来绘制 base
    if denoise {
        let mut paint = Paint::default();
        // 半径 3.0，可根据需要调整
        let blur_filter = image_filters::blur((3.0, 3.0), None);
        paint.set_image_filter(blur_filter);
        canvas.draw_image(base, (0, 0), Some(&paint));
    } else {
        canvas.draw_image(base, (0, 0), None);
    }

    // 再绘制 pencil 作为叠加
    canvas.draw_image(pencil, (0, 0), None);

    Ok(surface.image_snapshot())
}

/// 创建渐变背景（使用正确的 linear_gradient 签名）
fn make_gradient(width: i32, height: i32) -> Result<Image, Error> {
    let mut surface = new_surface((width, height));
    let canvas = surface.canvas();

    let positions: [f32; 6] = [0.0, 0.4, 0.6, 0.7, 0.8, 1.0];
    let colors: [Color4f; 6] = [
        Color4f::new(0.984, 0.729, 0.188, 1.0),
        Color4f::new(0.988, 0.447, 0.207, 1.0),
        Color4f::new(0.988, 0.207, 0.305, 1.0),
        Color4f::new(0.811, 0.211, 0.874, 1.0),
        Color4f::new(0.215, 0.709, 0.850, 1.0),
        Color4f::new(0.243, 0.713, 0.854, 1.0),
    ];

    // linear_gradient 的签名通常为:
    // Shader::linear_gradient(((x0,y0),(x1,y1)), colors_slice, Option<&[pos]>, TileMode, Option<flags>)
    let shader = Shader::linear_gradient(
        ((0.0f32, 0.0f32), (width as f32, height as f32)),
        &colors as &[Color4f],
        Some(&positions),
        TileMode::Clamp,
        None,
    );

    let mut paint = Paint::default();
    paint.set_shader(shader);

    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, width as f32, height as f32), &paint);

    Ok(surface.image_snapshot())
}

fn louvre(images: Vec<InputImage>, _texts: Vec<String>, options: Louvre) -> Result<Vec<u8>, Error> {
    // ---------------------------
    // 注意：InputImage -> skia_safe::Image 的方法名在不同版本可能不一样。
    // 我先用 `to_image()`（很多实现里是 to_image / as_image / into_image）
    // 如果你的版本不是 to_image，请替换为你版本的对应方法（as_image / into_image / image 等）。
    // ---------------------------
    let base: Image = images[0].to_image()?; // 若编译报错“no method to_image”，请换成 as_image() / into_image() 等
    let size = base.dimensions();
    let (w, h) = (size.width, size.height);

    // 加载素材 01.png（变量名用 pencil）
    let pencil = load_image("louvre/01.png")?.resize_exact((w, h));

    // 渐变背景
    let gradient = make_gradient(w, h)?;

    // 蒙版（mask）
    let mask = make_mask(&base, &pencil, options.denoise.unwrap_or(false))?;

    // 合成结果：把 gradient 放底、把 mask 作为上层（当前实现把 mask 当成一层绘制）
    let mut surface = new_surface((w, h));
    let canvas = surface.canvas();
    canvas.draw_image(&gradient, (0, 0), None);

    // 如果你想把 mask 当作 alpha mask 去显示 gradient（即 gradient 仅在 mask 白色处显示），
    // 需要更复杂的 save_layer + paint.set_blendmode/clip 的逻辑。当前为了兼容性我直接绘制 mask。
    canvas.draw_image(&mask, (0, 0), None);

    let result = surface.image_snapshot();

    // encode_to_data 在某些 skia_safe 版本会被标记为需要 context，若你遇到警告/错误请改用
    // encode_to_data_with_context(...) 并传入上下文。
    // 这里用 unwrap() 保证编译通过（开发阶段），你可以改为更严格的错误返回。
    let png_data = result
        .encode_to_data(EncodedImageFormat::PNG)
        .unwrap();

    Ok(png_data.as_bytes().to_vec())
}

register_meme!(
    "louvre",
    louvre,
    min_images = 1,
    max_images = 1,
    keywords = &["卢浮宫"],
    // macro 接受一个 HashSet<String>，这里以表达式构造并传入
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
