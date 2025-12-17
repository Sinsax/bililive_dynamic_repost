use std::{thread::sleep,path::Path,fs};
use bpi_rs::{ BpiClient, auth::Account};
use tracing_subscriber::{fmt, EnvFilter};
use bililive_dynamic_repost::{config::{Config,Livestatus,Poststatus},work::Repost};


#[tokio::main]
async fn main() {
    // 日志
    let dir = "logs";
    if !Path::new(dir).exists() {
        fs::create_dir_all(dir).expect("无法创建日志目录");
    } 

    let log_path = format!("log_{}.log", chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S"));
    let log_path = Path::new(dir).join(log_path);
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .expect("无法创建或打开日志文件");
    // let combined_writer = file_writer.and(io::stdout());
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(file)
        .compact()
        .init();
    

    // 初始化
    // 生成随机数

    let real = BpiClient::new();
    let fake = BpiClient::new();
    let config =Config::new("config.toml");
    let config_fake =Config::new("config_fake.toml");

    let account_real = Account{
        dede_user_id: config.dede_user_id,
        dede_user_id_ckmd5: config.dede_user_id_ckmd5,
        sessdata: config.sessdata,
        bili_jct: config.bili_jct,
        buvid3: config.buvid3,
    };
    // test empty account
    let account_fake = Account{
        dede_user_id: config_fake.dede_user_id,
        dede_user_id_ckmd5: config_fake.dede_user_id_ckmd5,
        sessdata: config_fake.sessdata,
        bili_jct: config_fake.bili_jct,
        buvid3: config_fake.buvid3,
    };
    

    // 登录
    fake.set_account(account_fake.clone());
    // match bpi.logout_web(Some("javascript:history.go(-1)")).await {
    //     Ok(_) => tracing::info!("登出成功"),
    //     Err(e) => tracing::error!("登出失败: {:#?}", e),
    // };
    let mut _staus = Poststatus::Wait;
    let mut livestatus = Livestatus{
        posted:false,
        lived:false,
    };
    let mut src_dyn: String;
    let mut repost_dyn= String::new();
    let mut _delete = false;
    let mut fail = 0;

    // 检查cookie有效性
    if fake.check_cookie().await==false{
        panic!("Cookie无效,程序终止");
    }
    // 用户空间id
    let taget_user=fake.user_card_info(config.mid, Some(false)).await;
    match taget_user {
        Ok(user_info) => {
            if let Some(data) = user_info.data {
                tracing::info!("获取用户信息成功: 用户名: {}", data.card.name);
            }
        }
        Err(e) => {
            tracing::error!("获取用户信息失败: {:#?}", e);
            panic!("请重新检查id是否正确");
        }
    }
    tracing::info!("初始化完成");
    // 主循环
    loop {
        if fail >= 5{
            panic!("连续失败5次,程序终止");
        }
        
        src_dyn = fake.dyn_getid(&config_fake.mid.to_string()).await;
        livestatus.lived = match src_dyn {
            ref s if s.is_empty() => false,
            _ => true,
        };
        // livestatus.lived =true; // 测试用,强制为true
        // 检查状态
        _staus = livestatus.check();

        // 执行操作
        match _staus {
            Poststatus::Wait=>{
                let rnd_num = rand::random_range(60..120);
                // panic!("{}",rnd_num);
                let dur = std::time::Duration::from_secs(rnd_num);
                tracing::info!("WAIT:等待{}秒后继续检测",dur.as_secs());
                sleep(dur);
            }

            Poststatus::Post=>{
                real.set_account(account_real.clone());
                if real.check_cookie().await==false{
                    panic!("Cookie无效,程序终止");
                }

                tracing::info!("POST:开始转发动态:https://t.bilibili.com/{}",src_dyn);
                
                match real.dyn_repost(&src_dyn,&config_fake.repost_text).await {
                    Ok(dyn_id) => {
                        repost_dyn = dyn_id;
                        livestatus.posted = true;
                    }
                    Err(_e) => {
                        livestatus.posted = false;
                        fail += 1;
                    }
                };
            }
            Poststatus::Delete=>{
                real.set_account(account_real.clone());
                if real.check_cookie().await==false{
                    panic!("Cookie无效,程序终止");
                }
                tracing::info!("DELETE:开始删除动态:{}",repost_dyn);
                if repost_dyn.is_empty(){
                    tracing::info!("动态ID为空,跳过删除");
                }else {
                    _delete = real.dyn_delete(&repost_dyn).await;
                }
                // if delete{
                    livestatus.posted=false;
                // }
            }
        }

        fake.set_account(account_fake.clone());
    }

}



