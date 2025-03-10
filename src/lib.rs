use ::color_eyre::Result;
use ::slint::{Image, Rgba8Pixel, SharedPixelBuffer};

pub mod ui {
    slint::include_modules!();
}

pub mod cli;

pub mod state;

pub fn icon() -> Result<Image> {
    const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));
    let icon = ::image::load_from_memory_with_format(ICON, ::image::ImageFormat::Png)?.into_rgba8();
    let icon_buf = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        icon.as_raw(),
        icon.width(),
        icon.height(),
    );
    Ok(Image::from_rgba8(icon_buf))
}
