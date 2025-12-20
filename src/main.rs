use clap::{Parser};
use tokio::fs;
use std::{io::{self, Write}};
use bpi_rs::{ BpiClient, auth::Account};
use bililive_dynamic_repost::{config::{Config},work::Repost};
use tracing_subscriber::{EnvFilter,fmt};
use std::error::Error;

#[derive(Debug,Parser)]
#[command(version, about, long_about = None)]
struct Cli{
    /// 执行转发操作
    #[arg(short = 'r',long = "repost", action = clap::ArgAction::SetTrue)]
    repost: bool,

    /// 执行删除操作
    #[arg(short = 'd',long = "delete", action = clap::ArgAction::SetTrue)]
    delete: bool,
}
enum Status {
    Post,
    DELETE,
}
fn confirm() -> bool {
    loop {
        // 打印提示并刷新输出（避免缓冲）
        print!("(Y/N): ");
        io::stdout().flush().unwrap();

        // 读取用户输入
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        // 处理输入（忽略空格/换行，不区分大小写）
        let input = input.trim().to_lowercase();
        match input.as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("无效输入！请输入 Y 或 N"),
        }
    }
}
#[tokio::main]
async fn main()->Result<(),Box<dyn Error>>{
    // 日志
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_ansi(std::env::var("NO_COLOR").is_err())
        .init();

    let cli = Cli::parse();
    let op_config = Config::new("config.toml");
    let mut config = op_config.ok_or("配置未找到")?;
    
    let op_status =  {
        match (cli.repost, cli.delete) {
            (true, false) => {
                if !&config.repost_dynid.is_empty(){
                    Err("toml文件中repost_dynid不为空,请清除后再试")
                }else {
                    println!("将执行转发操作！");
                    Ok(Status::Post)
                }
            }
            (false, true) => {
                if (&config.repost_dynid).is_empty(){
                    Err("toml文件中repost_dynid为空,请执行过转发或手动配置后再试")
                }else {
                    println!("将执行删除操作！");
                    Ok(Status::DELETE)
                }  
            }
            _ =>{
                Err("未指定有效操作（请用 -r/--repost 或 -d/--delete")
            }
        }
    };

    let status = match op_status {
        Ok(status) => status,
        Err(e) => return Err(e.into()),
    };

    let bpi = BpiClient::new();

    let account = Account{
        dede_user_id: config.dede_user_id.clone(),
        dede_user_id_ckmd5: config.dede_user_id_ckmd5.clone(),
        sessdata: config.sessdata.clone(),
        bili_jct: config.bili_jct.clone(),
        buvid3: config.buvid3.clone(),
    };
    // 登录
    bpi.set_account(account);

    // 获取用户信息
    let room_info = bpi.live_info(config.roomid).await?;

    match bpi.user_card_info(room_info.uid, Some(false)).await {
        Ok(some_userinfo)=>{
            if let Some(user_info) = some_userinfo.data{
                tracing::info!("获取用户信息成功: 用户名: {}", user_info.card.name);
            }
        }
        Err(e)=>{
            tracing::error!("获取用户信息失败: {:#?}", e);
        }
    }

    match status {
        Status::Post=>{
            if room_info.live_status !=1{
                return Err("POST:用户没有开播!".into());
            }
            let src_dyn = bpi.dyn_getid(&room_info.uid.to_string()).await;
            if src_dyn.is_empty() {
                // 直播和动态时间差可能有10分钟左右
                // tracing::error!("POST:请等待动态生成");
                return Err("POST:请等待动态生成".into());
            }

            tracing::info!("POST:开始转发动态:https://t.bilibili.com/{}",src_dyn);
            match bpi.dyn_repost(&src_dyn,&config.repost_text).await {
                Ok(repost_dynid) => {
                    tracing::info!("POST:动态转发成功");
                    config.repost_dynid =repost_dynid;

                    if let Ok(toml_str) = toml::to_string(&config){
                        match fs::write("config.toml", toml_str).await {
                            Ok(_)=>{
                                tracing::info!("POST:已保存动态id到toml中,后续可以使用--delete对动态进行删除");
                            }
                            Err(e)=>{
                                tracing::error!("POST:保存转发动态id失败,错误: {:#?}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("POST:动态转发失败,错误:{}",e).into());
                }
            }
        }

        Status::DELETE=>{
            if room_info.live_status ==1{
                tracing::info!("DELETE:用户还没有下播,是否执行删除操作");
                if !confirm(){
                    return Err(format!("DELETE:中止删除动态:{}",config.repost_dynid).into());
                }
            }

            tracing::info!("DELETE:开始删除动态:{}",config.repost_dynid);
            if bpi.dyn_delete(&config.repost_dynid).await{
                tracing::info!("DELETE:动态删除成功");
            }else {
                tracing::error!("DELETE:动态删除失败");
            }
            config.repost_dynid =String::new();
            if let Ok(toml_str) = toml::to_string(&config){
                match fs::write("config.toml", toml_str).await {
                    Ok(_)=>{
                        tracing::info!("DELETE:已清除toml中的动态id");
                    }
                    Err(e)=>{
                        tracing::error!("DELETE:清除toml中的动态id失败,错误: {:#?}", e);
                    }
                }
            }
        }
    }
    // }
    Ok(())
}