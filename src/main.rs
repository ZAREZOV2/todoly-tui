use std::fs::File; // Для работы с файлами
use std::io::{self, BufRead, BufReader, Write}; // Ввод-вывод, чтение и запись
use std::path::Path; // Для проверки существования пути

// Подключаем необходимые элементы из библиотеки crossterm (управление терминалом)
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// Подключаем необходимые элементы из библиотеки ratatui (интерфейс)
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Gauge, Clear},
    Terminal,
};

const FILE_PATH: &str = "tasks.txt";
const TRASH_PATH: &str = "trash.txt";

mod ai;

// Приоритеты задач
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

// Структура задачи с метаданными
struct Task {
    title: String,
    completed: bool,
    priority: Priority,
    created_at: String,
    modified_at: String,
}

// Помощники для приоритетов
fn priority_emoji(p: Priority) -> &'static str {
    match p {
        Priority::Low => "🟢",
        Priority::Medium => "🟡",
        Priority::High => "🟠",
        Priority::Critical => "🔴",
    }
}

fn priority_label(p: Priority) -> &'static str {
    match p {
        Priority::Low => "Низкий",
        Priority::Medium => "Средний",
        Priority::High => "Высокий",
        Priority::Critical => "Критический",
    }
}

fn priority_from_str(s: &str) -> Priority {
    match s {
        "Critical" => Priority::Critical,
        "High" => Priority::High,
        "Medium" => Priority::Medium,
        _ => Priority::Low,
    }
}

fn priority_to_str(p: Priority) -> &'static str {
    match p {
        Priority::Low => "Low",
        Priority::Medium => "Medium",
        Priority::High => "High",
        Priority::Critical => "Critical",
    }
}

// Сохранение и загрузка списка задач
fn save_tasks_to_file(tasks: &Vec<Task>, path: &str) {
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    for task in tasks {
        let line = format!(
            "{}|{}|{}|{}|{}\n",
            task.completed,
            priority_to_str(task.priority),
            task.created_at,
            task.modified_at,
            task.title
        );
        let _ = file.write_all(line.as_bytes());
    }
}

fn load_tasks_from_file(path: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    if !Path::new(path).exists() {
        return tasks;
    }
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return tasks,
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        if let Ok(content) = line {
            let parts: Vec<&str> = content.split('|').collect();
            if parts.len() == 2 {
                // Поддержка старого формата (совместимость)
                let completed = parts[0] == "true";
                let title = parts[1].to_string();
                let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                tasks.push(Task {
                    title,
                    completed,
                    priority: Priority::Low,
                    created_at: now.clone(),
                    modified_at: now,
                });
            } else if parts.len() >= 5 {
                // Новый расширенный формат
                let completed = parts[0] == "true";
                let priority = priority_from_str(parts[1]);
                let created_at = parts[2].to_string();
                let modified_at = parts[3].to_string();
                let title = parts[4..].join("|");
                tasks.push(Task {
                    title,
                    completed,
                    priority,
                    created_at,
                    modified_at,
                });
            }
        }
    }
    tasks
}

// Цветовые темы оформления
#[derive(Clone, Copy)]
struct Theme {
    name: &'static str,
    border_color: Color,
    selected_bg: Color,
    selected_fg: Color,
    completed_fg: Color,
    accent: Color,
}

const THEMES: [Theme; 8] = [
    Theme {
        name: "Classic Blue",
        border_color: Color::Blue,
        selected_bg: Color::Blue,
        selected_fg: Color::White,
        completed_fg: Color::DarkGray,
        accent: Color::Cyan,
    },
    Theme {
        name: "Emerald Green",
        border_color: Color::Green,
        selected_bg: Color::Green,
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::LightGreen,
    },
    Theme {
        name: "Cyberpunk Magenta",
        border_color: Color::Magenta,
        selected_bg: Color::Magenta,
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::LightMagenta,
    },
    Theme {
        name: "Nordic Ice",
        border_color: Color::Cyan,
        selected_bg: Color::Cyan,
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::LightCyan,
    },
    Theme {
        name: "Solarized Amber",
        border_color: Color::Yellow,
        selected_bg: Color::Yellow,
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::LightYellow,
    },
    Theme {
        name: "Dracula Purple",
        border_color: Color::Rgb(189, 147, 249),
        selected_bg: Color::Rgb(189, 147, 249),
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::Rgb(255, 121, 198),
    },
    Theme {
        name: "Sunset Coral",
        border_color: Color::Rgb(255, 69, 0),
        selected_bg: Color::Rgb(255, 127, 80),
        selected_fg: Color::White,
        completed_fg: Color::DarkGray,
        accent: Color::Rgb(255, 99, 71),
    },
    Theme {
        name: "Monochrome",
        border_color: Color::Gray,
        selected_bg: Color::White,
        selected_fg: Color::Black,
        completed_fg: Color::DarkGray,
        accent: Color::Gray,
    },
];

// Состояния нашего интерфейса
enum InputMode {
    Normal,
    Adding,
    EditingTitle,
    Searching,
    AiPrompt,
}

// Режимы просмотра
#[derive(PartialEq, Eq, Clone, Copy)]
enum ViewMode {
    Active,
    Trash,
}

// Подтверждаемые действия
#[derive(Clone, Copy, PartialEq, Eq)]
enum ConfirmAction {
    DeletePermanent(usize), // Индекс задачи в векторе trash
    ClearTrash,
}

// Статус фоновой проверки обновлений
#[derive(Clone, PartialEq, Eq, Debug)]
enum UpdateStatus {
    Checking,
    Latest,
    NewVersion(String),
    Error,
}

// Структура для управления состоянием всего приложения
struct App {
    tasks: Vec<Task>,
    trash: Vec<Task>,
    list_state: ListState,
    trash_list_state: ListState,
    input: String,
    input_mode: InputMode,
    view_mode: ViewMode,
    current_theme_index: usize,
    show_help: bool,
    show_about: bool,
    search_query: String,
    show_confirm: bool,
    confirm_action: Option<ConfirmAction>,
    update_status: UpdateStatus,
    ai_generating: bool,
    ai_downloading: bool,
    ai_download_progress: std::sync::Arc<std::sync::atomic::AtomicU32>,
    ai_error: Option<String>,
}

impl App {
    fn new(tasks: Vec<Task>, trash: Vec<Task>) -> App {
        let mut list_state = ListState::default();
        if !tasks.is_empty() {
            list_state.select(Some(0));
        }
        let mut trash_list_state = ListState::default();
        if !trash.is_empty() {
            trash_list_state.select(Some(0));
        }

        App {
            tasks,
            trash,
            list_state,
            trash_list_state,
            input: String::new(),
            input_mode: InputMode::Normal,
            view_mode: ViewMode::Active,
            current_theme_index: 0,
            show_help: false,
            show_about: false,
            search_query: String::new(),
            show_confirm: false,
            confirm_action: None,
            update_status: UpdateStatus::Checking,
            ai_generating: false,
            ai_downloading: false,
            ai_download_progress: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
            ai_error: None,
        }
    }

    fn theme(&self) -> Theme {
        THEMES[self.current_theme_index]
    }

    // Возвращает список индексов задач, отфильтрованных по поисковому запросу
    fn filtered_tasks(&self) -> Vec<usize> {
        let query = self.search_query.to_lowercase();
        self.tasks
            .iter()
            .enumerate()
            .filter(|(_, task)| task.title.to_lowercase().contains(&query))
            .map(|(idx, _)| idx)
            .collect()
    }

    // Возвращает список индексов удаленных задач в корзине, отфильтрованных по поисковому запросу
    fn filtered_trash(&self) -> Vec<usize> {
        let query = self.search_query.to_lowercase();
        self.trash
            .iter()
            .enumerate()
            .filter(|(_, task)| task.title.to_lowercase().contains(&query))
            .map(|(idx, _)| idx)
            .collect()
    }

    // Навигация вниз по отфильтрованному списку
    fn next(&mut self) {
        match self.view_mode {
            ViewMode::Active => {
                let filtered = self.filtered_tasks();
                if filtered.is_empty() {
                    self.list_state.select(None);
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= filtered.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            ViewMode::Trash => {
                let filtered = self.filtered_trash();
                if filtered.is_empty() {
                    self.trash_list_state.select(None);
                    return;
                }
                let i = match self.trash_list_state.selected() {
                    Some(i) => {
                        if i >= filtered.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.trash_list_state.select(Some(i));
            }
        }
    }

    // Навигация вверх по отфильтрованному списку
    fn previous(&mut self) {
        match self.view_mode {
            ViewMode::Active => {
                let filtered = self.filtered_tasks();
                if filtered.is_empty() {
                    self.list_state.select(None);
                    return;
                }
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            filtered.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            ViewMode::Trash => {
                let filtered = self.filtered_trash();
                if filtered.is_empty() {
                    self.trash_list_state.select(None);
                    return;
                }
                let i = match self.trash_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            filtered.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.trash_list_state.select(Some(i));
            }
        }
    }

    // Переключение статуса выполнения (активные) или восстановление (из корзины)
    fn toggle_or_restore(&mut self) {
        match self.view_mode {
            ViewMode::Active => {
                let filtered = self.filtered_tasks();
                if let Some(v_idx) = self.list_state.selected() {
                    if v_idx < filtered.len() {
                        let real_idx = filtered[v_idx];
                        self.tasks[real_idx].completed = !self.tasks[real_idx].completed;
                        self.tasks[real_idx].modified_at = chrono::Local::now()
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string();
                        save_tasks_to_file(&self.tasks, FILE_PATH);
                    }
                }
            }
            ViewMode::Trash => {
                let filtered = self.filtered_trash();
                if let Some(v_idx) = self.trash_list_state.selected() {
                    if v_idx < filtered.len() {
                        let real_idx = filtered[v_idx];
                        let mut restored = self.trash.remove(real_idx);
                        restored.modified_at = chrono::Local::now()
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string();
                        self.tasks.push(restored);

                        save_tasks_to_file(&self.tasks, FILE_PATH);
                        save_tasks_to_file(&self.trash, TRASH_PATH);

                        let new_len = self.filtered_trash().len();
                        if new_len == 0 {
                            self.trash_list_state.select(None);
                        } else if v_idx >= new_len {
                            self.trash_list_state.select(Some(new_len - 1));
                        } else {
                            self.trash_list_state.select(Some(v_idx));
                        }
                        self.list_state.select(Some(self.tasks.len() - 1));
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    let tasks = load_tasks_from_file(FILE_PATH);
    let trash = load_tasks_from_file(TRASH_PATH);
    let app = App::new(tasks, trash);

    let (update_tx, update_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = check_github_update();
        let status = match result {
            Ok(tag) => {
                let current_version = env!("CARGO_PKG_VERSION");
                let clean_tag = tag.trim_start_matches('v');
                let clean_current = current_version.trim_start_matches('v');
                if clean_tag != clean_current {
                    UpdateStatus::NewVersion(tag)
                } else {
                    UpdateStatus::Latest
                }
            }
            Err(_) => UpdateStatus::Error,
        };
        let _ = update_tx.send(status);
    });

    let (ai_tx, ai_rx) = std::sync::mpsc::channel();

    let res = run_app(&mut terminal, app, update_rx, ai_rx, ai_tx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Произошла ошибка: {:?}", err);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    update_rx: std::sync::mpsc::Receiver<UpdateStatus>,
    ai_rx: std::sync::mpsc::Receiver<Result<Vec<String>, String>>,
    ai_tx: std::sync::mpsc::Sender<Result<Vec<String>, String>>,
) -> io::Result<()> {
    loop {
        if app.update_status == UpdateStatus::Checking {
            if let Ok(status) = update_rx.try_recv() {
                app.update_status = status;
            }
        }

        // Проверяем результаты фоновых задач ИИ (скачивание или генерация)
        if let Ok(result) = ai_rx.try_recv() {
            if app.ai_downloading {
                app.ai_downloading = false;
                match result {
                    Ok(_) => {
                        app.input_mode = InputMode::AiPrompt;
                        app.input.clear();
                    }
                    Err(e) => {
                        app.ai_error = Some(e);
                    }
                }
            } else if app.ai_generating {
                app.ai_generating = false;
                match result {
                    Ok(new_tasks) => {
                        let now_str = chrono::Local::now()
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string();
                        for task_title in new_tasks {
                            app.tasks.push(Task {
                                title: task_title,
                                completed: false,
                                priority: Priority::Medium,
                                created_at: now_str.clone(),
                                modified_at: now_str.clone(),
                            });
                        }
                        save_tasks_to_file(&app.tasks, FILE_PATH);
                        if !app.tasks.is_empty() {
                            app.list_state.select(Some(app.tasks.len() - 1));
                        }
                    }
                    Err(e) => {
                        app.ai_error = Some(e);
                    }
                }
            }
        }

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Шапка (Заголовок + Прогресс)
                    Constraint::Min(3),    // Основная область
                    Constraint::Length(3), // Футер (Управление)
                ])
                .split(f.area());

            let theme = app.theme();

            // 1. Шапка: Разделяем по горизонтали на название и прогресс-бар
            let header_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(chunks[0]);

            let header_text = format!(
                "Умный органайзер задач в терминале | Тема: {}",
                theme.name
            );
            let header = Paragraph::new(header_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_color))
                    .title(" ToDoLy "),
            );
            f.render_widget(header, header_chunks[0]);

            // Вычисляем прогресс выполнения задач
            let total_tasks = app.tasks.len();
            let completed_tasks = app.tasks.iter().filter(|t| t.completed).count();
            let percent = if total_tasks > 0 {
                (completed_tasks as f32 / total_tasks as f32 * 100.0) as u16
            } else {
                0
            };

            let progress_gauge = Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border_color))
                        .title(" Выполнено "),
                )
                .gauge_style(
                    Style::default()
                        .fg(theme.accent)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .percent(percent);
            f.render_widget(progress_gauge, header_chunks[1]);

            // Разделяем основную область (chunks[1]) горизонтально:
            // - Список задач (60%)
            // - Подробная информация (40%)
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(chunks[1]);

            // Фильтруем списки для отображения в зависимости от поискового запроса
            let filtered_task_indices = app.filtered_tasks();
            let filtered_trash_indices = app.filtered_trash();

            let (list_title, list_items) = match app.view_mode {
                ViewMode::Active => {
                    let items: Vec<ListItem> = filtered_task_indices
                        .iter()
                        .map(|&idx| {
                            let task = &app.tasks[idx];
                            let status = if task.completed { "[x] " } else { "[ ] " };
                            let prio = if task.completed { "● " } else { priority_emoji(task.priority) };
                            let content = format!("{} {} {}", prio, status, task.title);
                            ListItem::new(content).style(if task.completed {
                                Style::default().fg(theme.completed_fg)
                            } else {
                                Style::default()
                            })
                        })
                        .collect();

                    let title = if app.search_query.is_empty() {
                        " Активные задачи ".to_string()
                    } else {
                        format!(" Результаты поиска ('{}') ", app.search_query)
                    };

                    (title, items)
                }
                ViewMode::Trash => {
                    let items: Vec<ListItem> = filtered_trash_indices
                        .iter()
                        .map(|&idx| {
                            let task = &app.trash[idx];
                            let prio = if task.completed { "● " } else { priority_emoji(task.priority) };
                            let content = format!("{} [УДАЛЕНА] {}", prio, task.title);
                            ListItem::new(content).style(Style::default().fg(Color::DarkGray))
                        })
                        .collect();

                    let title = if app.search_query.is_empty() {
                        " Корзина (Удаленные задачи) ".to_string()
                    } else {
                        format!(" Поиск в корзине ('{}') ", app.search_query)
                    };

                    (title, items)
                }
            };

            // Отрисовка списка
            let list_widget = List::new(list_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border_color))
                        .title(list_title),
                )
                .highlight_style(
                    Style::default()
                        .bg(theme.selected_bg)
                        .fg(theme.selected_fg)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            match app.view_mode {
                ViewMode::Active => {
                    f.render_stateful_widget(list_widget, main_chunks[0], &mut app.list_state)
                }
                ViewMode::Trash => {
                    f.render_stateful_widget(list_widget, main_chunks[0], &mut app.trash_list_state)
                }
            }

            // Получаем выбранную задачу ПОСЛЕ рендеринга списка
            let selected_task = match app.view_mode {
                ViewMode::Active => app
                    .list_state
                    .selected()
                    .and_then(|idx| filtered_task_indices.get(idx))
                    .and_then(|&real_idx| app.tasks.get(real_idx)),
                ViewMode::Trash => app
                    .trash_list_state
                    .selected()
                    .and_then(|idx| filtered_trash_indices.get(idx))
                    .and_then(|&real_idx| app.trash.get(real_idx)),
            };

            // Рисуем карточку подробных метаданных
            let details_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_color))
                .title(" Детали задачи ");

            let details_text = if let Some(task) = selected_task {
                let status_str = if task.completed {
                    "Выполнена [x]"
                } else {
                    "Активна [ ]"
                };
                let prio_lbl = priority_label(task.priority);
                let prio_em = priority_emoji(task.priority);
                vec![
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            "Название: ",
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                        ratatui::text::Span::raw(&task.title),
                    ]),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            "Статус:   ",
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                        ratatui::text::Span::raw(status_str),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            "Приоритет:",
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                        if task.completed {
                            ratatui::text::Span::styled(
                                format!(" ● {}", prio_lbl),
                                Style::default().fg(Color::DarkGray),
                            )
                        } else {
                            ratatui::text::Span::raw(format!(" {} {}", prio_em, prio_lbl))
                        },
                    ]),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            "Создана:  ",
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                        ratatui::text::Span::raw(&task.created_at),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            "Изменена: ",
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                        ratatui::text::Span::raw(&task.modified_at),
                    ]),
                ]
            } else {
                vec![ratatui::text::Line::from(
                    "Выберите задачу для просмотра подробной информации.",
                )]
            };

            let details_widget = Paragraph::new(details_text).block(details_block);
            f.render_widget(details_widget, main_chunks[1]);

            // 3. Отрисовка панели действий / строки ввода
            let footer_text = match app.input_mode {
                InputMode::Normal => {
                    if app.ai_generating {
                        " [ИИ-АССИСТЕНТ] Локальный ИИ генерирует подзадачи... Пожалуйста, подождите. ".to_string()
                    } else {
                        " F1 - Справка | F2 - О программе | i/ш - ИИ-помощник | q/й - Выход ".to_string()
                    }
                }
                InputMode::Adding => {
                    format!(
                        " [ДОБАВЛЕНИЕ] Имя новой задачи: {} (Enter — сохранить, Esc — отмена)",
                        app.input
                    )
                }
                InputMode::EditingTitle => {
                    format!(
                        " [РЕДАКТИРОВАНИЕ] Изменить имя: {} (Enter — сохранить, Esc — отмена)",
                        app.input
                    )
                }
                InputMode::Searching => {
                    format!(
                        " [ПОИСК] Запрос: {} (Enter — зафиксировать поиск, Esc — сбросить поиск)",
                        app.input
                    )
                }
                InputMode::AiPrompt => {
                    format!(
                        " [ИИ-АССИСТЕНТ] Запрос на генерацию подзадач: {} (Enter — сгенерировать, Esc — отмена)",
                        app.input
                    )
                }
            };

            let footer = Paragraph::new(footer_text).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_color))
                    .title(" Управление "),
            );
            f.render_widget(footer, chunks[2]);

            // Рисуем всплывающее окно справки FAQ (F1), если оно активно
            if app.show_help {
                let area = centered_rect(70, 75, f.area());
                let help_block = Block::default()
                    .title(" Справка по управлению (FAQ) ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent));

                let help_text = vec![
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Добавление задач:  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'a' (или 'ф'). Введите имя и нажмите Enter."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("ИИ-Ассистент:      ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'i' (или 'ш'). Генерация списка из 5 подзадач локальной моделью."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Редактирование:    ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'e' (или 'у') позволяет переименовать задачу."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Поиск / Фильтр:    ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша '/' (или '.' / 'ф'). Esc сбрасывает поиск."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Выполнить/Активно: ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша Space (Пробел) или Enter."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Очистка вып. дел:  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("В списке нажмите 'c' (или 'с') для отправки вып. задач в корзину."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Удаление в корзину:", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'd' (или 'в') отправляет выбранную задачу в корзину."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Корзина / Список:  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'r' (или 'к') переключает режим просмотра."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Восстановление:    ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("В корзине нажмите Space или Enter на задаче."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Очистить корзину:  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("В корзине нажмите 'c' (или 'с') для полной очистки (с подтв.)."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Удаление насовсем: ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("В корзине нажмите 'd' для удаления навсегда (с подтв.)."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Смена тем (8 шт):  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 't' (или 'е') переключает темы оформления."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Смена приоритета:  ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Клавиша 'p' (или 'з') меняет приоритет ( Low -> Medium -> High -> Critical )."),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Перемещение задач: ", Style::default().add_modifier(Modifier::BOLD).fg(theme.accent)),
                        ratatui::text::Span::raw("Shift+↑/↓, Alt+↑/↓ или Shift+J/K меняет их порядок."),
                    ]),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Для закрытия этого окна нажмите F1 или Esc.", Style::default().fg(Color::Gray)),
                    ]),
                ];

                let help_widget = Paragraph::new(help_text)
                    .block(help_block)
                    .alignment(ratatui::layout::Alignment::Left);

                f.render_widget(Clear, area); // Очищаем фон за модальным окном
                f.render_widget(help_widget, area);
            }

            // Рисуем всплывающее окно "О программе" (F2), если оно активно
            if app.show_about {
                let area = centered_rect(50, 40, f.area());
                let about_block = Block::default()
                    .title(" О программе ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent));

                let update_line = match &app.update_status {
                    UpdateStatus::Checking => ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Проверка обновлений...", Style::default().fg(Color::Gray))
                    ]),
                    UpdateStatus::Latest => ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Установлена последняя версия", Style::default().fg(Color::Green))
                    ]),
                    UpdateStatus::NewVersion(tag) => ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(format!("Доступна новая версия {}!", tag), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                    ]),
                    UpdateStatus::Error => ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Не удалось проверить обновления", Style::default().fg(Color::Red))
                    ]),
                };

                let about_text = vec![
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(
                            format!("ToDoLy v{}", env!("CARGO_PKG_VERSION")),
                            Style::default().add_modifier(Modifier::BOLD).fg(theme.accent),
                        ),
                    ]),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Разработчик: ", Style::default().add_modifier(Modifier::BOLD)),
                        ratatui::text::Span::raw("Bagrov"),
                    ]),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Discord:     ", Style::default().add_modifier(Modifier::BOLD)),
                        ratatui::text::Span::raw("@console.x"),
                    ]),
                    ratatui::text::Line::from(""),
                    update_line,
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled("Для закрытия этого окна нажмите F2 или Esc.", Style::default().fg(Color::Gray)),
                    ]),
                ];

                let about_widget = Paragraph::new(about_text)
                    .block(about_block)
                    .alignment(ratatui::layout::Alignment::Center);

                f.render_widget(Clear, area); // Очищаем фон за модальным окном
                f.render_widget(about_widget, area);
            }

            // Рисуем всплывающее окно подтверждения (поверх всего), если активно
            if app.show_confirm {
                let area = centered_rect(50, 25, f.area());
                let confirm_block = Block::default()
                    .title(" Подтверждение действия ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red));

                let action_msg = match app.confirm_action {
                    Some(ConfirmAction::DeletePermanent(_)) => "Удалить эту задачу из корзины НАВСЕГДА?",
                    Some(ConfirmAction::ClearTrash) => "Очистить ВСЮ корзину навсегда?",
                    None => "",
                };

                let confirm_text = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        action_msg,
                        Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
                    )),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(vec![
                        ratatui::text::Span::styled(" [y] Да (н) ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                        ratatui::text::Span::raw(" / "),
                        ratatui::text::Span::styled(" [n] Нет (т) ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    ]),
                ];

                let confirm_widget = Paragraph::new(confirm_text)
                    .block(confirm_block)
                    .alignment(ratatui::layout::Alignment::Center);

                f.render_widget(Clear, area); // Очищаем фон за модальным окном
                f.render_widget(confirm_widget, area);
            }

            // Рисуем всплывающее окно ввода промпта для ИИ
            if let InputMode::AiPrompt = app.input_mode {
                let area = centered_rect(65, 25, f.area());
                let block = Block::default()
                    .title(" ИИ-Ассистент: Создать подзадачи ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent));

                let text = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(" Опишите вашу цель на русском (например: 'Подготовка к Rust-интервью'):"),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        format!(" > {}", app.input),
                        Style::default().add_modifier(Modifier::BOLD).fg(Color::White),
                    )),
                ];

                let widget = Paragraph::new(text)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Left);

                f.render_widget(Clear, area);
                f.render_widget(widget, area);
            }

            // Рисуем индикатор генерации ИИ
            if app.ai_generating {
                let area = centered_rect(50, 20, f.area());
                let block = Block::default()
                    .title(" ИИ-Ассистент ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent));

                let text = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        " Локальный ИИ (SmolLM2) думает... ",
                        Style::default().add_modifier(Modifier::BOLD).fg(Color::Yellow),
                    )),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(" Генерация списка из 5 подзадач. Пожалуйста, подождите... "),
                ];

                let widget = Paragraph::new(text)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center);

                f.render_widget(Clear, area);
                f.render_widget(widget, area);
            }

            // Рисуем всплывающее окно скачивания ИИ модели
            if app.ai_downloading {
                let area = centered_rect(60, 25, f.area());
                let block = Block::default()
                    .title(" ИИ-Ассистент: Загрузка модели ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.accent));

                let pct = app.ai_download_progress.load(std::sync::atomic::Ordering::Relaxed);
                
                let progress_gauge = Gauge::default()
                    .block(Block::default().borders(Borders::NONE))
                    .gauge_style(
                        Style::default()
                            .fg(theme.accent)
                            .bg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD),
                    )
                    .percent(pct as u16);

                let text = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(" Скачивание локальной модели ИИ (SmolLM2, ~90 МБ)..."),
                    ratatui::text::Line::from(" Это требуется только при первом запуске ассистента."),
                    ratatui::text::Line::from(""),
                ];

                let paragraph = Paragraph::new(text)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Left);

                f.render_widget(Clear, area);
                f.render_widget(paragraph, area);
                
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(5),
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(area);
                f.render_widget(progress_gauge, inner_layout[1]);
            }

            // Рисуем всплывающее окно ошибки ИИ
            if let Some(err_msg) = &app.ai_error {
                let area = centered_rect(60, 30, f.area());
                let block = Block::default()
                    .title(" Ошибка ИИ-Ассистента ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red));

                let text = vec![
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        " Произошла ошибка при работе с ИИ: ",
                        Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
                    )),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(err_msg.as_str()),
                    ratatui::text::Line::from(""),
                    ratatui::text::Line::from(ratatui::text::Span::styled(
                        " Нажмите Esc для закрытия этого сообщения. ",
                        Style::default().fg(Color::Gray),
                    )),
                ];

                let widget = Paragraph::new(text)
                    .block(block)
                    .alignment(ratatui::layout::Alignment::Center);

                f.render_widget(Clear, area);
                f.render_widget(widget, area);
            }
        })?;

        // Считываем и обрабатываем клавиатурные события
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            // 1. Обработка подтверждений (наивысший приоритет)
            if app.show_confirm {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('н') | KeyCode::Char('Н') => {
                        if let Some(action) = app.confirm_action {
                            match action {
                                ConfirmAction::DeletePermanent(real_idx) => {
                                    if real_idx < app.trash.len() {
                                        app.trash.remove(real_idx);
                                        save_tasks_to_file(&app.trash, TRASH_PATH);
                                        
                                        // Корректируем выделенный индекс
                                        let filtered_len = app.filtered_trash().len();
                                        if filtered_len == 0 {
                                            app.trash_list_state.select(None);
                                        } else if let Some(v_idx) = app.trash_list_state.selected() {
                                            if v_idx >= filtered_len {
                                                app.trash_list_state.select(Some(filtered_len - 1));
                                            }
                                        }
                                    }
                                }
                                ConfirmAction::ClearTrash => {
                                    app.trash.clear();
                                    save_tasks_to_file(&app.trash, TRASH_PATH);
                                    app.trash_list_state.select(None);
                                }
                            }
                        }
                        app.show_confirm = false;
                        app.confirm_action = None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('т') | KeyCode::Char('Т') | KeyCode::Esc => {
                        app.show_confirm = false;
                        app.confirm_action = None;
                    }
                    KeyCode::Char('q') | KeyCode::Char('й') | KeyCode::Char('Q') | KeyCode::Char('Й') => return Ok(()),
                    _ => {}
                }
                continue;
            }

            // 2. Переключение окон F1 / F2 (работает в любом режиме, кроме подтверждения)
            if key.code == KeyCode::F(1) {
                app.show_help = !app.show_help;
                app.show_about = false;
                continue;
            }
            if key.code == KeyCode::F(2) {
                app.show_about = !app.show_about;
                app.show_help = false;
                continue;
            }

            // 3. Закрытие окон F1 / F2 / Ошибок по Esc
            if app.show_help || app.show_about || app.ai_error.is_some() {
                match key.code {
                    KeyCode::Esc => {
                        app.show_help = false;
                        app.show_about = false;
                        app.ai_error = None;
                    }
                    KeyCode::F(1) if app.show_help => app.show_help = false,
                    KeyCode::F(2) if app.show_about => app.show_about = false,
                    KeyCode::Char('q') | KeyCode::Char('й') | KeyCode::Char('Q') | KeyCode::Char('Й') => return Ok(()),
                    _ => {}
                }
                continue;
            }

            // 4. Обычная обработка в зависимости от режима ввода
            match app.input_mode {
                InputMode::Normal => {
                    // Перемещение задач Alt/Shift + Стрелки (только когда поиск пуст)
                    if key.modifiers.contains(KeyModifiers::ALT) || key.modifiers.contains(KeyModifiers::SHIFT) {
                        if app.view_mode == ViewMode::Active && app.search_query.is_empty() {
                            if let Some(index) = app.list_state.selected() {
                                let is_up = match key.code {
                                    KeyCode::Up => true,
                                    KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Char('л') | KeyCode::Char('Л') => true,
                                    _ => false,
                                };
                                let is_down = match key.code {
                                    KeyCode::Down => true,
                                    KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Char('о') | KeyCode::Char('О') => true,
                                    _ => false,
                                };

                                if is_up {
                                    if index > 0 && index < app.tasks.len() {
                                        app.tasks.swap(index, index - 1);
                                        app.list_state.select(Some(index - 1));
                                        save_tasks_to_file(&app.tasks, FILE_PATH);
                                    }
                                } else if is_down {
                                    if index < app.tasks.len() - 1 {
                                        app.tasks.swap(index, index + 1);
                                        app.list_state.select(Some(index + 1));
                                        save_tasks_to_file(&app.tasks, FILE_PATH);
                                    }
                                }
                            }
                        }
                        continue;
                    }

                    // Нормализация русской раскладки
                    if let KeyCode::Char(c) = key.code {
                        let normalized_char = match c {
                            'q' | 'й' | 'Q' | 'Й' => 'q',
                            'a' | 'ф' | 'A' | 'Ф' => 'a',
                            'd' | 'в' | 'D' | 'В' => 'd',
                            't' | 'е' | 'T' | 'Е' => 't',
                            'r' | 'к' | 'R' | 'К' => 'r',
                            'e' | 'у' | 'E' | 'У' => 'e',
                            'p' | 'з' | 'P' | 'З' => 'p',
                            'c' | 'с' | 'C' | 'С' => 'c',
                            'j' | 'о' | 'J' | 'О' => 'j',
                            'k' | 'л' | 'K' | 'Л' => 'k',
                            'i' | 'ш' | 'I' | 'Ш' => 'i',
                            ' ' => ' ',
                            '/' | '.' => '/',
                            other => other,
                        };

                        match normalized_char {
                            'q' => return Ok(()),
                            'j' => app.next(),
                            'k' => app.previous(),
                            ' ' => app.toggle_or_restore(),
                            'i' => {
                                if app.view_mode == ViewMode::Active && !app.ai_generating && !app.ai_downloading {
                                    let (model_path, tokenizer_path) = ai::get_model_paths();
                                    if model_path.exists() && tokenizer_path.exists() {
                                        app.input_mode = InputMode::AiPrompt;
                                        app.input.clear();
                                    } else {
                                        app.ai_downloading = true;
                                        app.ai_download_progress.store(0, std::sync::atomic::Ordering::Relaxed);
                                        app.ai_error = None;
                                        let progress = app.ai_download_progress.clone();
                                        let tx = ai_tx.clone();
                                        std::thread::spawn(move || {
                                            let res = ai::download_ai_files(progress);
                                            let _ = tx.send(res);
                                        });
                                    }
                                }
                            }
                            't' => {
                                app.current_theme_index = (app.current_theme_index + 1) % THEMES.len();
                            }
                            'r' => {
                                match app.view_mode {
                                    ViewMode::Active => {
                                        app.view_mode = ViewMode::Trash;
                                        app.trash_list_state.select(if app.filtered_trash().is_empty() { None } else { Some(0) });
                                    }
                                    ViewMode::Trash => {
                                        app.view_mode = ViewMode::Active;
                                        app.list_state.select(if app.filtered_tasks().is_empty() { None } else { Some(0) });
                                    }
                                }
                            }
                            'a' => {
                                if app.view_mode == ViewMode::Active {
                                    app.input_mode = InputMode::Adding;
                                    app.input.clear();
                                }
                            }
                            'e' => {
                                if app.view_mode == ViewMode::Active {
                                    let filtered = app.filtered_tasks();
                                    if let Some(v_idx) = app.list_state.selected() {
                                        if v_idx < filtered.len() {
                                            let real_idx = filtered[v_idx];
                                            app.input = app.tasks[real_idx].title.clone();
                                            app.input_mode = InputMode::EditingTitle;
                                        }
                                    }
                                }
                            }
                            '/' => {
                                app.input_mode = InputMode::Searching;
                                app.input = app.search_query.clone();
                            }
                            'c' => {
                                // Быстрая очистка
                                match app.view_mode {
                                    ViewMode::Active => {
                                        // Перемещение ВСЕХ выполненных задач в корзину
                                        let mut active = Vec::new();
                                        let mut completed = Vec::new();
                                        for task in app.tasks.drain(..) {
                                            if task.completed {
                                                let mut t = task;
                                                t.modified_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                                                completed.push(t);
                                            } else {
                                                active.push(task);
                                            }
                                        }
                                        let _moved_count = completed.len();
                                        app.tasks = active;
                                        app.trash.extend(completed);

                                        save_tasks_to_file(&app.tasks, FILE_PATH);
                                        save_tasks_to_file(&app.trash, TRASH_PATH);

                                        app.list_state.select(if app.filtered_tasks().is_empty() { None } else { Some(0) });
                                        
                                        // Показываем мини-уведомление в шапке (опционально) или просто выводим
                                        // Здесь мы просто сохраняем
                                    }
                                    ViewMode::Trash => {
                                        // Очистка ВСЕЙ корзины (запуск подтверждения)
                                        if !app.trash.is_empty() {
                                            app.confirm_action = Some(ConfirmAction::ClearTrash);
                                            app.show_confirm = true;
                                        }
                                    }
                                }
                            }
                            'd' => {
                                match app.view_mode {
                                    ViewMode::Active => {
                                        let filtered = app.filtered_tasks();
                                        if let Some(v_idx) = app.list_state.selected() {
                                            if v_idx < filtered.len() {
                                                let real_idx = filtered[v_idx];
                                                let mut removed = app.tasks.remove(real_idx);
                                                removed.modified_at = chrono::Local::now()
                                                    .format("%Y-%m-%d %H:%M:%S")
                                                    .to_string();
                                                app.trash.push(removed);

                                                save_tasks_to_file(&app.tasks, FILE_PATH);
                                                save_tasks_to_file(&app.trash, TRASH_PATH);

                                                let new_len = app.filtered_tasks().len();
                                                if new_len == 0 {
                                                    app.list_state.select(None);
                                                } else if v_idx >= new_len {
                                                    app.list_state.select(Some(new_len - 1));
                                                } else {
                                                    app.list_state.select(Some(v_idx));
                                                }
                                            }
                                        }
                                    }
                                    ViewMode::Trash => {
                                        // Запуск подтверждения удаления задачи
                                        let filtered = app.filtered_trash();
                                        if let Some(v_idx) = app.trash_list_state.selected() {
                                            if v_idx < filtered.len() {
                                                let real_idx = filtered[v_idx];
                                                app.confirm_action = Some(ConfirmAction::DeletePermanent(real_idx));
                                                app.show_confirm = true;
                                            }
                                        }
                                    }
                                }
                            }
                            'p' => {
                                if app.view_mode == ViewMode::Active {
                                    let filtered = app.filtered_tasks();
                                    if let Some(v_idx) = app.list_state.selected() {
                                        if v_idx < filtered.len() {
                                            let real_idx = filtered[v_idx];
                                            let current = app.tasks[real_idx].priority;
                                            app.tasks[real_idx].priority = match current {
                                                Priority::Low => Priority::Medium,
                                                Priority::Medium => Priority::High,
                                                Priority::High => Priority::Critical,
                                                Priority::Critical => Priority::Low,
                                            };
                                            app.tasks[real_idx].modified_at = chrono::Local::now()
                                                .format("%Y-%m-%d %H:%M:%S")
                                                .to_string();
                                            save_tasks_to_file(&app.tasks, FILE_PATH);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    } else {
                        // Стрелки и Enter
                        match key.code {
                            KeyCode::Down => app.next(),
                            KeyCode::Up => app.previous(),
                            KeyCode::Enter => {
                                app.toggle_or_restore();
                            }
                            _ => {}
                        }
                    }
                }
                InputMode::Adding => match key.code {
                    KeyCode::Enter => {
                        let title = app.input.trim().to_string();
                        if !title.is_empty() {
                            let now_str = chrono::Local::now()
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string();
                            app.tasks.push(Task {
                                title,
                                completed: false,
                                priority: Priority::Low,
                                created_at: now_str.clone(),
                                modified_at: now_str,
                            });
                            save_tasks_to_file(&app.tasks, FILE_PATH);
                            app.list_state.select(Some(app.tasks.len() - 1));
                        }
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    _ => {}
                },
                InputMode::EditingTitle => match key.code {
                    KeyCode::Enter => {
                        let title = app.input.trim().to_string();
                        let filtered = app.filtered_tasks();
                        if let Some(v_idx) = app.list_state.selected() {
                            if !title.is_empty() && v_idx < filtered.len() {
                                let real_idx = filtered[v_idx];
                                app.tasks[real_idx].title = title;
                                app.tasks[real_idx].modified_at = chrono::Local::now()
                                    .format("%Y-%m-%d %H:%M:%S")
                                    .to_string();
                                save_tasks_to_file(&app.tasks, FILE_PATH);
                            }
                        }
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    _ => {}
                },
                InputMode::Searching => match key.code {
                    KeyCode::Enter => {
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.input.clear();
                        app.search_query.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                        app.search_query = app.input.clone();
                        app.list_state.select(Some(0));
                        app.trash_list_state.select(Some(0));
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                        app.search_query = app.input.clone();
                        app.list_state.select(Some(0));
                        app.trash_list_state.select(Some(0));
                    }
                    _ => {}
                },
                InputMode::AiPrompt => match key.code {
                    KeyCode::Enter => {
                        let prompt = app.input.trim().to_string();
                        if !prompt.is_empty() {
                            app.ai_generating = true;
                            let tx = ai_tx.clone();
                            std::thread::spawn(move || {
                                let result = ai::generate_subtasks(&prompt);
                                let _ = tx.send(result);
                            });
                        }
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        app.input.clear();
                        app.input_mode = InputMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    _ => {}
                }
            }
        }
    }
}

// Вспомогательная функция для создания центрированного прямоугольника (для модальных окон)
fn centered_rect(percent_x: u16, percent_y: u16, r: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// Функция для проверки обновлений с GitHub API
fn check_github_update() -> Result<String, String> {
    let response = ureq::get("https://api.github.com/repos/ZAREZOV2/todoly-tui/releases/latest")
        .set("User-Agent", "ToDoLy-TUI")
        .call()
        .map_err(|e| e.to_string())?;

    let response_str = response.into_string().map_err(|e| e.to_string())?;
    
    // Простой ручной поиск поля "tag_name"
    if let Some(tag_idx) = response_str.find("\"tag_name\"") {
        let after_tag = &response_str[tag_idx + 10..];
        let mut chars = after_tag.chars();
        let mut start_idx = None;
        let mut end_idx = None;
        let mut current_pos = 0;
        
        while let Some(c) = chars.next() {
            if c == '"' {
                if start_idx.is_none() {
                    start_idx = Some(current_pos + 1);
                } else {
                    end_idx = Some(current_pos);
                    break;
                }
            }
            current_pos += c.len_utf8();
        }
        
        if let (Some(s), Some(e)) = (start_idx, end_idx) {
            let tag = after_tag[s..e].to_string();
            return Ok(tag);
        }
    }
    
    Err("Could not parse tag_name from GitHub API response".to_string())
}
