use candle_core::quantized::gguf_file;
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
use tokenizers::Tokenizer;

// Вшиваем модель и токенизатор в бинарный файл
const MODEL_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/model.gguf"));
const TOKENIZER_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tokenizer.json"));

pub fn generate_subtasks(user_prompt: &str) -> Result<Vec<String>, String> {
    // 1. Инициализация токенизатора из памяти
    let tokenizer = Tokenizer::from_bytes(TOKENIZER_BYTES).map_err(|e| e.to_string())?;

    // 2. Инициализация модели из памяти на CPU
    let mut reader = std::io::Cursor::new(MODEL_BYTES);
    let gguf = gguf_file::Content::read(&mut reader).map_err(|e| e.to_string())?;
    let mut model = ModelWeights::from_gguf(gguf, &mut reader, &Device::Cpu).map_err(|e| e.to_string())?;

    // 3. Подготовка системного промпта и формата диалога для SmolLM2-Instruct
    let prompt = format!(
        "<|im_start|>system\nYou are a helpful task assistant. Output exactly 5 concrete and short subtasks in Russian. One task per line starting with a dash. Do not write any intro or outro.\n<|im_end|>\n<|im_start|>user\n{}\n<|im_end|>\n<|im_start|>assistant\n",
        user_prompt
    );

    // 4. Токенизация промпта
    let tokens = tokenizer.encode(prompt, true).map_err(|e| e.to_string())?;
    let input_tokens = tokens.get_ids();
    
    // 5. Начальный проход (наполнение KV-кэша)
    let mut index_pos = 0;
    let tensor = Tensor::new(input_tokens, &Device::Cpu).map_err(|e| e.to_string())?.unsqueeze(0).map_err(|e| e.to_string())?;
    let logits = model.forward(&tensor, index_pos).map_err(|e| e.to_string())?;
    index_pos += input_tokens.len();

    // Получаем логиты последнего токена
    let logits = logits.squeeze(0).map_err(|e| e.to_string())?;
    let logits = logits.get(logits.dim(0).map_err(|e| e.to_string())? - 1).map_err(|e| e.to_string())?;
    let logits_vec: Vec<f32> = logits.to_vec1().map_err(|e| e.to_string())?;
    
    // Жадный выбор (Argmax) первого сгенерированного токена
    let mut next_token = logits_vec
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(idx, _)| idx as u32)
        .ok_or("Logits are empty")?;

    // Находим токен завершения генерации
    let eos_token_id = tokenizer.token_to_id("<|im_end|>")
        .or_else(|| tokenizer.token_to_id("<|endoftext|>"))
        .unwrap_or(0);

    let mut generated_tokens = Vec::new();
    let max_new_tokens = 250; // Достаточно для 5 строк списка

    // 6. Цикл генерации последующих токенов
    for _ in 0..max_new_tokens {
        if next_token == eos_token_id {
            break;
        }
        generated_tokens.push(next_token);

        let input = Tensor::new(&[next_token], &Device::Cpu).map_err(|e| e.to_string())?.unsqueeze(0).map_err(|e| e.to_string())?;
        let logits = model.forward(&input, index_pos).map_err(|e| e.to_string())?;
        index_pos += 1;

        let logits = logits.squeeze(0).map_err(|e| e.to_string())?.squeeze(0).map_err(|e| e.to_string())?;
        let logits_vec: Vec<f32> = logits.to_vec1().map_err(|e| e.to_string())?;
        
        next_token = logits_vec
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(idx, _)| idx as u32)
            .ok_or("Logits are empty")?;
    }

    // 7. Декодирование результата в текст
    let generated_text = tokenizer.decode(&generated_tokens, true).map_err(|e| e.to_string())?;

    // 8. Парсинг сгенерированного текста в отдельные задачи
    let mut tasks = Vec::new();
    for line in generated_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Очищаем маркеры списков
        let clean = trimmed
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim_start_matches(|c: char| c.is_ascii_digit())
            .trim_start_matches('.')
            .trim();
            
        if !clean.is_empty() {
            tasks.push(clean.to_string());
        }
    }

    Ok(tasks)
}
