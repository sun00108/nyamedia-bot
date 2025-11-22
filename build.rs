use std::process::Command;
use std::path::Path;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=web/");
    
    // 如果设置了跳过前端构建的环境变量，直接返回
    if std::env::var("SKIP_FRONTEND_BUILD").is_ok() {
        println!("cargo:warning=SKIP_FRONTEND_BUILD is set, skipping frontend build");
        return;
    }
    
    // 检查 web 目录是否存在
    if !Path::new("web").exists() {
        println!("cargo:warning=web directory not found, skipping frontend build");
        return;
    }
    
    // 检查 package.json 是否存在
    if !Path::new("web/package.json").exists() {
        println!("cargo:warning=web/package.json not found, skipping frontend build");
        return;
    }
    
    // 检查是否已安装 Node.js
    let node_check = Command::new("node")
        .args(&["--version"])
        .output();
    
    if node_check.is_err() {
        println!("cargo:warning=Node.js not found, skipping frontend build. Please install Node.js to build the web frontend.");
        return;
    }
    
    // 安装依赖
    println!("cargo:warning=Installing frontend dependencies...");
    let npm_install = Command::new("npm")
        .args(&["install"])
        .current_dir("web")
        .output()
        .expect("Failed to run npm install");
    
    if !npm_install.status.success() {
        panic!("npm install failed: {}", String::from_utf8_lossy(&npm_install.stderr));
    }
    
    // 构建前端
    println!("cargo:warning=Building frontend...");
    let mut npm_build = Command::new("npm");
    npm_build
        .args(&["run", "build"])
        .current_dir("web");

    // 传递 VITE_ 开头的环境变量到前端构建过程
    for (key, value) in env::vars() {
        if key.starts_with("VITE_") {
            npm_build.env(key, value);
        }
    }

    let npm_build = npm_build
        .output()
        .expect("Failed to run npm build");
    
    if !npm_build.status.success() {
        panic!("Frontend build failed: {}", String::from_utf8_lossy(&npm_build.stderr));
    }
    
    println!("cargo:warning=Frontend build completed successfully");
}