use ::std::{env, fs, path::Path};

use ::image::ImageFormat::Png;
use ::resvg::{
    tiny_skia::Pixmap,
    usvg::{Options, Transform, Tree},
};

fn main() {
    let config = ::slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    let ui_path = "ui/appwindow.slint";
    slint_build::compile_with_config(ui_path, config).unwrap();
    println!("cargo::rerun-if-changed={ui_path}");

    if cfg!(target_os = "windows") {
        let png_icon = {
            let path = "resources/icon.svg";
            println!("cargo::rerun-if-changed={path}");

            let svg = fs::read(path).unwrap();
            let svg = Tree::from_data(&svg, &Options::default()).unwrap();

            let mut pixmap = Pixmap::new(256, 256).unwrap();

            ::resvg::render(&svg, Transform::identity(), &mut pixmap.as_mut());

            pixmap.encode_png().unwrap()
        };

        let icon = ::image::load_from_memory_with_format(&png_icon, Png).unwrap();
        drop(png_icon);

        let out_dir = env::var_os("OUT_DIR").unwrap();
        let out_dir = Path::new(&out_dir);

        let icon_path = out_dir.join("icon.ico");
        let rc_path = out_dir.join("resources.rc");

        fs::write(&rc_path, "APPICON ICON icon.ico").unwrap();
        icon.save(&icon_path).unwrap();

        ::embed_resource::compile(&rc_path, ::embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
