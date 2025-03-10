use ::std::{env, fs, path::Path};

use ::resvg::{
    tiny_skia::Pixmap,
    usvg::{Options, Transform, Tree},
};

fn main() {
    let config = ::slint_build::CompilerConfiguration::new().with_style("fluent-dark".into());
    let ui_path = "ui/appwindow.slint";
    slint_build::compile_with_config(ui_path, config).unwrap();
    println!("cargo::rerun-if-changed={ui_path}");

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let png_path = out_dir.join("icon.png");
    {
        let path = "resources/icon.svg";
        println!("cargo::rerun-if-changed={path}");

        let svg = fs::read(path).unwrap();
        let svg = Tree::from_data(&svg, &Options::default()).unwrap();

        let mut pixmap = Pixmap::new(256, 256).unwrap();

        ::resvg::render(&svg, Transform::identity(), &mut pixmap.as_mut());

        pixmap.save_png(&png_path).unwrap();
    };

    if cfg!(target_os = "windows") {
        let icon = ::image::open(&png_path).unwrap();
        let icon_name = "icon.ico";

        let icon_path = out_dir.join(icon_name);
        let rc_path = out_dir.join("resources.rc");

        icon.save(&icon_path).unwrap();
        fs::write(&rc_path, format!("AppIcon ICON \"{icon_name}\"\n")).unwrap();

        ::embed_resource::compile(&rc_path, ::embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
