use std::error::Error;
use std::io;
use std::process::Command;
use std::time::Duration;

use csv::ReaderBuilder;
use serde::Deserialize;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::Color,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};
use tui::widgets::canvas::{Canvas, Line};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// ============================
// Structs for CSV data
// ============================
#[derive(Debug, Deserialize)]
struct AccountSummary {
    name: String,
    initial_amount: f64,
    current_amount: f64,
    change: f64,
    percentage_change: f64,
}

#[derive(Debug, Deserialize)]
struct TradeRecord {
    name: String,
    transaction: f64,
    new_balance: f64,
}

// ============================
// CSV reading functions
// ============================
fn read_accounts_from_csv(path: &str) -> Result<Vec<AccountSummary>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().from_path(path)?;
    let mut records = Vec::new();
    for result in rdr.deserialize() {
        let rec: AccountSummary = result?;
        records.push(rec);
    }
    Ok(records)
}

fn read_trades_from_csv(path: &str) -> Result<Vec<TradeRecord>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().from_path(path)?;
    let mut trades = Vec::new();
    for result in rdr.deserialize() {
        let rec: TradeRecord = result?;
        trades.push(rec);
    }
    Ok(trades)
}

// ============================
// Example line chart data
// ============================
fn get_stock_timeseries() -> Vec<(f64, f64)> {
    vec![
        (0.0, 100.0),
        (1.0, 102.5),
        (2.0, 105.0),
        (3.0, 103.0),
        (4.0, 107.0),
        (5.0, 106.0),
        (6.0, 110.0),
    ]
}

fn main() -> Result<(), Box<dyn Error>> {
    // Try reading account summaries. If missing, use an empty vector.
    let accounts = read_accounts_from_csv("account_summary.csv").unwrap_or_else(|err| {
        eprintln!("Warning: could not read account_summary.csv: {}", err);
        Vec::new()
    });

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, accounts);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }
    Ok(())
}

fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    accounts: Vec<AccountSummary>,
) -> io::Result<()> {
    let mut bottom_text = String::from("Press 'm' to run ML, 'q' to quit.");

    loop {
        terminal.draw(|f| {
            // Layout:
            //  1) top (50%): horizontally split => left=chart, right=live trades
            //  2) middle (30%): account summary
            //  3) bottom (20%): instructions or ML output
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                ].as_ref())
                .split(f.size());

            // top is horizontally split into two
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(main_chunks[0]);

            // =========== Left (top) panel: line chart
            let data = get_stock_timeseries();
            let (x_min, x_max) = data.iter().fold((f64::MAX, f64::MIN), |(mn, mx), &(x,_)| {
                (mn.min(x), mx.max(x))
            });
            let (y_min, y_max) = data.iter().fold((f64::MAX, f64::MIN), |(mn, mx), &(_,y)| {
                (mn.min(y), mx.max(y))
            });

            let line_segments = data.windows(2).map(|pair| {
                let (x1, y1) = pair[0];
                let (x2, y2) = pair[1];
                Line {
                    x1, y1,
                    x2, y2,
                    color: Color::Green,
                }
            });

            let chart_canvas = Canvas::default()
                .block(Block::default().title("Stock Chart").borders(Borders::ALL))
                .x_bounds([x_min - 0.5, x_max + 0.5])
                .y_bounds([y_min - 2.0, y_max + 2.0])
                .paint(move |ctx| {
                    for seg in line_segments.clone() {
                        ctx.draw(&seg);
                    }
                });
            f.render_widget(chart_canvas, top_chunks[0]);

            // =========== Right (top) panel: live trades
            // read the CSV each loop so we see updates
            let trades = read_trades_from_csv("trading_history.csv").unwrap_or_else(|_| Vec::new());
            let rows_trades: Vec<Row> = trades.iter().map(|t| {
                Row::new(vec![
                    t.name.clone(),
                    format!("{:.2}", t.transaction),
                    format!("{:.2}", t.new_balance),
                ])
            }).collect();
            let table_trades = Table::new(rows_trades)
                .header(
                    Row::new(vec!["Name", "Trans", "New Bal"])
                        .bottom_margin(1),
                )
                .block(Block::default().title("Live Trades").borders(Borders::ALL))
                .widths(&[
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ]);
            f.render_widget(table_trades, top_chunks[1]);

            // =========== Middle panel (30%): account summary
            let rows_accounts: Vec<Row> = accounts.iter().map(|acc| {
                Row::new(vec![
                    acc.name.clone(),
                    format!("{:.2}", acc.initial_amount),
                    format!("{:.2}", acc.current_amount),
                    format!("{:.2}", acc.change),
                    format!("{:.2}%", acc.percentage_change),
                ])
            }).collect();
            let table_accounts = Table::new(rows_accounts)
                .header(
                    Row::new(vec!["Name", "Initial", "Current", "Change", "% Change"])
                        .bottom_margin(1),
                )
                .block(Block::default().title("Account Summary").borders(Borders::ALL))
                .widths(&[
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                    Constraint::Length(10),
                ]);
            f.render_widget(table_accounts, main_chunks[1]);

            // =========== Bottom panel (20%): instructions / ML
            let paragraph = Paragraph::new(Spans::from(vec![Span::raw(&bottom_text)]))
                .block(Block::default().title("ML / Instructions").borders(Borders::ALL));
            f.render_widget(paragraph, main_chunks[2]);
        })?;

        // Key events
        if event::poll(Duration::from_millis(300))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('m') => {
                        let output = Command::new("python3")
                            .arg("ml/model.py")
                            .arg("3.14")
                            .output();
                        match output {
                            Ok(o) if o.status.success() => {
                                let pred = String::from_utf8_lossy(&o.stdout);
                                bottom_text = format!("ML Prediction: {}", pred.trim());
                            }
                            Ok(o) => {
                                let err = String::from_utf8_lossy(&o.stderr);
                                bottom_text = format!("Error: {}", err.trim());
                            }
                            Err(e) => {
                                bottom_text = format!("Failed to run model.py: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

