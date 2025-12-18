use std::{any::Any, f32::consts::E, fs, path::Path, thread::sleep};
use bpi_rs::{ BpiClient, auth::Account, fav, login::member_center::account};
use tracing_subscriber::{fmt, EnvFilter};
use bililive_dynamic_repost::{config::{Config,Livestatus,Poststatus},work::{Repost, RoomInfo}};

#[derive(PartialEq)]
enum LoginType{
    Guest,
    Logged
}
struct LoginSet{
    config_guest:Account,
    config_login:Account,
    login_type:LoginType
}
impl LoginSet {
    fn change(&mut self,logintype:LoginType)->Option<Account>{
        if self.login_type != logintype{
            self.login_type = logintype;

            match self.login_type {
                LoginType::Guest =>{
                    Some(self.config_guest.clone())
                }
                LoginType::Logged =>{
                    Some(self.config_login.clone())
                }
            }
        }else {
            None
        }
    }
}

#[tokio::main]
async fn main() {
    // // 日志
    // let dir = "logs";
    // if !Path::new(dir).exists() {
    //     fs::create_dir_all(dir).expect("无法创建日志目录");
    // } 
    // let log_path = format!("log_{}.log", chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S"));
    // let log_path = Path::new(dir).join(log_path);
    // let file = fs::OpenOptions::new()
    //     .create(true)
    //     .append(true)
    //     .open(log_path)
    //     .expect("无法创建或打开日志文件");
    // // let combined_writer = file_writer.and(io::stdout());
    // let filter = EnvFilter::try_from_default_env()
    //     .unwrap_or_else(|_| EnvFilter::new("info"));
    // fmt()
    //     .with_env_filter(filter)
    //     .with_ansi(false)
    //     .with_writer(file)
    //     .compact()
    //     .init();
    
    // 初始化
    let bpi = BpiClient::new();
    let config =Config::new("config.toml");
    let account = Account{
        dede_user_id: config.dede_user_id,
        dede_user_id_ckmd5: config.dede_user_id_ckmd5,
        sessdata: config.sessdata,
        bili_jct: config.bili_jct,
        buvid3: config.buvid3,
    };
    // empty account
    let account_empty = Account{
        dede_user_id: "".to_string(),
        dede_user_id_ckmd5: "".to_string(),
        sessdata: "".to_string(),
        bili_jct: "".to_string(),
        buvid3: "".to_string(),
    };
    let mut _staus = Poststatus::Wait4live;
    let mut livestatus = Livestatus{
        posted:false,
        lived:false,
        dynamic:false,
    };
    let mut src_dyn= String::new();
    let mut repost_dyn= String::new();
    let mut delete = false;
    let mut fail = 0;
    let mut room_info:RoomInfo;
    let rnd_num = rand::random_range(60..120);
    
    // 游客登录
    bpi.set_account(account_empty.clone());
    // 登录器设置
    let mut login_set:LoginSet = LoginSet{
        config_guest:account_empty,
        config_login:account,
        login_type:LoginType::Guest,
    };

    // 获取roominfo
    room_info = match bpi.live_info(config.roomid).await{
        Ok(room_info)=>{
            room_info
        },
        Err(e)=>{
            panic!("请重新检查直播间id,错误:{}",e);
        }
    };

    // 打印用户名称，防止弄错了
    match bpi.user_card_info(room_info.uid, Some(false)).await{
        Ok(user_info) => {
            if let Some(data) = user_info.data {
                tracing::info!("获取用户信息成功: 用户名: {}", data.card.name);
            }
        }
        Err(e) => {
            tracing::error!("获取用户信息失败: {:#?}", e);
        }
    }

    tracing::info!("初始化完成");
    // 主循环
    loop {
        if fail >= 5{
            panic!("连续失败5次,程序终止");
        }

        // 检查状态
        _staus = livestatus.check();

        // 执行操作
        match _staus {
            Poststatus::Wait4live =>{
                if let Some(loginset) = login_set.change(LoginType::Guest) {
                    bpi.set_account(loginset);
                }   

                // 获取直播信息
                match bpi.live_info(config.roomid).await {
                    Ok(roominfo)=>{
                        room_info = roominfo
                    }
                    Err(e)=>{
                        fail +=1;
                        tracing::error!("请重新检查直播间id,错误:{}",e);
                    }
                } 

                // 确定直播状态
                if room_info.live_status == 1 {
                    livestatus.lived =true;
                    continue;
                }else {
                    livestatus.lived =false;
                }

                tracing::info!("WAIT:等待{}秒后继续检测直播状态",rnd_num);
            }
            Poststatus::Wait4dynamic=>{
                if let Some(loginset) = login_set.change(LoginType::Logged) {
                    bpi.set_account(loginset);
                }

                // 获取直播动态id
                src_dyn = bpi.dyn_getid(&room_info.uid.to_string()).await;

                if src_dyn.is_empty() {
                    // 直播和动态时间差可能有10分钟左右
                    tracing::info!("POST:等待{}秒后继续检测动态生成",rnd_num);

                }else {
                    // 检测到动态直接跳过睡眠进行post
                    livestatus.dynamic=true;
                    continue;
                }
            }
            Poststatus::Post=>{
                if let Some(loginset) = login_set.change(LoginType::Logged) {
                    bpi.set_account(loginset);
                }

                tracing::info!("POST:开始转发动态:https://t.bilibili.com/{}",src_dyn);
                match bpi.dyn_repost(&src_dyn,&config.repost_text).await {
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
                if let Some(loginset) = login_set.change(LoginType::Logged) {
                    bpi.set_account(loginset);
                }

                tracing::info!("DELETE:开始删除动态:{}",repost_dyn);
                if repost_dyn.is_empty(){
                    tracing::info!("动态ID为空,跳过删除");
                }else {
                    delete = bpi.dyn_delete(&repost_dyn).await;
                }

                if delete{
                    livestatus.posted=false;
                }else {
                    fail += 1;
                    tracing::error!("删除失败");
                }
            }
        }
        
        // sleep
        let dur = std::time::Duration::from_secs(rnd_num);
        sleep(dur);

    }
}
