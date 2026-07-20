use anyhow::{Result, anyhow};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Flex, Layout, Rect},
    style::Stylize as _,
    text::Line,
    widgets::{Block, Paragraph},
};

use crate::{
    cli::{
        Commands, format_balance, format_breaked_duration, format_focused_duration, send_command,
    },
    stopwatch::{Phase, StopwatchStatus},
};

/// Run the TUI, restoring the terminal on exit even if the event loop errors.
///
/// # Errors
///
/// Returns an error if terminal event handling or daemon communication fails.
pub async fn run_tui() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal).await;
    ratatui::restore();
    result
}

async fn event_loop(terminal: &mut DefaultTerminal) -> Result<()> {
    loop {
        let status_minutes = fetch_status().await?;

        terminal.draw(|frame| {
            let [state_area, durations_area, hints_area] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            let [pause_area, phase_area] =
                Layout::horizontal([Constraint::Ratio(1, 2); 2]).areas(state_area);
            let [focus_area, break_area, balance_area] =
                Layout::horizontal([Constraint::Ratio(1, 3); 3]).areas(durations_area);

            draw_box(
                frame,
                pause_area,
                " pause ",
                if status_minutes.is_paused {
                    "⏸ paused".to_owned()
                } else {
                    "▶".to_owned()
                },
            );
            draw_box(
                frame,
                phase_area,
                " phase ",
                match status_minutes.phase {
                    Phase::Idle => "✋ idle".to_owned(),
                    Phase::Focusing => "⏰ focusing".to_owned(),
                    Phase::Breaking => "🏖️ breaking".to_owned(),
                },
            );
            draw_box(
                frame,
                focus_area,
                " focus ",
                format_focused_duration(status_minutes.focused_duration),
            );
            draw_box(
                frame,
                break_area,
                " break ",
                format_breaked_duration(status_minutes.breaked_duration),
            );
            draw_box(
                frame,
                balance_area,
                " balance ",
                format_balance(status_minutes.balance),
            );

            let hints = Line::from("space ↕ pause · ←→ phase · q quit").dark_gray();
            frame.render_widget(hints, hints_area);
        })?;

        if event::poll(std::time::Duration::from_millis(500))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char(' ') | KeyCode::Up | KeyCode::Down => {
                    send_command(Commands::TogglePause).await?;
                }
                KeyCode::Right | KeyCode::Left => {
                    send_command(Commands::TogglePhase).await?;
                }
                _ => {}
            }
        }
    }
}

fn draw_box(frame: &mut Frame<'_>, area: Rect, title: &str, text: String) {
    let block = Block::bordered().title(Line::from(title).centered());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [middle] = Layout::vertical([Constraint::Length(1)])
        .flex(Flex::Center)
        .areas(inner);
    frame.render_widget(Paragraph::new(text).centered(), middle);
}

async fn fetch_status() -> Result<StopwatchStatus> {
    let raw = send_command(Commands::Status {
        focus: false,
        breaks: false,
        balance: false,
    })
    .await?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let stopwatch_status: StopwatchStatus =
        serde_json::from_value(value.get("data").ok_or_else(|| anyhow!("no data"))?.clone())?;
    let stopwatch_status = stopwatch_status.to_minutes();
    Ok(stopwatch_status)
}
