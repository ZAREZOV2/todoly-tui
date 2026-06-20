use std::env;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let model_dest = Path::new(&out_dir).join("model.gguf");
    let tokenizer_dest = Path::new(&out_dir).join("tokenizer.json");

    let agent = ureq::AgentBuilder::new()
        .timeout_read(Duration::from_secs(600)) // 10 minutes timeout for slow connections
        .timeout_connect(Duration::from_secs(15))
        .build();

    // 1. Скачивание квантованной модели (90MB)
    if !model_dest.exists() {
        println!("cargo:warning=Downloading SmolLM2 local AI model (~90MB)... This may take a minute.");
        let response = agent.get("https://huggingface.co/bartowski/SmolLM2-135M-Instruct-GGUF/resolve/main/SmolLM2-135M-Instruct-Q4_K_M.gguf")
            .set("User-Agent", "ToDoLy-Build")
            .call();
        
        match response {
            Ok(res) => {
                let mut out = File::create(&model_dest).unwrap();
                let mut reader = res.into_reader();
                std::io::copy(&mut reader, &mut out).unwrap();
                println!("cargo:warning=Model downloaded successfully.");
            }
            Err(e) => {
                panic!("Failed to download model weights from Hugging Face: {}. Check internet connection.", e);
            }
        }
    }

    // 2. Скачивание токенизатора (2MB)
    if !tokenizer_dest.exists() {
        println!("cargo:warning=Downloading tokenizer (~2MB)...");
        let response = agent.get("https://huggingface.co/HuggingFaceTB/SmolLM2-135M-Instruct/resolve/main/tokenizer.json")
            .set("User-Agent", "ToDoLy-Build")
            .call();
        
        match response {
            Ok(res) => {
                let mut out = File::create(&tokenizer_dest).unwrap();
                let mut reader = res.into_reader();
                std::io::copy(&mut reader, &mut out).unwrap();
                println!("cargo:warning=Tokenizer downloaded successfully.");
            }
            Err(e) => {
                panic!("Failed to download tokenizer from Hugging Face: {}. Check internet connection.", e);
            }
        }
    }

    // Заставляем Cargo перезапускать build.rs только если изменился сам build.rs
    println!("cargo:rerun-if-changed=build.rs");
}
