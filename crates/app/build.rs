//! # Windows 资源编译脚本
//!
//! 仅在 Windows 目标上把 `resources/app.rc`（图标 + 版本信息）
//! 嵌入到可执行文件；其他平台（macOS 开发环境）跳过。

fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("resources/app.rc", embed_resource::NONE)
            .expect("编译 Windows 资源失败");
    }
    #[cfg(not(windows))]
    {
        // macOS / Linux 开发环境无需嵌入 Windows 资源
    }
}
