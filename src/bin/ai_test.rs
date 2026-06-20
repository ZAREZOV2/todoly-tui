use std::sync::Arc;
use std::sync::atomic::AtomicU32;

#[path = "../ai.rs"]
mod ai;

fn main() {
    println!("=== ToDoLy AI Test Stand ===");
    let (model_path, tokenizer_path) = ai::get_model_paths();
    println!("Model path: {:?}", model_path);
    println!("Tokenizer path: {:?}", tokenizer_path);

    if !model_path.exists() || !tokenizer_path.exists() {
        println!("Files missing. Downloading...");
        let progress = Arc::new(AtomicU32::new(0));
        let progress_clone = progress.clone();
        
        // Spawn status printing thread
        std::thread::spawn(move || {
            let mut last = 0;
            while last < 100 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let val = progress_clone.load(std::sync::atomic::Ordering::Relaxed);
                if val != last {
                    println!("Download progress: {}%", val);
                    last = val;
                }
            }
        });

        match ai::download_ai_files(progress) {
            Ok(_) => println!("Download complete!"),
            Err(e) => {
                eprintln!("Download failed: {}", e);
                return;
            }
        }
    } else {
        println!("Files already present!");
    }

    let prompts = vec![
        "Подготовиться к Rust-собеседованию",
        "Спланировать поездку на выходные",
    ];

    for prompt in prompts {
        println!("\nGenerating subtasks for: '{}'...", prompt);
        let start = std::time::Instant::now();
        match ai::generate_subtasks(prompt) {
            Ok(tasks) => {
                let duration = start.elapsed();
                println!("Success! Generation took: {:?}", duration);
                println!("Generated tasks:");
                for (i, task) in tasks.iter().enumerate() {
                    println!("  {}. {}", i + 1, task);
                }
            }
            Err(e) => {
                eprintln!("Generation failed: {}", e);
            }
        }
    }
}
