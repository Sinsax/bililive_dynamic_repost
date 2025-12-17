pub mod config {
use std::{path::Path,fs};
use serde::Deserialize;

    // config.toml
    #[derive(Deserialize)]
    pub struct Config{
        pub mid:u64,
        pub repost_text:String,
        pub dede_user_id: String,
        pub dede_user_id_ckmd5: String,
        pub sessdata: String,
        pub bili_jct: String,
        pub buvid3: String,
    }
    impl Config {
        pub fn new(config_name: &str) -> Self {
            let default_toml = 
r#"# 配置文件
# 请填写需要监控的用户UID,不是直播间ID
mid = 327311724
# 转发动态所需文本
repost_text = "转发动态"
# Cookie
bili_jct = "your_bili_jct_here"
dede_user_id = "your_dede_user_id_here"
dede_user_id_ckmd5 = "your_dede_user_id_ckmd5_here"  
sessdata = "your_sessdata_here"
buvid3 = "your_buvid3_here"
"#;

            // 获取config.toml内容到结构体
            let toml_path = Path::new(config_name);
            let toml_content = match fs::read_to_string(toml_path) {
                Ok(content) => content,
                Err(e) => {
                    tracing::error!("读取{}失败: {:#?}", config_name, e);
                    // 重新生成config.toml
                    fs::write(toml_path, default_toml).expect(format!("无法创建{}", config_name).as_str());
                    panic!("已重新生成{}于当前目录下", config_name);
                }
            };

            // 2. 反序列化为结构体（核心步骤）
            let config: Config = match toml::from_str(&toml_content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::error!("解析{}失败: {:#?}", config_name, e);
                    // 删除错误的config.toml
                    fs::remove_file(toml_path).expect("无法删除错误的config.toml");
                    // 重新生成config.toml
                    fs::write(toml_path, default_toml).expect(format!("无法创建{}", config_name).as_str());
                    panic!("已重新生成{}于当前目录下", config_name);
                }
            };

            config
        }
    }

    pub enum Poststatus {
        Post,
        Wait,
        Delete
    }

    pub struct Livestatus {
        pub posted:bool,
        pub lived:bool,
    }

    impl Livestatus {
        // live post
        // true true
        //     sleep
        // true false
        //     post
        // false true
        //     del
        // false false
        //     sleep
        // 确定运行状态
        pub fn check(&self) ->Poststatus{
            if self.lived == self.posted{
                return Poststatus::Wait;
            }
            if self.lived && !self.posted{
                return Poststatus::Post;
            }
            if !self.lived && self.posted{
                return Poststatus::Delete;
            }
            Poststatus::Wait

            // match self.lived {
            //     true => {
            //         if self.posted{
            //             Poststatus::Wait
            //         }else {
            //             Poststatus::Post
            //         }
            //     }
            //     false => {
            //         if self.posted{
            //             Poststatus::Delete
            //         }else {
            //             Poststatus::Wait
            //         }
            //     }
            // }

            // if self.lived{
            //     if self.posted{
            //         Poststatus::Wait
            //     }else {
            //         Poststatus::Post
            //     }
            // }else {
            //     if self.posted{
            //         Poststatus::Delete
            //     }else {
            //         Poststatus::Wait
            //     }
            // }
        }
    }

}
pub mod work{
use std::vec;
use bpi_rs::{BilibiliRequest, BpiClient, BpiError, BpiResponse,
        dynamic::publish::{
            CreateComplexDynamicData, DynamicContent, DynamicContentItem, DynamicPic, DynamicRequest, DynamicTopic
        }, };
use serde_json::{Value, from_slice,json};


pub trait Repost {
    #[allow(async_fn_in_trait)]
    async fn dynamic_repost(
        &self,
        scene: u8,
        contents: Vec<DynamicContentItem>,
        pics: Option<Vec<DynamicPic>>,
        topic: Option<DynamicTopic>,
        src_dyn: &str,
    ) -> Result<BpiResponse<CreateComplexDynamicData>, BpiError>;
    #[allow(async_fn_in_trait)]
    async fn dyn_delete(&self,repost_dyn:&str)->bool;
    #[allow(async_fn_in_trait)]
    async fn dyn_getid(&self,mid:&str)->String;
    #[allow(async_fn_in_trait)]
    async fn dyn_repost(&self,src_dyn:&str,repost_text:&str)->Result<String,BpiError>;
    #[allow(async_fn_in_trait)]
    async fn check_cookie(&self)->bool;

}
impl Repost for BpiClient {
    async fn dynamic_repost(
        &self,
        scene: u8,
        contents: Vec<DynamicContentItem>,
        pics: Option<Vec<DynamicPic>>,
        topic: Option<DynamicTopic>,
        src_dyn: &str,
    ) -> Result<BpiResponse<CreateComplexDynamicData>, BpiError> {
        let csrf: String = self.csrf()?;

        let dyn_req = DynamicRequest {
            attach_card: None,
            content: DynamicContent { contents },
            meta: Some(
                json!({
                "app_meta": {
                    "from": "create.dynamic.web",
                    "mobi_app": "web"
                }
            })
            ),
            scene,
            pics,
            topic,
            option: None,
        };

        let request_body = json!({
            "dyn_req": dyn_req,
            "web_repost_src": {
                "dyn_id_str": src_dyn
            }
        });

        self
            .post("https://api.bilibili.com/x/dynamic/feed/create/dyn")
            .header("Content-Type", "application/json")
            .query(&[("csrf", csrf)])
            .body(request_body.to_string())
            .send_bpi("发表复杂动态").await
    }
    async fn dyn_delete(&self,repost_dyn:&str)->bool{
        // 删除动态
        let result = self.dynamic_delete(&repost_dyn).await;
        match result {
            Ok(resp) => {
                if let Some(data) = resp.data {
                    tracing::info!("删除成功:{:?}", data);
                }
                true
            }
            Err(e) => { 
                tracing::error!("删除失败:{:#?}", e);
                false
            }
        }
    }
    async fn dyn_getid(&self,mid:&str)->String{
        // 获取动态
        let mut url = String::from("https://api.bilibili.com/x/polymer/web-dynamic/v1/feed/space?offset=&host_mid=");
        // tracing::info!("{}",url);
        url.insert_str(url.len(),mid);

        let result = self.get(&url).send_request("").await;
        tracing::info!("获取动态列表成功");
        let result = match result {
            Ok(data) => {
                data
            }
            Err(e) => { 
                tracing::error!("{:#?}", e);
                return String::new();
            }
        };
        let dyn_list:Value = from_slice(&result).unwrap();
        let dyn_list: Vec<Value> = dyn_list["data"]["items"].as_array().unwrap().to_vec();

        let dyn_id: String = dyn_list
            .iter()
            .filter(|dynamic| {
                dynamic.get("type")
                    .and_then(Value::as_str)
                    == Some("DYNAMIC_TYPE_LIVE_RCMD")
            })
            .filter_map(|dynamic| {
                // 过滤掉 id_str 不存在/非字符串的元素
                dynamic.get("id_str")
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .collect();
        dyn_id
    }
    async fn dyn_repost(&self,src_dyn:&str,repost_text:&str)->Result<String,BpiError>{
        // 转发动态
        let contents:Vec<DynamicContentItem> = vec![
            DynamicContentItem{
                type_num:0,
                biz_id:Some("".to_string()),
                raw_text:repost_text.to_string(),
            }
        ];
        let pics: Option<Vec<DynamicPic>> = None;
        let topic: Option<DynamicTopic> = None;

        let result = self.dynamic_repost(4, contents, pics, topic,src_dyn).await;

        let repost_dyn = match result {
            Ok(resp) => {
                if let Some(data) = resp.data {
                    tracing::info!("转发成功,https://t.bilibili.com/{:?}", data.dyn_id_str);
                    Ok(data.dyn_id_str)
                }else {
                    tracing::info!("转发成功,但是获取转发后的id失败,可能需要手动管理动态");
                    Ok(String::new())
                }
            }
            Err(e) => { 
                tracing::error!("转发失败:{:#?}", e);
                Err(e)
            }
        };
        repost_dyn
    }
    async fn check_cookie(&self)->bool{
        // 验证cookie有效性
        match self.member_center_account_info().await {
            Ok(info) => {
                if let Some(data) = info.data {
                    tracing::info!("验证成功,当前用户: {}", data.uname);
                }else {
                    tracing::info!("验证成功但是获取用户信息失败");
                }
                true
            }
            Err(e) => {
                tracing::error!("验证失败,请检查cookie是否有效: {:#?}", e);
                false
            }
        }
    }

}
    
}