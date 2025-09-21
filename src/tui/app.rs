use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::io;

/// The main TUI application struct
pub struct App {
    /// Whether the application should quit
    should_quit: bool,
    /// The current view mode
    view_mode: ViewMode,
}

/// Different view modes for the TUI
#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    /// Main splash screen with ASCII art
    Splash,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            should_quit: false,
            view_mode: ViewMode::Splash,
        }
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the main loop
        let result = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main application loop
    async fn run_app(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.should_quit = true;
                        }
                        _ => {}
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    /// Render the UI
    fn ui(&mut self, f: &mut Frame) {
        match self.view_mode {
            ViewMode::Splash => self.render_splash(f),
        }
    }

    /// Render the splash screen with ASCII art
    fn render_splash(&self, f: &mut Frame) {
        let size = f.area();

        // Create main layout - split horizontally
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(size);

        // Left side - VEGA ASCII art
        let vega_art = self.create_vega_ascii_art();
        let vega_paragraph = Paragraph::new(vega_art)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(vega_paragraph, chunks[0]);

        // Right side - Star field background
        let star_field = self.create_star_field(chunks[1]);
        let star_paragraph = Paragraph::new(star_field)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::NONE));

        f.render_widget(star_paragraph, chunks[1]);

        // Bottom instruction
        let instruction_area = Rect {
            x: 0,
            y: size.height.saturating_sub(3),
            width: size.width,
            height: 3,
        };

        let instruction = Paragraph::new("Press 'q' or ESC to exit")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(Clear, instruction_area);
        f.render_widget(instruction, instruction_area);
    }

    /// Create ASCII art for "vega" in lowercase
    fn create_vega_ascii_art(&self) -> Text<'static> {
        let art = vec![
            "                                    ",
            "                                    ",
            "                                    ",
            "██╗   ██╗███████╗ ██████╗  █████╗  ",
            "██║   ██║██╔════╝██╔════╝ ██╔══██╗ ",
            "██║   ██║█████╗  ██║  ███╗███████║ ",
            "╚██╗ ██╔╝██╔══╝  ██║   ██║██╔══██║ ",
            " ╚████╔╝ ███████╗╚██████╔╝██║  ██║ ",
            "  ╚═══╝  ╚══════╝ ╚═════╝ ╚═╝  ╚═╝ ",
            "                                    ",
            "                                    ",
            "                                    ",
        ];

        let lines: Vec<Line> = art
            .into_iter()
            .map(|line| Line::from(Span::styled(line, Style::default().fg(Color::Cyan))))
            .collect();

        Text::from(lines)
    }

    /// Create a star field pattern
    fn create_star_field(&self, area: Rect) -> Text<'static> {
        let height = area.height as usize;
        let width = area.width as usize;

        let mut lines = Vec::new();

        // Create a pattern of stars with different brightness
        for row in 0..height {
            let mut line_content = String::new();
            for col in 0..width {
                // Use a simple pattern to determine star placement
                let star_chance = (row * 7 + col * 11) % 100;
                if star_chance < 3 {
                    // Bright star
                    line_content.push('★');
                } else if star_chance < 8 {
                    // Medium star
                    line_content.push('✦');
                } else if star_chance < 15 {
                    // Dim star
                    line_content.push('·');
                } else {
                    line_content.push(' ');
                }
            }

            // Style the line with different colors for different star types
            let spans: Vec<Span> = line_content
                .chars()
                .map(|c| match c {
                    '★' => Span::styled(
                        c.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    '✦' => Span::styled(c.to_string(), Style::default().fg(Color::White)),
                    '·' => Span::styled(c.to_string(), Style::default().fg(Color::Gray)),
                    _ => Span::styled(c.to_string(), Style::default()),
                })
                .collect();

            lines.push(Line::from(spans));
        }

        Text::from(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_creation() {
        let app = App::new();
        assert_eq!(app.view_mode, ViewMode::Splash);
        assert!(!app.should_quit);
    }

    #[test]
    fn test_default_app() {
        let app = App::default();
        assert_eq!(app.view_mode, ViewMode::Splash);
        assert!(!app.should_quit);
    }
}
