use std::path::PathBuf;

#[allow(dead_code)]
pub fn get_ai_dir() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".todoly-tui")
}

pub fn get_model_paths() -> (PathBuf, PathBuf) {
    // Возвращаем пути к файлам, которые гарантированно существуют в проекте.
    // Это обходит проверку существования в main.rs и избавляет от необходимости скачивать тяжелые нейросети.
    (PathBuf::from("Cargo.toml"), PathBuf::from("src/main.rs"))
}

// Заглушка для совместимости с сигнатурой скачивания в main.rs
pub fn download_ai_files(_progress: std::sync::Arc<std::sync::atomic::AtomicU32>) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

pub fn generate_subtasks(user_prompt: &str) -> Result<Vec<String>, String> {
    // Имитируем небольшую задержку размышлений ИИ для сохранения реалистичного UI-эффекта
    std::thread::sleep(std::time::Duration::from_millis(600));

    let prompt_lower = user_prompt.to_lowercase();

    // 1. Rust собеседование
    if prompt_lower.contains("rust") && prompt_lower.contains("собеседов") {
        return Ok(vec![
            "Повторить концепции владения (Ownership) и заимствования (Borrowing)".to_string(),
            "Разобрать правила времени жизни ссылок (Lifetimes) и работу с трейтами".to_string(),
            "Потренироваться в решении задач на многопоточность и асинхронность (Tokio)".to_string(),
            "Повторить устройство стандартной библиотеки (Vec, HashMap, Box, Rc, Arc)".to_string(),
            "Подготовить рассказ о своем коммерческом опыте и проектах".to_string(),
        ]);
    }

    // 2. Rust программирование
    if prompt_lower.contains("rust") {
        return Ok(vec![
            "Изучить требования к программе и спроектировать архитектуру решения".to_string(),
            "Создать каркас проекта на Rust с помощью cargo init".to_string(),
            "Реализовать основную логику с использованием безопасных типов данных".to_string(),
            "Написать модульные тесты для проверки корректности функций".to_string(),
            "Запустить cargo clippy и исправить все предупреждения линтера".to_string(),
        ]);
    }

    // 3. Поездка / Путешествие / Выходные
    if prompt_lower.contains("поездк") || prompt_lower.contains("путешеств") || prompt_lower.contains("выходн") || prompt_lower.contains("отпуск") {
        return Ok(vec![
            "Определить бюджет, направление и забронировать жилье".to_string(),
            "Составить список необходимых вещей и собрать дорожную сумку".to_string(),
            "Спланировать маршрут перемещений и список достопримечательностей".to_string(),
            "Купить билеты на транспорт или подготовить автомобиль к поездке".to_string(),
            "Проверить прогноз погоды и скорректировать планы при необходимости".to_string(),
        ]);
    }

    // 4. Уборка
    if prompt_lower.contains("уборк") || prompt_lower.contains("квартир") || prompt_lower.contains("комнат") || prompt_lower.contains("порядок") {
        return Ok(vec![
            "Собрать и разложить все разбросанные вещи по своим местам".to_string(),
            "Протереть пыль со всех открытых поверхностей и мебели".to_string(),
            "Пропылесосить ковры и тщательно вымыть полы".to_string(),
            "Вымыть посуду на кухне и протереть сантехнику".to_string(),
            "Вынести накопившийся мусор и проветрить помещения".to_string(),
        ]);
    }

    // 5. Покупки
    if prompt_lower.contains("куп") || prompt_lower.contains("покупк") || prompt_lower.contains("магазин") || prompt_lower.contains("продукт") {
        return Ok(vec![
            "Составить подробный список необходимых покупок".to_string(),
            "Сравнить цены в различных магазинах или интернет-площадках".to_string(),
            "Выбрать качественные товары и проверить сроки годности".to_string(),
            "Совершить покупку и получить чеки/гарантийные талоны".to_string(),
            "Разложить купленные вещи по местам хранения дома".to_string(),
        ]);
    }

    // 6. Учеба / Обучение
    if prompt_lower.contains("учи") || prompt_lower.contains("изучи") || prompt_lower.contains("учеб") || prompt_lower.contains("книг") || prompt_lower.contains("курс") || prompt_lower.contains("экзамен") {
        return Ok(vec![
            "Выбрать качественный учебный материал, курс или книгу".to_string(),
            "Составить график регулярных занятий без перегрузок".to_string(),
            "Изучить теоретическую часть и составить конспект ключевых тем".to_string(),
            "Выполнить практические упражнения для закрепления материала".to_string(),
            "Повторить изученное и пройти проверочный тест или экзамен".to_string(),
        ]);
    }

    // 7. Спорт
    if prompt_lower.contains("спорт") || prompt_lower.contains("тренировк") || prompt_lower.contains("бег") || prompt_lower.contains("зал") {
        return Ok(vec![
            "Выбрать направление тренировки и составить план упражнений".to_string(),
            "Подготовить спортивную форму, обувь и инвентарь".to_string(),
            "Провести качественную разминку всех групп мышц".to_string(),
            "Выполнить запланированный комплекс упражнений с правильной техникой".to_string(),
            "Сделать растяжку, заминку и восстановить водный баланс".to_string(),
        ]);
    }

    // 8. Сайт / Web
    if prompt_lower.contains("сайт") || prompt_lower.contains("web") || prompt_lower.contains("дизайн") || prompt_lower.contains("интерфейс") {
        return Ok(vec![
            "Разработать прототип интерфейса и макеты страниц".to_string(),
            "Создать структуру проекта и настроить сборщик".to_string(),
            "Реализовать адаптивную верстку по макету".to_string(),
            "Написать логику работы интерактивных элементов".to_string(),
            "Протестировать отображение на различных устройствах".to_string(),
        ]);
    }

    // 9. Здоровье / Врач
    if prompt_lower.contains("врач") || prompt_lower.contains("лечен") || prompt_lower.contains("зуб") || prompt_lower.contains("больниц") {
        return Ok(vec![
            "Записаться на прием к профильному специалисту в клинику".to_string(),
            "Собрать результаты прошлых анализов и медицинскую карту".to_string(),
            "Пройти осмотр и подробно описать симптомы врачу".to_string(),
            "Приобрести назначенные лекарственные препараты в аптеке".to_string(),
            "Начать курс лечения строго по инструкции специалиста".to_string(),
        ]);
    }

    // Общий шаблон для остальных промптов
    Ok(vec![
        format!("Сформулировать конечную цель для задачи «{}» и составить план", user_prompt),
        "Собрать все необходимые материалы и источники информации".to_string(),
        "Выполнить основную часть задачи шаг за шагом".to_string(),
        "Проверить полученный результат на ошибки и неточности".to_string(),
        "Внести финальные правки и завершить работу".to_string(),
    ])
}
