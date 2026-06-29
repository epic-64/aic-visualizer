mod app;
mod model;
mod parser;
mod storage;
mod ui;

use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Screen};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    res
}

fn run<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        // Global quit.
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }

        match app.screen {
            Screen::ModelSelect => handle_model_select(app, key.code),
            Screen::CustomModel => handle_custom_model(app, key.code),
            Screen::Start => handle_start(app, key.code),
            Screen::Chat => handle_chat(app, key),
            Screen::SaveName => handle_save_name(app, key.code),
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_model_select(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Up | KeyCode::Char('k') => app.select_up(),
        KeyCode::Down | KeyCode::Char('j') => app.select_down(),
        KeyCode::Char('c') => app.open_form(),
        KeyCode::Enter => app.choose_model(),
        _ => {}
    }
}

fn handle_start(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.screen = Screen::ModelSelect,
        KeyCode::Up | KeyCode::Char('k') => app.start_up(),
        KeyCode::Down | KeyCode::Char('j') => app.start_down(),
        KeyCode::Enter => app.start_choose(),
        _ => {}
    }
}

fn handle_save_name(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.screen = Screen::Chat,
        KeyCode::Enter => app.submit_save(),
        KeyCode::Backspace => {
            app.save_name.pop();
        }
        KeyCode::Char(c) => app.save_name.push(c),
        _ => {}
    }
}

fn handle_custom_model(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.status.clear();
            app.screen = Screen::ModelSelect;
        }
        KeyCode::Tab | KeyCode::Down => app.form_next(),
        KeyCode::BackTab | KeyCode::Up => app.form_prev(),
        KeyCode::Enter => match app.submit_form() {
            Ok(()) => app.status.clear(),
            Err(e) => app.status = e,
        },
        KeyCode::Backspace => {
            app.form[app.form_field].pop();
        }
        KeyCode::Char(c) => app.form[app.form_field].push(c),
        _ => {}
    }
}

fn handle_chat(app: &mut App, key: KeyEvent) {
    // Ctrl+S saves the conversation.
    if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.open_save();
        return;
    }
    match key.code {
        KeyCode::Esc => {
            app.status.clear();
            app.screen = Screen::ModelSelect;
        }
        KeyCode::Enter => app.submit_turn(),
        KeyCode::Up => app.history_up(),
        KeyCode::Down => app.history_down(),
        KeyCode::Backspace => {
            app.input.pop();
            app.on_input_edit();
        }
        KeyCode::Char(c) => {
            app.input.push(c);
            app.on_input_edit();
        }
        _ => {}
    }
}
