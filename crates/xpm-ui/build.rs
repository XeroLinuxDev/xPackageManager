fn main() {
    println!("cargo:rerun-if-changed=translations/");
    slint_build::compile_with_config(
        "ui/main.slint",
        slint_build::CompilerConfiguration::new()
            .with_style("native".into())
            .with_bundled_translations("translations/")
            .with_default_translation_context(
                slint_build::DefaultTranslationContext::None,
            ),
    )
    .unwrap();
}
