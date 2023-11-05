use deeplx_rs::deepl_translate;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        if let Ok(res) = deepl_translate("hello world", "EN", "ZH").await {
            println!("{:?}", res);
        }
    });
}
