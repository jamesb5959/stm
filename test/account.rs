use std::error::Error;
use csv::{ReaderBuilder, WriterBuilder};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct AccountSummary {
    name: String,
    initial_amount: f64,
    current_amount: f64,
    change: f64,
    percentage_change: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TradingRecord {
    name: String,
    transaction: f64,     // + for gain, - for loss
    new_balance: f64,
    percentage_change: f64, // change for this transaction relative to initial amount
}

fn main() -> Result<(), Box<dyn Error>> {
    // Create some initial accounts.
    let mut accounts = vec![
        AccountSummary {
            name: "Alice".to_string(),
            initial_amount: 10.0,
            current_amount: 10.0,
            change: 0.0,
            percentage_change: 0.0,
        },
        AccountSummary {
            name: "Bob".to_string(),
            initial_amount: 20.0,
            current_amount: 20.0,
            change: 0.0,
            percentage_change: 0.0,
        },
    ];

    // Vector to hold the trading history.
    let mut history: Vec<TradingRecord> = Vec::new();

    // Simulate some trades:
    // Alice gains $5 (balance goes from 10 to 15).
    process_trade(&mut accounts, &mut history, "Alice", 5.0)?;
    // Bob loses $3 (balance goes from 20 to 17).
    process_trade(&mut accounts, &mut history, "Bob", -3.0)?;
    // Alice gains another $2 (balance goes from 15 to 17).
    process_trade(&mut accounts, &mut history, "Alice", 2.0)?;

    // Write the account summary to "account_summary.csv".
    let mut account_writer = WriterBuilder::new().from_path("account_summary.csv")?;
    for account in &accounts {
        account_writer.serialize(account)?;
    }
    account_writer.flush()?;

    // Write the trading history to "trading_history.csv".
    let mut history_writer = WriterBuilder::new().from_path("trading_history.csv")?;
    for record in &history {
        history_writer.serialize(record)?;
    }
    history_writer.flush()?;

    println!("CSV files written successfully.");
    Ok(())
}

/// Processes a trade for a given account:
/// - Finds the account by name.
/// - Updates the current amount, total change, and percentage change.
/// - Logs the trade in the trading history.
fn process_trade(
    accounts: &mut Vec<AccountSummary>, 
    history: &mut Vec<TradingRecord>, 
    name: &str, 
    trade_amount: f64
) -> Result<(), Box<dyn Error>> {
    if let Some(account) = accounts.iter_mut().find(|a| a.name == name) {
        account.current_amount += trade_amount;
        account.change = account.current_amount - account.initial_amount;
        account.percentage_change = (account.change / account.initial_amount) * 100.0;

        // Create a record for this trade.
        let record = TradingRecord {
            name: name.to_string(),
            transaction: trade_amount,
            new_balance: account.current_amount,
            percentage_change: (trade_amount / account.initial_amount) * 100.0,
        };
        history.push(record);
    } else {
        println!("Account {} not found.", name);
    }
    Ok(())
}

