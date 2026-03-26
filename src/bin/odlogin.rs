use dotenvy::dotenv;
use nyamedia_bot::onedrive::service::OnedriveService;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let service = match OnedriveService::from_env() {
        Ok(service) => service,
        Err(err) => {
            eprintln!("初始化 OneDrive 登录器失败: {:?}", err);
            std::process::exit(1);
        }
    };

    let session = service.session_snapshot().await;

    if let Some(authorization_code) = session.authorization_code.as_deref() {
        if let Err(err) = service.exchange_authorization_code_input(authorization_code).await {
            eprintln!("完成 OneDrive 登录失败: {:?}", err);
            std::process::exit(1);
        }
    } else {
        let (auth_url, state) = match service.prepare_authorization().await {
            Ok(result) => result,
            Err(err) => {
                eprintln!("准备 OneDrive 授权失败: {:?}", err);
                std::process::exit(1);
            }
        };

        println!("请在浏览器中打开以下链接完成 Microsoft 登录：");
        println!();
        println!("{}", auth_url);
        println!();
        println!("登录完成后，请把回调 URL 里 code= 后面的内容写入以下文件：");
        println!("{}", service.session_file_path().display());
        println!();
        println!("写入字段：authorization_code");
        println!("本次 OAuth state: {}", state);
        println!("写入完成后，请重新运行 odlogin。");
        return;
    }

    match service.status().await {
        Ok(status) => {
            println!("OneDrive 登录成功。");
            println!("session 文件: {}", service.session_file_path().display());
            println!("connected: {}", status.connected);
            if let Some(owner) = status.owner {
                println!("owner: {}", owner);
            }
            if let Some(drive_type) = status.drive_type {
                println!("drive_type: {}", drive_type);
            }
        }
        Err(err) => {
            println!("OneDrive token 已保存，但读取状态失败: {:?}", err);
            println!("session 文件: {}", service.session_file_path().display());
        }
    }

}
