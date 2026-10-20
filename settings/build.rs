// SPDX-License-Identifier: GPL-3.0-or-later

use std::{env, fs, path::Path};
use xdgen::{App, Context, FluentString};

const APP_ID: &str = "io.github.bbbenji.ActivateLinux";

fn main() {
    let ctx = Context::new("i18n", env::var("CARGO_PKG_NAME").unwrap()).unwrap();
    let app = App::new(FluentString("app-title"))
        .comment(FluentString("app-comment"))
        .keywords(FluentString("app-keywords"));

    let resources = Path::new("../resources");
    let desktop = app
        .expand_desktop(resources.join(format!("{APP_ID}.desktop")), &ctx)
        .unwrap();
    let metainfo = app
        .expand_metainfo(resources.join(format!("{APP_ID}.metainfo.xml")), &ctx)
        .unwrap();

    // Same location the template uses, relative to the workspace root.
    let output = Path::new("../target/xdgen");
    fs::create_dir_all(output).unwrap();
    fs::write(output.join(format!("{APP_ID}.desktop")), desktop).unwrap();
    fs::write(output.join(format!("{APP_ID}.metainfo.xml")), metainfo).unwrap();

    println!("cargo::rerun-if-changed=../resources");
    println!("cargo::rerun-if-changed=i18n");
}
