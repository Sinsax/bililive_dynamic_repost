# bililive_dynamic_repos

## 📖 介绍

bililive_dynamic_repos 动态转发是一个用 Rust 编写的自动化工具，用于实时监听 B 站直播动态并自动转发到动态。

## 🚀 使用方式

### 前置要求
- Rust 1.90+ (安装：https://www.rust-lang.org/zh-CN/tools/install)
- 有效的 B 站账号

### 安装

```bash
git clone https://github.com/your-repo/bililive_dynamic_repost.git
cd bililive_dynamic_repost
```

### 配置

1. 编辑 `config.toml`
填入所需的cookie和需要转发直播用户的uid

### 构建

```bash
cargo build --release
```

### 运行

```bash
cargo run --release
# 或直接运行编译后的二进制文件
./target/release/bililive_dynamic_repost
```

## ⚠️ 注意事项

- ⛔ **禁止在公共仓库提交 `config.toml`**，添加到 `.gitignore`
- 📊 不要频繁请求同一 UID 的动态（建议间隔 > 30 秒）

## 📦 项目结构

```
bililive_dynamic_repost/
├── src/
│   ├── main.rs          # 主程序入口
│   ├── bili_client.rs   # B站API客户端
│   ├── forwarder.rs     # 转发模块
│   └── config.rs        # 配置管理
├── Cargo.toml           # 项目配置
├── config.example.toml  # 配置文件模板
└── README.md            # 本文件
```