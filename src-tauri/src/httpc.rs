use std::time::Duration;

/// 统一的 HTTP 客户端工厂。
///
/// 关键点：**本机地址绝不能走系统代理**。
/// 用户装了 Clash / v2ray 之类，会设置 HTTP_PROXY；reqwest 默认读到它，
/// 于是 http://127.0.0.1:9090 这种请求被丢给代理，代理连不上 → 502 / 掉线。
pub fn client_for(url: &str) -> Result<reqwest::Client, String> {
    let local = url.contains("127.0.0.1")
        || url.contains("localhost")
        || url.contains("[::1]")
        || url.contains("0.0.0.0");
    let mut b = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .user_agent("axiao-gateway");
    if local {
        b = b.no_proxy();
    }
    b.build().map_err(|e| e.to_string())
}

/// 流式专用客户端：**不设整请求超时**。
///
/// 普通请求用 180 秒总超时是对的，但流式生成长文可能要几分钟，
/// 总超时会把正常的长回复从中间掐断。这里只限制"连不上"的时间，
/// 读数据阶段的超时交给调用方按「相邻两帧间隔」判断。
pub fn stream_client_for(url: &str) -> Result<reqwest::Client, String> {
    let mut b = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .user_agent("axiao-gateway");
    if is_local(url) {
        b = b.no_proxy();
    }
    b.build().map_err(|e| e.to_string())
}

pub fn is_local(url: &str) -> bool {
    url.contains("127.0.0.1") || url.contains("localhost") || url.contains("[::1]")
}
