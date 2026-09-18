fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Windows 给主线程的默认栈只有 1MB，而 gpui 的渲染是逐层递归的：
    // debug 构建下光进入页面 render 就要用掉 0.8MB 左右，界面再复杂一点
    // 就会直接“thread 'main' has overflowed its stack”。这里把主线程栈加大到 8MB。
    println!("cargo:rustc-link-arg-bins=/STACK:8388608");

    winres::WindowsResource::new()
        .set_icon("icon.ico")
        .compile()?;
    Ok(())
}
