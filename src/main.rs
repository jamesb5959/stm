use std::error::Error;
use std::fs;
use std::io;
use std::process::Command;
use std::time::Duration;

use csv::ReaderBuilder;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};
use tui::widgets::canvas::{Canvas, Line};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use serde::Deserialize;

// ============================
// CSV Structures and Functions
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
// Stock Data for ML List
// ============================
#[derive(Debug)]
enum MLMode {
    List,
    Search,
}

#[derive(Debug)]
struct StockInfo {
    ticker: String,
    price: f64,
    change: f64,
    pct_change: f64,
}

fn get_stock_info(file_path: &str, ticker: &str) -> Option<StockInfo> {
    // Expects a Yahoo Finance CSV with header; "Close" is at index 4.
    let mut rdr = ReaderBuilder::new().from_path(file_path).ok()?;
    let mut close_prices = Vec::new();
    for result in rdr.records() {
        if let Ok(record) = result {
            if let Some(close_str) = record.get(4) {
                if let Ok(close) = close_str.parse::<f64>() {
                    close_prices.push(close);
                }
            }
        }
    }
    if close_prices.len() >= 2 {
        let last = *close_prices.last()?;
        let prev = close_prices[close_prices.len()-2];
        let change = last - prev;
        let pct_change = if prev != 0.0 { change / prev * 100.0 } else { 0.0 };
        Some(StockInfo {
            ticker: ticker.to_string(),
            price: last,
            change,
            pct_change,
        })
    } else {
        None
    }
}

fn load_stocks() -> Vec<StockInfo> {
    let mut stocks = Vec::new();
    let dir = "pre_stock";
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "csv" {
                        if let Some(ticker_os) = path.file_stem() {
                            if let Some(ticker) = ticker_os.to_str() {
                                if let Some(info) = get_stock_info(path.to_str().unwrap(), ticker) {
                                    stocks.push(info);
                                } else {
                                    stocks.push(StockInfo {
                                        ticker: ticker.to_string(),
                                        price: 0.0,
                                        change: 0.0,
                                        pct_change: 0.0,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    stocks
}

// ============================
// App State
// ============================
struct App {
    stocks: Vec<StockInfo>,
    selected: usize,
    ml_mode: MLMode,
    search_input: String,
    show_instructions: bool,
    ml_output: String,
    accounts: Vec<AccountSummary>,
}

impl App {
    fn new() -> Self {
        Self {
            stocks: Vec::new(),
            selected: 0,
            ml_mode: MLMode::List,
            search_input: String::new(),
            show_instructions: false,
            ml_output: String::new(),
            accounts: Vec::new(),
        }
    }
}

// ============================
// Main TUI Application
// ============================
fn main() -> Result<(), Box<dyn Error>> {
    // Load account summary data from CSV
    let accounts = read_accounts_from_csv("account_summary.csv").unwrap_or_else(|err| {
        eprintln!("Warning: could not read account_summary.csv: {}", err);
        Vec::new()
    });

    let mut app = App::new();
    app.stocks = load_stocks();
    app.accounts = accounts;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, &mut app);

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

fn run_app<B: tui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        // Refresh stocks list each loop
        app.stocks = load_stocks();

        terminal.draw(|f| {
            let size = f.size();

            if app.show_instructions {
                let instructions = "\
Instructions:
 - Up/Down: Navigate ML stock list
 - Enter (List mode): Preprocess & train on selected stock
 - s: Activate search box
 - In Search mode: Type ticker and press Enter to download data
 - Esc (in Search mode): Cancel search
 - h: Toggle instructions overlay
 - q: Quit";
                let block = Block::default().title("Instructions").borders(Borders::ALL);
                let paragraph = Paragraph::new(instructions).block(block);
                f.render_widget(paragraph, size);
                return;
            }

            // Main vertical layout: Top (50%), Middle (30%), Bottom (20%)
            let vertical_chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(30),
                    Constraint::Percentage(20),
                ].as_ref())
                .split(size);

            // Top panel: split horizontally into Left (Stock Chart) and Right (Live Trades)
            let top_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(vertical_chunks[0]);

            // Top Left: Stock Chart (dummy line chart)
            let data = vec![
                (0.0, 100.0),
                (1.0, 102.5),
                (2.0, 105.0),
                (3.0, 103.0),
                (4.0, 107.0),
                (5.0, 106.0),
                (6.0, 110.0),
            ];
            let (x_min, x_max) = data.iter().fold((f64::MAX, f64::MIN), |(mn, mx), &(x,_)| (mn.min(x), mx.max(x)));
            let (y_min, y_max) = data.iter().fold((f64::MAX, f64::MIN), |(mn, mx), &(_, y)| (mn.min(y), mx.max(y)));
            let line_segments = data.windows(2).map(|pair| {
                let (x1, y1) = pair[0];
                let (x2, y2) = pair[1];
                Line { x1, y1, x2, y2, color: Color::Green }
            });
            let chart = Canvas::default()
                .block(Block::default().title("Stock Chart").borders(Borders::ALL))
                .x_bounds([x_min - 0.5, x_max + 0.5])
                .y_bounds([y_min - 2.0, y_max + 2.0])
                .paint(move |ctx| {
                    for seg in line_segments.clone() {
                        ctx.draw(&seg);
                    }
                });
            f.render_widget(chart, top_chunks[0]);

            // Top Right: Live Trades from trading_history.csv
            let trades = read_trades_from_csv("trading_history.csv").unwrap_or_else(|_| Vec::new());
            let live_trades_text = trades.iter().map(|t| {
                format!("{}  {:.2}  {:.2}", t.name, t.transaction, t.new_balance)
            }).collect::<Vec<_>>().join("\n");
            let live_trades = Paragraph::new(live_trades_text)
                .block(Block::default().title("Live Trades").borders(Borders::ALL));
            f.render_widget(live_trades, top_chunks[1]);

            // Middle: Account Summary Table
            let rows: Vec<Row> = app.accounts.iter().map(|acc| {
                Row::new(vec![
                    acc.name.clone(),
                    format!("{:.2}", acc.initial_amount),
                    format!("{:.2}", acc.current_amount),
                    format!("{:.2}", acc.change),
                    format!("{:.2}%", acc.percentage_change),
                ])
            }).collect();
            let table = Table::new(rows)
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
            f.render_widget(table, vertical_chunks[1]);

            // Bottom: Split horizontally into ML List and Search Box
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ].as_ref())
                .split(vertical_chunks[2]);

            // Bottom Left: ML List of available stocks from pre_stock/
            let ml_list_text = app.stocks.iter().enumerate().map(|(i, s)| {
                let marker = if i == app.selected { ">" } else { " " };
                format!("{} {}  {:.2}  {:.2} ({:.2}%)", marker, s.ticker, s.price, s.change, s.pct_change)
            }).collect::<Vec<String>>().join("\n");
            let ml_list = Paragraph::new(ml_list_text)
                .block(Block::default().title("ML List").borders(Borders::ALL));
            f.render_widget(ml_list, bottom_chunks[0]);

            // Bottom Right: Search Box (always visible)
            let search_text = format!("Search Ticker: {}\n\n{}", app.search_input, app.ml_output);
            let search_box = Paragraph::new(search_text)
                .block(Block::default().title("Search").borders(Borders::ALL));
            f.render_widget(search_box, bottom_chunks[1]);
        })?;

        // Event handling
        if event::poll(Duration::from_millis(300))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('h') => {
                        app.show_instructions = !app.show_instructions;
                    }
                    KeyCode::Char('s') => {
                        app.ml_mode = MLMode::Search;
                        app.search_input.clear();
                    }
                    KeyCode::Esc => {
                        app.ml_mode = MLMode::List;
                        app.search_input.clear();
                    }
                    KeyCode::Enter => {
                        if let MLMode::Search = app.ml_mode {
                            // In search mode, download stock data.
                            let ticker = app.search_input.trim().to_uppercase();
                            if !ticker.is_empty() {
                                let output_dl = Command::new("python3")
                                    .arg("download_stock.py")
                                    .arg(&ticker)
                                    .output();
                                match output_dl {
                                    Ok(o) if o.status.success() => {
                                        app.ml_output = format!("Downloaded data for {}", ticker);
                                    }
                                    Ok(o) => {
                                        let err = String::from_utf8_lossy(&o.stderr);
                                        app.ml_output = format!("Download error: {}", err.trim());
                                    }
                                    Err(e) => {
                                        app.ml_output = format!("Failed to run download_stock.py: {}", e);
                                    }
                                }
                                app.ml_mode = MLMode::List;
                                app.search_input.clear();
                                app.stocks = load_stocks();
                            }
                        } else {
                            // In list mode, run preprocess & model on selected stock.
                            if let Some(stock) = app.stocks.get(app.selected) {
                                let csv_file = format!("pre_stock/{}.csv", stock.ticker);
                                let output_pre = Command::new("python3")
                                    .arg("ml/preprocess.py")
                                    .arg(&csv_file)
                                    .output();
                                match output_pre {
                                    Ok(o) if o.status.success() => {
                                        app.ml_output = format!("Preprocess OK for {}", stock.ticker);
                                    }
                                    Ok(o) => {
                                        let err = String::from_utf8_lossy(&o.stderr);
                                        app.ml_output = format!("Preprocess error: {}", err.trim());
                                    }
                                    Err(e) => {
                                        app.ml_output = format!("Failed to run preprocess.py: {}", e);
                                    }
                                }
                                let output_model = Command::new("python3")
                                    .arg("ml/model.py")
                                    .output();
                                match output_model {
                                    Ok(o) if o.status.success() => {
                                        let pred = String::from_utf8_lossy(&o.stdout);
                                        app.ml_output = format!("ML Prediction for {}: {}", stock.ticker, pred.trim());
                                    }
                                    Ok(o) => {
                                        let err = String::from_utf8_lossy(&o.stderr);
                                        app.ml_output = format!("Model error: {}", err.trim());
                                    }
                                    Err(e) => {
                                        app.ml_output = format!("Failed to run model.py: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let MLMode::List = app.ml_mode {
                            if !app.stocks.is_empty() {
                                app.selected = (app.selected + 1) % app.stocks.len();
                            }
                        }
                    }
                    KeyCode::Up => {
                        if let MLMode::List = app.ml_mode {
                            if !app.stocks.is_empty() {
                                if app.selected == 0 {
                                    app.selected = app.stocks.len() - 1;
                                } else {
                                    app.selected -= 1;
                                }
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        if let MLMode::Search = app.ml_mode {
                            app.search_input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if let MLMode::Search = app.ml_mode {
                            app.search_input.pop();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

