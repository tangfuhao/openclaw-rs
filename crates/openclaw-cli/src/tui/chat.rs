use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::io;

struct ChatState {
    messages: Vec<(String, String)>, // (role, content)
    input: String,
    scroll: usize,
}

pub async fn run_chat_tui(gateway_url: &str, _session: Option<&str>) -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = ChatState {
        messages: vec![
            ("system".into(), format!("Connected to OpenClaw gateway at {gateway_url}")),
            ("system".into(), "Type a message and press Enter to send. Ctrl+C to quit.".into()),
        ],
        input: String::new(),
        scroll: 0,
    };

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Messages area
            let items: Vec<ListItem> = state
                .messages
                .iter()
                .map(|(role, content)| {
                    let style = match role.as_str() {
                        "user" => Style::default().fg(Color::Cyan),
                        "assistant" => Style::default().fg(Color::Green),
                        "system" => Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
                        _ => Style::default(),
                    };
                    let prefix = match role.as_str() {
                        "user" => "You",
                        "assistant" => "AI",
                        "system" => "SYS",
                        _ => role.as_str(),
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("[{prefix}] "), style.add_modifier(Modifier::BOLD)),
                        Span::styled(content.clone(), style),
                    ]))
                })
                .collect();

            let messages_widget = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(" OpenClaw Chat "));
            f.render_widget(messages_widget, chunks[0]);

            // Input area
            let input_widget = Paragraph::new(state.input.as_str())
                .block(Block::default().borders(Borders::ALL).title(" Message "));
            f.render_widget(input_widget, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                    (KeyCode::Enter, _) => {
                        if !state.input.is_empty() {
                            let msg = state.input.clone();
                            state.messages.push(("user".into(), msg.clone()));
                            state.input.clear();
                            // TODO: Send to gateway via WebSocket
                            state.messages.push((
                                "assistant".into(),
                                "Agent integration pending - gateway connection required.".into(),
                            ));
                        }
                    }
                    (KeyCode::Char(c), _) => state.input.push(c),
                    (KeyCode::Backspace, _) => { state.input.pop(); }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
