#[napi]
pub async fn greet(name: Buffer) -> String {
    format!("hello {}", String::from_utf8_lossy(&name))
}
