pub fn new_reqwest_client() -> reqwest::ClientBuilder {
    let mut builder = reqwest::Client::builder();
    // if let Ok(http_proxy) = env::var("HTTP_PROXY") {
    builder = builder.proxy(reqwest::Proxy::http("http://0.0.0.0:1087").unwrap());
    // }
    // if let Ok(https_proxy) = env::var("HTTPS_PROXY") {
    builder = builder.proxy(reqwest::Proxy::https("http://0.0.0.0:1087").unwrap());
    // }
    builder = builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36 Edg/110.0.1587.69");
    builder
}
